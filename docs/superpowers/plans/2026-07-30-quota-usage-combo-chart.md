# Quota Usage Combo Chart Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Overlay a downward remaining-quota line and pace band on the existing seven-day Token chart without adding another dashboard panel.

**Architecture:** Derive a compact trend payload from the repository's current rate-limit segment, using the same robust slope basis as exhaustion forecasting. Serialize history and pace in the dashboard snapshot, then render one accessible SVG layer behind the existing daily Token columns so both series share dates but retain independent scales.

**Tech Stack:** Rust, chrono, serde, Tauri, React, TypeScript, SVG, Vitest, Testing Library, CSS

---

## File Structure

- Create `app/src-tauri/src/quota_trend.rs`: current-segment downsampling, remaining percentage conversion, and pace classification.
- Modify `app/src-tauri/src/lib.rs`: register the new Rust module.
- Modify `app/src-tauri/src/domain/dashboard.rs`: serialize quota history and pace.
- Modify `app/src-tauri/src/monitor.rs`: compute forecast and trend from one current-segment read.
- Modify `app/src-tauri/src/tray.rs`: update dashboard snapshot test fixtures.
- Modify `app/src/types/dashboard.ts`: mirror quota history and pace types.
- Create `app/src/components/UsageQuotaChart.tsx`: render the combined Token and remaining-quota chart.
- Create `app/src/components/UsageQuotaChart.test.tsx`: test direction, pace labels, and insufficient samples.
- Modify `app/src/components/UsageForecastPanel.tsx`: use the combined chart inside the current panel.
- Modify `app/src/components/Dashboard.test.tsx`: update snapshot fixture and integrated chart assertions.
- Modify `app/src/lib/backend.ts`: add realistic preview history.
- Modify `app/src/styles/app.css`: style the overlay, legend, pace pill, and responsive states.
- Delete `app/public/usage-chart-preview.html`: remove the temporary design-only page after the real component is available.

### Task 1: Define Quota Trend and Pace

**Files:**
- Create: `app/src-tauri/src/quota_trend.rs`
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing unit tests in the new module**

Define tests for:

```rust
#[test]
fn converts_used_percent_to_downward_remaining_history() {
    let trend = build_trend(&samples(&[
        (0, 10.0),
        (86_400, 25.0),
        (172_800, 40.0),
    ]), 10_080);
    assert_eq!(
        trend.history.iter().map(|point| point.remaining_percent).collect::<Vec<_>>(),
        vec![90.0, 75.0, 60.0]
    );
}

#[test]
fn classifies_seven_day_pace_with_a_twenty_percent_band() {
    assert_eq!(classify_pace(10.0, 10_080), QuotaPaceStatus::Slow);
    assert_eq!(classify_pace(14.2857, 10_080), QuotaPaceStatus::Normal);
    assert_eq!(classify_pace(18.0, 10_080), QuotaPaceStatus::Fast);
}

#[test]
fn keeps_first_last_and_daily_latest_points_under_the_limit() {
    let trend = build_trend(&dense_samples(), 10_080);
    assert!(trend.history.len() <= 64);
    assert_eq!(trend.history.first().unwrap().observed_at, dense_samples()[0].observed_at);
    assert_eq!(trend.history.last().unwrap().observed_at, dense_samples().last().unwrap().observed_at);
}
```

- [ ] **Step 2: Run the new tests and verify compile failure**

Run:

```bash
cd app/src-tauri
cargo test quota_trend
```

Expected: compile failure because the module and types do not exist.

- [ ] **Step 3: Implement focused trend types**

Create:

```rust
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuotaHistoryPoint {
    pub observed_at: i64,
    pub remaining_percent: f64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuotaPaceStatus {
    Slow,
    Normal,
    Fast,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuotaPace {
    pub percent_per_day: f64,
    pub ideal_percent_per_day: f64,
    pub status: QuotaPaceStatus,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuotaTrend {
    pub history: Vec<QuotaHistoryPoint>,
    pub pace: Option<QuotaPace>,
}
```

`build_trend(points, window_duration_mins)` sorts observations, converts
`remaining_percent = (100.0 - used_percent).clamp(0.0, 100.0)`, retains daily last points and
endpoints up to 64, and returns `pace: None` until there are at least three points spanning
30 minutes.

Use the median pairwise used-percentage slope in percent per day:

```rust
let percent_per_day = median_pairwise_hourly_slope(points)? * 24.0;
let ideal = 100.0 / (window_duration_mins as f64 / 1_440.0);
let status = if percent_per_day < ideal * 0.8 {
    QuotaPaceStatus::Slow
} else if percent_per_day > ideal * 1.2 {
    QuotaPaceStatus::Fast
} else {
    QuotaPaceStatus::Normal
};
```

- [ ] **Step 4: Register and test the module**

Add `mod quota_trend;` in `lib.rs`.

Run:

```bash
cd app/src-tauri
cargo test quota_trend
```

Expected: all trend tests PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/quota_trend.rs app/src-tauri/src/lib.rs
git commit -m "feat: derive remaining quota trend and pace"
```

### Task 2: Add Trend Data to Dashboard Snapshots

**Files:**
- Modify: `app/src-tauri/src/domain/dashboard.rs`
- Modify: `app/src-tauri/src/monitor.rs`
- Modify: `app/src-tauri/src/tray.rs`

- [ ] **Step 1: Add a failing monitor assertion**

Extend `startup_refresh_reads_quota_and_account_usage`:

```rust
assert_eq!(snapshot.quota_history.last().unwrap().remaining_percent, 82.0);
```

The startup fixture has one account sample, so also assert
`assert_eq!(snapshot.quota_pace, None);`; pace classification remains covered by the focused
`quota_trend` unit tests.

- [ ] **Step 2: Verify monitor tests fail**

Run:

```bash
cd app/src-tauri
cargo test monitor::tests
```

Expected: compile failure because snapshot trend fields do not exist.

- [ ] **Step 3: Extend `DashboardSnapshot`**

Add:

```rust
pub quota_history: Vec<QuotaHistoryPoint>,
pub quota_pace: Option<QuotaPace>,
```

Initialize them to `Vec::new()` and `None` in `Default`, and update all Rust snapshot literals.

- [ ] **Step 4: Reuse one repository segment read**

In `Monitor::refresh`, replace the forecast-only closure with:

```rust
let segment = primary_quota
    .as_ref()
    .and_then(|quota| {
        self.repository
            .current_segment(&quota.limit_id, &quota.window_kind)
            .ok()
            .map(|points| (quota, points))
    });
let forecast = segment.as_ref().and_then(|(quota, points)| {
    forecast::forecast(points, now, quota.resets_at)
});
let trend = segment
    .as_ref()
    .map(|(quota, points)| quota_trend::build_trend(points, quota.window_duration_mins))
    .unwrap_or_default();
```

Assign `trend.history` and `trend.pace` to the snapshot.

- [ ] **Step 5: Run monitor and full Rust tests**

Run:

```bash
cd app/src-tauri
cargo test monitor::tests
cargo test
```

Expected: all Rust tests PASS.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/domain/dashboard.rs app/src-tauri/src/monitor.rs app/src-tauri/src/tray.rs
git commit -m "feat: expose account quota history"
```

### Task 3: Build the Combined Chart Component

**Files:**
- Create: `app/src/components/UsageQuotaChart.tsx`
- Create: `app/src/components/UsageQuotaChart.test.tsx`
- Modify: `app/src/types/dashboard.ts`

- [ ] **Step 1: Add TypeScript types**

```ts
export type QuotaPaceStatus = "slow" | "normal" | "fast";

export interface QuotaHistoryPoint {
  observedAt: number;
  remainingPercent: number;
}

export interface QuotaPace {
  percentPerDay: number;
  idealPercentPerDay: number;
  status: QuotaPaceStatus;
  sampleCount: number;
}
```

Add `quotaHistory: QuotaHistoryPoint[]` and `quotaPace: QuotaPace | null` to
`DashboardSnapshot`.

- [ ] **Step 2: Write failing component tests**

Test the pure render contract:

```tsx
test("renders token bars and a remaining-quota line that descends over time", () => {
  const { container } = render(
    <UsageQuotaChart
      buckets={buckets}
      history={[
        { observedAt: day1, remainingPercent: 90 },
        { observedAt: day2, remainingPercent: 75 },
        { observedAt: day3, remainingPercent: 60 },
      ]}
      pace={{ percentPerDay: 15, idealPercentPerDay: 14.2857, status: "normal", sampleCount: 3 }}
    />,
  );
  expect(container.querySelectorAll(".token-bar")).toHaveLength(7);
  const path = container.querySelector('[data-testid="remaining-quota-line"]');
  expect(path).toHaveAttribute("d", expect.stringMatching(/^M/));
  expect(screen.getByText("正常 · 15.0%/天")).toBeVisible();
});

test("does not invent a line from one quota sample", () => {
  const { container } = render(
    <UsageQuotaChart buckets={buckets} history={[onePoint]} pace={null} />,
  );
  expect(container.querySelector('[data-testid="remaining-quota-line"]')).toBeNull();
  expect(screen.getByText("正在积累额度样本")).toBeVisible();
});
```

Parse the line coordinates and assert later `y` values are larger when remaining percentage is lower.

- [ ] **Step 3: Run the new test and verify failure**

Run:

```bash
cd app
pnpm test --run src/components/UsageQuotaChart.test.tsx
```

Expected: compile failure because the component does not exist.

- [ ] **Step 4: Implement SVG geometry**

Create a component with constants:

```ts
const WIDTH = 700;
const HEIGHT = 150;
const TOP = 12;
const BOTTOM = 24;
const plotHeight = HEIGHT - TOP - BOTTOM;
const quotaY = (remaining: number) =>
  TOP + (1 - Math.min(100, Math.max(0, remaining)) / 100) * plotHeight;
```

Map the last seven bucket dates to evenly spaced x positions. Align history points by local
`YYYY-MM-DD`; use the last observation for each day. Build a path only with two or more aligned
points:

```ts
const remainingPath = aligned
  .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${quotaY(point.remainingPercent)}`)
  .join(" ");
```

Render, in order:

1. horizontal grid lines;
2. clipped green ideal band and dashed ideal line;
3. existing Token columns;
4. warm-white remaining quota path and point markers;
5. legend and pace pill.

Use `aria-label="最近 7 日 Token 与剩余额度"` and keep exact values in SVG `<title>` nodes.

- [ ] **Step 5: Run component tests**

Run:

```bash
cd app
pnpm test --run src/components/UsageQuotaChart.test.tsx
```

Expected: all chart tests PASS.

- [ ] **Step 6: Commit**

```bash
git add app/src/types/dashboard.ts app/src/components/UsageQuotaChart.tsx app/src/components/UsageQuotaChart.test.tsx
git commit -m "feat: render token and remaining quota together"
```

### Task 4: Integrate the Chart Into the Existing Usage Panel

**Files:**
- Modify: `app/src/components/UsageForecastPanel.tsx`
- Modify: `app/src/components/Dashboard.tsx`
- Modify: `app/src/components/Dashboard.test.tsx`
- Modify: `app/src/lib/backend.ts`

- [ ] **Step 1: Add an integrated dashboard test**

Extend the snapshot with decreasing remaining history and add:

```ts
test("combines daily tokens and remaining quota in the existing usage panel", () => {
  const { container } = renderDashboard();
  expect(screen.getByRole("heading", { name: "Token 与额度趋势" })).toBeVisible();
  expect(container.querySelectorAll(".usage-panel")).toHaveLength(1);
  expect(container.querySelector('[data-testid="remaining-quota-line"]')).toBeInTheDocument();
  expect(screen.getByText("剩余额度")).toBeVisible();
});
```

- [ ] **Step 2: Verify the integrated test fails**

Run:

```bash
cd app
pnpm test --run src/components/Dashboard.test.tsx
```

Expected: FAIL because the panel still renders only `.token-chart`.

- [ ] **Step 3: Thread trend props into the existing panel**

Extend `UsageForecastPanelProps`:

```ts
history: QuotaHistoryPoint[];
pace: QuotaPace | null;
```

Pass `snapshot.quotaHistory` and `snapshot.quotaPace` from `Dashboard.tsx`. Change the heading to
`Token 与额度趋势` and replace the old `.token-chart` block with:

```tsx
<UsageQuotaChart
  buckets={usage.dailyUsageBuckets.slice(-7)}
  history={history}
  pace={pace}
/>
```

Keep the existing forecast row in the same card.

- [ ] **Step 4: Update preview data**

Add seven decreasing `quotaHistory` points and a `quotaPace` object to `previewSnapshot`; update
all dashboard test fixtures.

- [ ] **Step 5: Run integrated frontend tests**

Run:

```bash
cd app
pnpm test --run src/components/Dashboard.test.tsx src/components/UsageQuotaChart.test.tsx
```

Expected: all focused tests PASS.

- [ ] **Step 6: Commit**

```bash
git add app/src/components/UsageForecastPanel.tsx app/src/components/Dashboard.tsx app/src/components/Dashboard.test.tsx app/src/lib/backend.ts
git commit -m "feat: integrate remaining quota into usage trend"
```

### Task 5: Match the Approved Visual and Remove the Temporary Preview

**Files:**
- Modify: `app/src/styles/app.css`
- Delete: `app/public/usage-chart-preview.html`

- [ ] **Step 1: Add the approved chart styling**

Replace `.token-chart` layout rules with:

```css
.usage-combo-chart {
  position: relative;
  min-height: 146px;
  margin-top: 14px;
}

.usage-combo-chart svg {
  display: block;
  width: 100%;
  height: 146px;
  overflow: visible;
}

.quota-normal-band {
  fill: color-mix(in srgb, var(--success) 10%, transparent);
}

.quota-ideal-line {
  fill: none;
  stroke: var(--success);
  stroke-dasharray: 4 5;
  opacity: 0.72;
}

.remaining-quota-line {
  fill: none;
  stroke: var(--text);
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 2.25;
}

.quota-pace-pill.fast { color: var(--danger); }
.quota-pace-pill.normal { color: var(--success); }
.quota-pace-pill.slow { color: var(--warning); }
```

Keep bars blue, reduce their opacity behind the quota line, and include a compact legend below the
plot. Under 760px, reduce date labels rather than introducing horizontal scrolling.

- [ ] **Step 2: Delete the design-only preview**

Remove `app/public/usage-chart-preview.html`; the real dashboard at the Vite root now provides the
preview.

- [ ] **Step 3: Run frontend verification**

Run:

```bash
cd app
pnpm test --run
pnpm lint
pnpm build
```

Expected: all frontend tests PASS, lint exits zero with no new warning, and build succeeds.

- [ ] **Step 4: Commit**

```bash
git add app/src/styles/app.css app/public/usage-chart-preview.html
git commit -m "style: finish remaining quota combo chart"
```

### Task 6: Full Product Verification

**Files:**
- Modify only if verification exposes a defect.

- [ ] **Step 1: Run all automated checks**

Run:

```bash
cd app/src-tauri
cargo fmt --check
cargo test
cd ..
pnpm test --run
pnpm lint
pnpm build
```

Expected: all tests and builds PASS; no new lint warnings.

- [ ] **Step 2: Inspect the live dashboard**

Launch the app and verify at normal macOS window size:

- the white remaining-quota line descends left to right;
- Token bars remain readable behind the line;
- the green band is visually secondary;
- the pace pill matches the numeric rate;
- there is still only one right-side usage panel;
- one quota sample shows “正在积累额度样本” without a fake line;
- the card height remains aligned with the seven-day quota card.

- [ ] **Step 3: Inspect reset behavior**

Use a test snapshot with observations from two reset segments and verify only the current segment
appears. Confirm no upward connector crosses a reset.

- [ ] **Step 4: Commit any verification-only corrections**

If corrections were required:

```bash
git add app
git commit -m "fix: align quota trend with live observations"
```
