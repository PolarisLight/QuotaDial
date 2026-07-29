# Session Accuracy and Table Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Correct guardian session ownership and GPT-5.5 equivalent pricing, preserve partial known costs, and make the session table vertically scrollable without horizontal overflow.

**Architecture:** Normalize guardian parent IDs while parsing JSONL, then aggregate all child usage at repository read time while excluding internal review models from primary-model selection. Extend the serialized session summary with priced and unpriced token counts, and update the existing table component with fixed column sizing and an internal vertical scroll boundary.

**Tech Stack:** Rust, serde_json, rusqlite, Tauri, React, TypeScript, Vitest, Testing Library, CSS

---

## File Structure

- Create `app/src-tauri/tests/fixtures/sessions/guardian.jsonl`: representative new-format guardian metadata and usage.
- Modify `app/src-tauri/src/sessions/parser.rs`: resolve old subagent and new guardian parent formats.
- Modify `app/src-tauri/src/sessions/pricing.rs`: add official GPT-5.5 standard pricing.
- Modify `app/src-tauri/src/domain/session.rs`: serialize priced and unpriced token counts.
- Modify `app/src-tauri/src/storage/repository.rs`: exclude internal review models from primary-model selection, hide orphan review roots, and aggregate partial costs.
- Modify `app/src/types/dashboard.ts`: mirror new session summary fields.
- Modify `app/src/lib/backend.ts`: update preview data.
- Modify `app/src/components/SessionDetails.tsx`: show all sessions, partial cost state, titles, and accessible complete labels.
- Modify `app/src/components/Dashboard.test.tsx`: cover scrolling, rows, model display, and cost states.
- Modify `app/src/styles/app.css`: add fixed table layout, sticky header, vertical scrolling, and explicit column widths.

### Task 1: Parse Guardian Ownership

**Files:**
- Create: `app/src-tauri/tests/fixtures/sessions/guardian.jsonl`
- Modify: `app/src-tauri/src/sessions/parser.rs`

- [ ] **Step 1: Add a guardian fixture**

Create a complete-line JSONL fixture whose metadata uses the current guardian shape and whose token event uses `codex-auto-review`:

```json
{"timestamp":"2026-07-29T10:00:00Z","type":"session_meta","payload":{"id":"guardian-1","session_id":"root-1","cwd":"/tmp/example","source":{"subagent":{"other":"guardian"}}}}
{"timestamp":"2026-07-29T10:01:00Z","type":"turn_context","payload":{"model":"codex-auto-review"}}
{"timestamp":"2026-07-29T10:02:00Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":300,"cached_input_tokens":100,"output_tokens":50,"reasoning_output_tokens":10,"total_tokens":350},"total_token_usage":{"total_tokens":350}}}}
```

- [ ] **Step 2: Write the failing parser test**

Add:

```rust
#[test]
fn maps_guardian_session_to_the_user_session_id() {
    let parsed = parse_reader(
        include_bytes!("../../tests/fixtures/sessions/guardian.jsonl").as_slice(),
        "guardian.jsonl",
        0,
    )
    .unwrap();

    let metadata = parsed.metadata.unwrap();
    assert_eq!(metadata.session_id, "guardian-1");
    assert_eq!(metadata.parent_session_id.as_deref(), Some("root-1"));
    assert_eq!(parsed.events[0].model.as_deref(), Some("codex-auto-review"));
}
```

- [ ] **Step 3: Run the focused test and verify failure**

Run:

```bash
cd app/src-tauri
cargo test sessions::parser::tests::maps_guardian_session_to_the_user_session_id
```

Expected: FAIL because `parent_session_id` is `None`.

- [ ] **Step 4: Implement normalized parent extraction**

Add a helper and use it when constructing `ParsedSessionMetadata`:

```rust
fn parent_session_id(payload: &Value, session_id: &str) -> Option<String> {
    payload
        .pointer("/source/subagent/thread_spawn/parent_thread_id")
        .and_then(Value::as_str)
        .or_else(|| {
            let is_guardian =
                payload.pointer("/source/subagent/other").and_then(Value::as_str)
                    == Some("guardian");
            let owner = payload.get("session_id").and_then(Value::as_str);
            (is_guardian && owner != Some(session_id)).then_some(owner).flatten()
        })
        .map(str::to_owned)
}
```

Replace the inline pointer with `parent_session_id(payload, session_id)`.

- [ ] **Step 5: Run parser tests**

Run:

```bash
cd app/src-tauri
cargo test sessions::parser
```

Expected: all parser tests PASS, including the old `thread_spawn` fixture.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/tests/fixtures/sessions/guardian.jsonl app/src-tauri/src/sessions/parser.rs
git commit -m "fix: attach guardian usage to its session"
```

### Task 2: Add GPT-5.5 Pricing

**Files:**
- Modify: `app/src-tauri/src/sessions/pricing.rs`

- [ ] **Step 1: Write the failing GPT-5.5 price test**

```rust
#[test]
fn prices_gpt_5_5_at_the_standard_short_context_rate() {
    let tokens = TokenBreakdown {
        input_tokens: 1_000_000,
        cached_input_tokens: 500_000,
        output_tokens: 200_000,
        reasoning_output_tokens: 0,
    };
    let cost = PriceCatalog::built_in()
        .estimate("gpt-5.5", 1_785_283_200, &tokens)
        .unwrap();
    assert_eq!(cost, 8.75);
}
```

- [ ] **Step 2: Verify the test fails**

Run:

```bash
cd app/src-tauri
cargo test sessions::pricing::tests::prices_gpt_5_5_at_the_standard_short_context_rate
```

Expected: FAIL because the catalog returns `None`.

- [ ] **Step 3: Add the catalog entry**

Add the official pricing source comment and entry:

```rust
// https://developers.openai.com/api/docs/pricing
PriceEntry {
    model: "gpt-5.5",
    effective_from: 0,
    input_per_million: 5.0,
    cached_input_per_million: 0.5,
    output_per_million: 30.0,
},
```

- [ ] **Step 4: Run pricing tests**

Run:

```bash
cd app/src-tauri
cargo test sessions::pricing
```

Expected: all pricing tests PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/sessions/pricing.rs
git commit -m "feat: price gpt-5.5 session usage"
```

### Task 3: Aggregate Review Usage Without Mislabeling the Session

**Files:**
- Modify: `app/src-tauri/src/domain/session.rs`
- Modify: `app/src-tauri/src/storage/repository.rs`

- [ ] **Step 1: Write failing repository tests**

Extend repository tests with:

```rust
#[test]
fn review_usage_is_merged_but_never_becomes_the_primary_model() {
    let repository = AccountRepository::open_in_memory().unwrap();
    import_fixture(
        &repository,
        include_bytes!("../../tests/fixtures/sessions/root.jsonl"),
        "root.jsonl",
    );
    import_fixture(
        &repository,
        include_bytes!("../../tests/fixtures/sessions/guardian.jsonl"),
        "guardian.jsonl",
    );

    let sessions = repository.local_session_view(2_000).unwrap().sessions;
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, "root-1");
    assert_ne!(sessions[0].primary_model.as_deref(), Some("codex-auto-review"));
    assert_eq!(sessions[0].unpriced_tokens, 350);
    assert!(sessions[0].equivalent_cost_usd.is_some());
}

#[test]
fn partial_pricing_keeps_the_known_cost() {
    let repository = AccountRepository::open_in_memory().unwrap();
    import_fixture(
        &repository,
        include_bytes!("../../tests/fixtures/sessions/root.jsonl"),
        "root.jsonl",
    );
    import_fixture(
        &repository,
        include_bytes!("../../tests/fixtures/sessions/guardian.jsonl"),
        "guardian.jsonl",
    );

    let session = repository.local_session_view(2_000).unwrap().sessions.remove(0);
    assert!(session.equivalent_cost_usd.unwrap() > 0.0);
    assert!(session.priced_tokens > 0);
    assert_eq!(session.unpriced_tokens, 350);
}
```

Also add an orphan-review test using a parsed file whose guardian owner is absent; expect an empty visible `sessions` list.

- [ ] **Step 2: Verify repository tests fail**

Run:

```bash
cd app/src-tauri
cargo test storage::repository::tests
```

Expected: compile failure for missing token-count fields, followed by behavioral failures after fields are added.

- [ ] **Step 3: Extend the serialized domain**

Add to `SessionSummary`:

```rust
pub priced_tokens: i64,
pub unpriced_tokens: i64,
```

Update all Rust test constructors for `SessionSummary` with explicit values.

- [ ] **Step 4: Track metadata model and pricing coverage**

Extend `SessionMetadataRow` with `model: Option<String>` and select `model` in
`load_session_metadata`.

Replace `has_unknown_cost` in `SessionAccumulator` with:

```rust
priced_tokens: i64,
unpriced_tokens: i64,
```

Use:

```rust
fn is_internal_review_model(model: &str) -> bool {
    matches!(model, "codex-auto-review")
}
```

For each event:

```rust
let event_tokens = event.tokens.total();
if let Some(model) = event.model.as_deref() {
    if !is_internal_review_model(model) {
        *group.model_weights.entry(model.to_owned()).or_default() += event_tokens;
    }
    if let Some(cost) = catalog.estimate(model, event.occurred_at, &event.tokens) {
        group.cost_usd += cost;
        group.priced_tokens += event_tokens;
    } else {
        group.unpriced_tokens += event_tokens;
    }
} else {
    group.unpriced_tokens += event_tokens;
}
```

Serialize `equivalent_cost_usd` when `priced_tokens > 0` or total tokens are zero:

```rust
equivalent_cost_usd: (group.priced_tokens > 0 || group.tokens.total() == 0)
    .then_some(group.cost_usd),
priced_tokens: group.priced_tokens,
unpriced_tokens: group.unpriced_tokens,
```

- [ ] **Step 5: Suppress orphan internal-review roots**

Before mapping groups to summaries, filter a group only when all of these are true:

```rust
let root_metadata = metadata.get(&group.root_id);
let orphan_internal_review = group.model_weights.is_empty()
    && root_metadata
        .and_then(|item| item.model.as_deref())
        .is_some_and(is_internal_review_model);
!orphan_internal_review
```

Do not suppress roots merely because `primary_model` is absent; ordinary empty sessions remain visible.

- [ ] **Step 6: Run repository and full Rust tests**

Run:

```bash
cd app/src-tauri
cargo test storage::repository::tests
cargo test
```

Expected: all Rust tests PASS.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/src/domain/session.rs app/src-tauri/src/storage/repository.rs
git commit -m "fix: keep review usage out of primary models"
```

### Task 4: Show Honest Partial Costs and All Session Rows

**Files:**
- Modify: `app/src/types/dashboard.ts`
- Modify: `app/src/lib/backend.ts`
- Modify: `app/src/components/SessionDetails.tsx`
- Modify: `app/src/components/Dashboard.test.tsx`

- [ ] **Step 1: Replace the eight-row test with scrolling and cost tests**

Update mock sessions with:

```ts
pricedTokens: 1_200,
unpricedTokens: 350,
```

Replace “shows only the eight most recent sessions” with:

```ts
test("renders all sessions inside the session scroll region", () => {
  const sessions = Array.from({ length: 12 }, (_, index) => ({
    sessionId: `session-${index}`,
    title: `会话 ${index}`,
    projectPath: `/tmp/project-${index}`,
    lastActiveAt: 1_785_330_000 - index,
    primaryModel: "gpt-5.5",
    tokens: {
      inputTokens: 1_000,
      cachedInputTokens: 400,
      outputTokens: 200,
      reasoningOutputTokens: 50,
    },
    equivalentCostUsd: 0.01,
    pricedTokens: 1_200,
    unpricedTokens: 0,
    childSessionCount: 0,
  }));
  const { container } = render(
    <DashboardView
      snapshot={{
        ...snapshot,
        localSessions: { ...snapshot.localSessions, sessions },
      }}
      loading={false}
      refreshing={false}
      error={null}
      onRefresh={vi.fn()}
    />,
  );
  expect(container.querySelectorAll("tbody .session-row")).toHaveLength(12);
  expect(screen.getByText("12 个会话")).toBeVisible();
  expect(container.querySelector(".session-table-wrap")).toHaveClass(
    "session-table-wrap",
  );
});
```

Add:

```ts
test("shows known cost as a lower bound when some tokens are unpriced", () => {
  renderDashboard({
    ...snapshot,
    localSessions: {
      ...snapshot.localSessions,
      sessions: [{
        sessionId: "mixed-1",
        title: "example-project · 7月29日",
        projectPath: "/tmp/example-project",
        lastActiveAt: 1_785_330_000,
        primaryModel: "gpt-5.5",
        tokens: {
          inputTokens: 1_300,
          cachedInputTokens: 500,
          outputTokens: 250,
          reasoningOutputTokens: 50,
        },
        equivalentCostUsd: 0.42,
        pricedTokens: 1_200,
        unpricedTokens: 350,
        childSessionCount: 1,
      }],
    },
  });
  expect(screen.getByText("≥ US$0.42")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: /example-project/ }));
  expect(screen.getByText("350 未定价 Token")).toBeVisible();
});

test("never presents codex-auto-review as the main model", () => {
  renderDashboard({
    ...snapshot,
    localSessions: {
      ...snapshot.localSessions,
      sessions: [{
        sessionId: "gpt-5-5",
        title: "priced-session",
        projectPath: "/tmp/priced",
        lastActiveAt: 1_785_330_000,
        primaryModel: "gpt-5.5",
        tokens: {
          inputTokens: 1_000,
          cachedInputTokens: 400,
          outputTokens: 200,
          reasoningOutputTokens: 50,
        },
        equivalentCostUsd: 0.01,
        pricedTokens: 1_200,
        unpricedTokens: 0,
        childSessionCount: 0,
      }],
    },
  });
  expect(screen.getByText("gpt-5.5")).toBeVisible();
  expect(screen.queryByText("codex-auto-review")).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run the focused frontend tests and verify failure**

Run:

```bash
cd app
pnpm test --run src/components/Dashboard.test.tsx
```

Expected: failure because only eight rows render and partial cost has no representation.

- [ ] **Step 3: Extend TypeScript types and mocks**

Add to `SessionSummary`:

```ts
pricedTokens: number;
unpricedTokens: number;
```

Populate the fields in `previewSnapshot` and every dashboard test fixture.

- [ ] **Step 4: Update session rendering**

Remove `SESSION_DISPLAY_LIMIT`, render `view.sessions`, and change the counter to:

```tsx
<span className="session-scan-time">{view.sessions.length} 个会话</span>
```

Use:

```ts
function formatCost(session: SessionSummary) {
  if (session.equivalentCostUsd === null) return "费用待定";
  const prefix = session.unpricedTokens > 0 ? "≥" : "≈";
  const digits = session.equivalentCostUsd < 1 ? 2 : 1;
  return `${prefix} US$${session.equivalentCostUsd.toFixed(digits)}`;
}
```

Add `title` to the full title, project, and model cells. In the expanded breakdown render:

```tsx
{session.unpricedTokens > 0 && (
  <div>
    <dt>未定价</dt>
    <dd>{fullNumber.format(session.unpricedTokens)} 未定价 Token</dd>
  </div>
)}
```

- [ ] **Step 5: Run frontend tests**

Run:

```bash
cd app
pnpm test --run src/components/Dashboard.test.tsx
```

Expected: focused tests PASS.

- [ ] **Step 6: Commit**

```bash
git add app/src/types/dashboard.ts app/src/lib/backend.ts app/src/components/SessionDetails.tsx app/src/components/Dashboard.test.tsx
git commit -m "feat: show complete session cost coverage"
```

### Task 5: Fix Session Table Layout

**Files:**
- Modify: `app/src/styles/app.css`
- Modify: `app/src/components/Dashboard.test.tsx`

- [ ] **Step 1: Add structural assertions**

Add a table layout test:

```ts
test("uses a vertical session scroller and fixed columns", () => {
  const { container } = render(
    <DashboardView
      snapshot={{
        ...snapshot,
        localSessions: {
          ...snapshot.localSessions,
          sessions: [{
            sessionId: "layout-1",
            title: "layout session",
            projectPath: "/tmp/layout",
            lastActiveAt: 1_785_330_000,
            primaryModel: "gpt-5.5",
            tokens: {
              inputTokens: 1_000,
              cachedInputTokens: 400,
              outputTokens: 200,
              reasoningOutputTokens: 50,
            },
            equivalentCostUsd: 0.01,
            pricedTokens: 1_200,
            unpricedTokens: 0,
            childSessionCount: 0,
          }],
        },
      }}
      loading={false}
      refreshing={false}
      error={null}
      onRefresh={vi.fn()}
    />,
  );
  expect(container.querySelector(".session-table-wrap")).toHaveStyle({
    overflowX: "hidden",
    overflowY: "auto",
  });
  expect(container.querySelector(".session-table")).toHaveStyle({
    tableLayout: "fixed",
  });
});
```

- [ ] **Step 2: Verify the layout test fails**

Run:

```bash
cd app
pnpm test --run src/components/Dashboard.test.tsx
```

Expected: FAIL because the wrapper currently uses horizontal scrolling.

- [ ] **Step 3: Implement fixed layout and sticky header**

Update CSS:

```css
.session-table-wrap {
  max-height: 320px;
  margin-top: 16px;
  overflow-x: hidden;
  overflow-y: auto;
  scrollbar-gutter: stable;
  border: 1px solid var(--line);
  border-radius: 13px;
}

.session-table {
  width: 100%;
  table-layout: fixed;
  border-collapse: collapse;
}

.session-table th {
  position: sticky;
  top: 0;
  z-index: 1;
}

.session-table th:nth-child(2) { width: 112px; }
.session-table th:nth-child(3) { width: 104px; }
.session-table th:nth-child(4) { width: 88px; }
.session-table th:nth-child(5) { width: 104px; }
.session-table th:nth-child(6) { width: 116px; }

.session-table td {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.session-title-button,
.session-title-button > span {
  min-width: 0;
}

.session-title-button strong {
  max-width: none;
}
```

Right-align Token and equivalent-cost headers/cells while leaving the activity column left-aligned.

- [ ] **Step 4: Run tests and production build**

Run:

```bash
cd app
pnpm test --run
pnpm build
```

Expected: all frontend tests PASS and production build succeeds.

- [ ] **Step 5: Commit**

```bash
git add app/src/styles/app.css app/src/components/Dashboard.test.tsx
git commit -m "fix: make session details vertically scrollable"
```

### Task 6: Verify Session Accuracy End to End

**Files:**
- Modify only if verification exposes a defect.

- [ ] **Step 1: Format and lint**

Run:

```bash
cd app/src-tauri
cargo fmt --check
cd ..
pnpm lint
```

Expected: formatter passes; lint exits zero with no new warnings.

- [ ] **Step 2: Run full test suites**

Run:

```bash
cd app/src-tauri
cargo test
cd ..
pnpm test --run
pnpm build
```

Expected: Rust and frontend tests all PASS; production build succeeds.

- [ ] **Step 3: Inspect real aggregation**

Launch the app against the existing local database and verify:

- no visible row has `codex-auto-review` as its primary model;
- known GPT-5.5 usage has a numeric equivalent cost;
- mixed sessions show `≥ US$...` plus unpriced internal Token;
- the table scrolls vertically without horizontal scrolling.

- [ ] **Step 4: Commit any verification-only corrections**

If no corrections are needed, skip this commit. Otherwise:

```bash
git add app
git commit -m "fix: align session summary with live data"
```
