# Session Details, Menu Bar, and Brand Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import trustworthy local Codex session usage, roll child-agent usage into top-level sessions, expose it in the dashboard and menu bar, correct the quota progress semantics, and replace the Codex Pacer-like icon with the approved “remaining window” identity.

**Architecture:** A Rust session subsystem discovers and incrementally parses Codex JSONL files into idempotent SQLite source tables, then derives top-level session summaries and versioned equivalent-cost estimates. `AccountMonitor` combines those local summaries with the existing authoritative account snapshot; React renders the combined read model, while a stateful Tauri tray controller formats the same snapshot into native menu items. Brand assets come from committed SVG masters and Tauri’s icon generator.

**Tech Stack:** Rust, Serde JSON, Rusqlite, Tokio, notify, blake3, Chrono, Tauri 2 native tray/menu, React 19, TypeScript, Vitest, Testing Library, SVG, Tauri CLI.

---

## File Structure

### Rust session subsystem

- Create `app/src-tauri/src/domain/session.rs`: normalized token event and dashboard session read models.
- Create `app/src-tauri/src/sessions/mod.rs`: public session subsystem exports.
- Create `app/src-tauri/src/sessions/discovery.rs`: Codex home resolution and active/archive JSONL discovery.
- Create `app/src-tauri/src/sessions/parser.rs`: privacy-preserving JSONL parser.
- Create `app/src-tauri/src/sessions/importer.rs`: incremental offsets, file generations, transactions, and reconciliation.
- Create `app/src-tauri/src/sessions/pricing.rs`: effective-dated model aliases and equivalent API cost calculation.
- Create `app/src-tauri/src/sessions/service.rs`: startup scan, filesystem watch, low-frequency reconciliation, and snapshot publication.
- Create `app/src-tauri/migrations/002_sessions.sql`: source, lineage, pricing, and derived-session tables.
- Create `app/src-tauri/tests/fixtures/sessions/*.jsonl`: synthetic root, child, replay, archive, and malformed fixtures containing no user data.

### Existing Rust integration

- Modify `app/src-tauri/src/domain/mod.rs`: export session domain.
- Modify `app/src-tauri/src/domain/dashboard.rs`: add local session snapshot fields.
- Modify `app/src-tauri/src/storage/migrations.rs`: run migration 2 idempotently.
- Modify `app/src-tauri/src/storage/repository.rs`: expose transaction helpers needed by the importer.
- Modify `app/src-tauri/src/monitor.rs`: merge account and local-session read models.
- Modify `app/src-tauri/src/commands.rs`: add rescan and section-focus commands.
- Modify `app/src-tauri/src/tray.rs`: replace the static menu with a stateful menu controller.
- Modify `app/src-tauri/src/lib.rs`: start and stop the session service, register commands.
- Modify `app/src-tauri/Cargo.toml`: add `notify`, `blake3`, `dirs`, and test-only `tempfile`.

### React

- Modify `app/src/types/dashboard.ts`: add session summary, breakdown, and diagnostics types.
- Modify `app/src/lib/backend.ts`: add realistic preview sessions and section-focus event support.
- Modify `app/src/components/SessionDetails.tsx`: render expandable top-level session rows.
- Modify `app/src/components/Dashboard.tsx`: pass snapshot data and focus the requested section.
- Modify `app/src/components/QuotaCard.tsx`: label both used and remaining percentages.
- Modify `app/src/components/AppSidebar.tsx`: use the new brand mark.
- Modify `app/src/components/Dashboard.test.tsx`: cover session aggregation, errors, focus, and quota semantics.
- Modify `app/src/styles/app.css`: session table, expansion, focus highlight, and quota threshold styles.

### Brand assets

- Create `app/brand/monitor-window.svg`: approved colored vector master.
- Create `app/brand/tray-template.svg`: simplified monochrome menu-bar derivative.
- Create `app/src/components/BrandMark.tsx`: reusable inline vector mark.
- Replace generated files in `app/src-tauri/icons/`.
- Replace `app/public/favicon.svg`.

## Task 1: Add Session Domain Types and Database Migration

**Files:**
- Create: `app/src-tauri/src/domain/session.rs`
- Modify: `app/src-tauri/src/domain/mod.rs`
- Create: `app/src-tauri/migrations/002_sessions.sql`
- Modify: `app/src-tauri/src/storage/migrations.rs`
- Test: `app/src-tauri/src/storage/migrations.rs`

- [ ] **Step 1: Write a failing migration test**

Add a test that opens a database containing migration 1, inserts one account observation, runs
the migration runner twice, and asserts both the old row and the session tables remain:

```rust
#[test]
fn migration_two_is_idempotent_and_preserves_account_observations() {
    let mut connection = Connection::open_in_memory().unwrap();
    run(&mut connection).unwrap();
    connection.execute(
        "INSERT INTO account_usage_observations(
           observed_at, lifetime_tokens, peak_daily_tokens, daily_buckets_json, payload_json
         ) VALUES (1, 10, 5, NULL, '{}')",
        [],
    ).unwrap();

    run(&mut connection).unwrap();
    run(&mut connection).unwrap();

    let account_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM account_usage_observations", [], |row| row.get(0))
        .unwrap();
    let session_table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name = 'session_usage_events'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(account_count, 1);
    assert_eq!(session_table_count, 1);
}
```

- [ ] **Step 2: Run the migration test and verify RED**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml storage::migrations::tests::migration_two_is_idempotent_and_preserves_account_observations
```

Expected: FAIL because `session_usage_events` does not exist.

- [ ] **Step 3: Define the session domain models**

Create `domain/session.rs` with serialization matching the frontend:

```rust
#[derive(Debug, Clone, Default, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenBreakdown {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
}

impl TokenBreakdown {
    pub fn total(&self) -> i64 {
        self.input_tokens + self.output_tokens
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: String,
    pub title: String,
    pub project_path: Option<String>,
    pub last_active_at: i64,
    pub primary_model: Option<String>,
    pub tokens: TokenBreakdown,
    pub equivalent_cost_usd: Option<f64>,
    pub child_session_count: i64,
}

#[derive(Debug, Clone, Default, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionDiagnostics {
    pub scanned_files: i64,
    pub skipped_lines: i64,
    pub last_imported_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalSessionView {
    pub sessions: Vec<SessionSummary>,
    pub diagnostics: SessionDiagnostics,
}
```

Export it from `domain/mod.rs` with `pub mod session;`.

- [ ] **Step 4: Add migration 2**

Create `002_sessions.sql` with these exact invariants:

```sql
CREATE TABLE IF NOT EXISTS session_source_files (
  path TEXT NOT NULL,
  generation INTEGER NOT NULL,
  file_identity TEXT,
  byte_offset INTEGER NOT NULL DEFAULT 0,
  observed_size INTEGER NOT NULL DEFAULT 0,
  modified_at INTEGER NOT NULL DEFAULT 0,
  parser_version INTEGER NOT NULL,
  last_error TEXT,
  PRIMARY KEY(path, generation)
);

CREATE TABLE IF NOT EXISTS session_metadata (
  session_id TEXT PRIMARY KEY,
  parent_session_id TEXT,
  started_at INTEGER NOT NULL,
  last_active_at INTEGER NOT NULL,
  cwd TEXT,
  model TEXT,
  source_path TEXT NOT NULL,
  FOREIGN KEY(parent_session_id) REFERENCES session_metadata(session_id)
);

CREATE TABLE IF NOT EXISTS session_usage_events (
  fingerprint TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  occurred_at INTEGER NOT NULL,
  model TEXT,
  input_tokens INTEGER NOT NULL CHECK(input_tokens >= 0),
  cached_input_tokens INTEGER NOT NULL CHECK(cached_input_tokens >= 0),
  output_tokens INTEGER NOT NULL CHECK(output_tokens >= 0),
  reasoning_output_tokens INTEGER NOT NULL CHECK(reasoning_output_tokens >= 0),
  source_path TEXT NOT NULL,
  source_offset INTEGER NOT NULL,
  FOREIGN KEY(session_id) REFERENCES session_metadata(session_id)
);

CREATE INDEX IF NOT EXISTS idx_session_usage_session_time
ON session_usage_events(session_id, occurred_at);

CREATE TABLE IF NOT EXISTS model_price_versions (
  model_pattern TEXT NOT NULL,
  effective_from INTEGER NOT NULL,
  input_per_million REAL NOT NULL,
  cached_input_per_million REAL NOT NULL,
  output_per_million REAL NOT NULL,
  catalog_version TEXT NOT NULL,
  PRIMARY KEY(model_pattern, effective_from)
);
```

Change `migrations.rs` to use a small ordered list:

```rust
const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../../migrations/001_account.sql")),
    (2, include_str!("../../migrations/002_sessions.sql")),
];

for (version, sql) in MIGRATIONS {
    let applied = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
        [version],
        |row| row.get::<_, bool>(0),
    )?;
    if !applied {
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, unixepoch())",
            [version],
        )?;
    }
}
```

- [ ] **Step 5: Run the migration test and all storage tests**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml storage::
```

Expected: PASS with the new migration test and all existing repository tests.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/domain app/src-tauri/src/storage/migrations.rs app/src-tauri/migrations/002_sessions.sql
git commit -m "feat: add local session storage model"
```

## Task 2: Parse Codex Session JSONL Without Persisting Message Bodies

**Files:**
- Modify: `app/src-tauri/Cargo.toml`
- Create: `app/src-tauri/src/sessions/mod.rs`
- Create: `app/src-tauri/src/sessions/parser.rs`
- Create: `app/src-tauri/tests/fixtures/sessions/root.jsonl`
- Create: `app/src-tauri/tests/fixtures/sessions/child.jsonl`
- Create: `app/src-tauri/tests/fixtures/sessions/malformed.jsonl`
- Modify: `app/src-tauri/src/lib.rs`
- Test: `app/src-tauri/src/sessions/parser.rs`

- [ ] **Step 1: Add parser dependencies**

Run:

```bash
cd app
cargo add --manifest-path src-tauri/Cargo.toml blake3 dirs notify
cargo add --manifest-path src-tauri/Cargo.toml --dev tempfile
```

Expected: `Cargo.toml` and `Cargo.lock` contain all four crates.

- [ ] **Step 2: Create synthetic fixtures**

Use only invented text. `root.jsonl` must contain:

```json
{"timestamp":"2026-07-29T08:00:00Z","type":"session_meta","payload":{"id":"root-1","cwd":"/tmp/example-project","source":"vscode","timestamp":"2026-07-29T08:00:00Z"}}
{"timestamp":"2026-07-29T08:00:01Z","type":"turn_context","payload":{"model":"gpt-5.6-codex","cwd":"/tmp/example-project"}}
{"timestamp":"2026-07-29T08:01:00Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":200,"reasoning_output_tokens":50,"total_tokens":1200},"total_token_usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":200,"reasoning_output_tokens":50,"total_tokens":1200}}}}
```

`child.jsonl` must use the real structural source shape:

```json
{"timestamp":"2026-07-29T08:02:00Z","type":"session_meta","payload":{"id":"child-1","cwd":"/tmp/example-project","source":{"subagent":{"thread_spawn":{"parent_thread_id":"root-1","depth":1,"agent_path":"/root/review","agent_nickname":"Ada","agent_role":"reviewer"}}},"timestamp":"2026-07-29T08:02:00Z"}}
{"timestamp":"2026-07-29T08:02:01Z","type":"turn_context","payload":{"model":"gpt-5.6-codex","cwd":"/tmp/example-project"}}
{"timestamp":"2026-07-29T08:03:00Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":300,"cached_input_tokens":100,"output_tokens":80,"reasoning_output_tokens":20,"total_tokens":380},"total_token_usage":{"input_tokens":1300,"cached_input_tokens":500,"output_tokens":280,"reasoning_output_tokens":70,"total_tokens":1580}}}}
```

`malformed.jsonl` starts with a broken JSON line and ends with the same valid root metadata and
one valid token event.

- [ ] **Step 3: Write failing parser tests**

Define the desired API in tests:

```rust
#[test]
fn parses_last_usage_and_subagent_parent_without_message_text() {
    let parsed = parse_reader(
        include_bytes!("../../tests/fixtures/sessions/child.jsonl").as_slice(),
        "child.jsonl",
        0,
    ).unwrap();

    assert_eq!(parsed.metadata.unwrap().parent_session_id.as_deref(), Some("root-1"));
    assert_eq!(parsed.events.len(), 1);
    assert_eq!(parsed.events[0].tokens.input_tokens, 300);
    assert_eq!(parsed.events[0].model.as_deref(), Some("gpt-5.6-codex"));
    assert!(!serde_json::to_string(&parsed).unwrap().contains("agent_nickname"));
}

#[test]
fn skips_a_bad_line_and_continues_at_the_next_complete_line() {
    let parsed = parse_reader(
        include_bytes!("../../tests/fixtures/sessions/malformed.jsonl").as_slice(),
        "malformed.jsonl",
        0,
    ).unwrap();
    assert_eq!(parsed.skipped_lines, 1);
    assert_eq!(parsed.events.len(), 1);
}
```

- [ ] **Step 4: Run parser tests and verify RED**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml sessions::parser::tests
```

Expected: FAIL because `parse_reader` and parser types do not exist.

- [ ] **Step 5: Implement the minimal parser**

Define private Serde structs only for `session_meta`, `turn_context`, and
`event_msg.payload.type == "token_count"`. Parse `last_token_usage`, never infer billable usage
from `total_token_usage`, and hash structural fields:

```rust
pub struct ParsedFile {
    pub metadata: Option<ParsedSessionMetadata>,
    pub events: Vec<ParsedUsageEvent>,
    pub next_offset: u64,
    pub skipped_lines: i64,
}

fn fingerprint(
    session_id: &str,
    model: Option<&str>,
    tokens: &TokenBreakdown,
    cumulative_total_tokens: Option<i64>,
) -> String {
    let value = format!(
        "{session_id}\0{}\0{}\0{}\0{}\0{}\0{}",
        model.unwrap_or(""),
        tokens.input_tokens,
        tokens.cached_input_tokens,
        tokens.output_tokens,
        tokens.reasoning_output_tokens,
        cumulative_total_tokens.unwrap_or(-1),
    );
    blake3::hash(value.as_bytes()).to_hex().to_string()
}
```

Track the most recent `turn_context.payload.model`. Read `parent_thread_id` only from
`session_meta.payload.source.subagent.thread_spawn`. Generate the UI title as
`"<cwd basename> · <local start date>"`; do not read or persist user messages, tool outputs,
reasoning, `base_instructions`, summaries, or agent nicknames. The cumulative total participates
only in the fingerprint so the same event remains stable after a file is moved to
`archived_sessions`; it is never added to billable Token.

- [ ] **Step 6: Run parser tests**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml sessions::parser::tests
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/Cargo.toml app/src-tauri/Cargo.lock app/src-tauri/src/sessions app/src-tauri/src/lib.rs app/src-tauri/tests/fixtures/sessions
git commit -m "feat: parse local Codex session usage"
```

## Task 3: Import Files Incrementally and Idempotently

**Files:**
- Create: `app/src-tauri/src/sessions/discovery.rs`
- Create: `app/src-tauri/src/sessions/importer.rs`
- Modify: `app/src-tauri/src/sessions/mod.rs`
- Modify: `app/src-tauri/src/storage/repository.rs`
- Test: `app/src-tauri/src/sessions/importer.rs`

- [ ] **Step 1: Write failing importer tests**

Use a temporary Codex home with `sessions/YYYY/MM/DD` and `archived_sessions`. Test the three
critical behaviors:

```rust
#[test]
fn importing_the_same_file_twice_does_not_duplicate_usage() {
    let fixture = TestCodexHome::with_root_fixture();
    let repository = AccountRepository::open_in_memory().unwrap();
    let importer = SessionImporter::new(&repository, fixture.path());

    importer.reconcile(1_000).unwrap();
    importer.reconcile(2_000).unwrap();

    assert_eq!(repository.session_event_count().unwrap(), 1);
}

#[test]
fn appending_a_complete_line_imports_only_the_new_event() {
    let fixture = TestCodexHome::with_root_fixture();
    let repository = AccountRepository::open_in_memory().unwrap();
    let importer = SessionImporter::new(&repository, fixture.path());
    importer.reconcile(1_000).unwrap();
    fixture.append_usage_event(200, 40);

    importer.reconcile(2_000).unwrap();

    assert_eq!(repository.session_event_count().unwrap(), 2);
}

#[test]
fn truncation_starts_a_new_generation_but_replay_stays_idempotent() {
    let fixture = TestCodexHome::with_root_fixture();
    let repository = AccountRepository::open_in_memory().unwrap();
    let importer = SessionImporter::new(&repository, fixture.path());
    importer.reconcile(1_000).unwrap();
    fixture.rewrite_with_same_fixture();

    importer.reconcile(2_000).unwrap();

    assert_eq!(repository.session_event_count().unwrap(), 1);
    assert_eq!(repository.source_generation_count().unwrap(), 2);
}
```

- [ ] **Step 2: Run importer tests and verify RED**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml sessions::importer::tests
```

Expected: FAIL because `SessionImporter` does not exist.

- [ ] **Step 3: Implement discovery**

Expose:

```rust
pub fn codex_home() -> Result<PathBuf, AppError> {
    if let Some(path) = std::env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(path));
    }
    dirs::home_dir()
        .map(|path| path.join(".codex"))
        .ok_or_else(|| AppError::Unavailable("unable to resolve Codex home".into()))
}

pub fn discover_jsonl(codex_home: &Path) -> Result<Vec<PathBuf>, AppError>
```

`discover_jsonl` recursively collects `.jsonl` files below `sessions` and directly below
`archived_sessions`, normalizes paths, sorts them, and ignores symlinks that resolve outside
the Codex home.

- [ ] **Step 4: Add repository transaction methods**

Add:

```rust
pub fn import_session_file(
    &self,
    state: &SourceFileState,
    parsed: ParsedFile,
    imported_at: i64,
) -> Result<(), AppError>
```

Within one SQLite transaction:

1. upsert session metadata;
2. `INSERT OR IGNORE` every event fingerprint;
3. update the exact `(path, generation)` offset, size, mtime, parser version, and error;
4. commit.

Add read helpers used by tests and later aggregation:

```rust
pub fn latest_source_state(&self, path: &str) -> Result<Option<SourceFileState>, AppError>;
pub fn session_event_count(&self) -> Result<i64, AppError>;
pub fn source_generation_count(&self) -> Result<i64, AppError>;
```

- [ ] **Step 5: Implement reconciliation**

`SessionImporter::reconcile(now)` discovers files, compares size and file identity with the
latest state, and either resumes from `byte_offset` or inserts generation `previous + 1` with
offset zero. A parser error updates `last_error` without advancing beyond the last complete
newline. Return:

```rust
pub struct ImportReport {
    pub scanned_files: i64,
    pub imported_events: i64,
    pub skipped_lines: i64,
    pub last_error: Option<String>,
}
```

- [ ] **Step 6: Run importer and storage tests**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml sessions::importer
cargo test --manifest-path app/src-tauri/Cargo.toml storage::
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/src/sessions app/src-tauri/src/storage/repository.rs
git commit -m "feat: import Codex session files idempotently"
```

## Task 4: Aggregate Child Sessions and Calculate Equivalent Cost

**Files:**
- Create: `app/src-tauri/src/sessions/pricing.rs`
- Modify: `app/src-tauri/src/storage/repository.rs`
- Modify: `app/src-tauri/src/sessions/importer.rs`
- Test: `app/src-tauri/src/sessions/pricing.rs`
- Test: `app/src-tauri/src/storage/repository.rs`

- [ ] **Step 1: Write failing lineage aggregation tests**

Import `root.jsonl` and `child.jsonl`, then query summaries:

```rust
#[test]
fn rolls_child_usage_into_one_top_level_session_row() {
    let repository = repository_with_root_and_child();
    let sessions = repository.local_session_view(2_000).unwrap().sessions;

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, "root-1");
    assert_eq!(sessions[0].child_session_count, 1);
    assert_eq!(sessions[0].tokens.input_tokens, 1_300);
    assert_eq!(sessions[0].tokens.output_tokens, 280);
}

#[test]
fn an_orphan_is_visible_until_its_parent_arrives() {
    let repository = repository_with_child_only();
    let before = repository.local_session_view(1_000).unwrap();
    assert_eq!(before.sessions[0].session_id, "child-1");

    import_root(&repository);
    let after = repository.local_session_view(2_000).unwrap();
    assert_eq!(after.sessions.len(), 1);
    assert_eq!(after.sessions[0].session_id, "root-1");
}
```

- [ ] **Step 2: Run aggregation tests and verify RED**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml rolls_child_usage_into_one_top_level_session_row
```

Expected: FAIL because `local_session_view` does not exist.

- [ ] **Step 3: Implement root resolution and summary query**

Load session metadata and events, resolve parent chains in memory with a visited set, and treat a
cycle as an orphan instead of recursing forever. Group events by resolved root session ID, count
distinct child session IDs, sum each token field, select the model with the most total tokens, and
sort by `last_active_at DESC`.

Do not subtract `total_token_usage`: each event row already represents `last_token_usage`, so
inherited cumulative counters never enter the event table.

- [ ] **Step 4: Write failing pricing tests**

```rust
#[test]
fn prices_input_cached_input_and_output_separately() {
    let prices = PriceCatalog::built_in();
    let tokens = TokenBreakdown {
        input_tokens: 1_000_000,
        cached_input_tokens: 500_000,
        output_tokens: 200_000,
        reasoning_output_tokens: 50_000,
    };
    let cost = prices.estimate("gpt-5.6-codex", 1_785_283_200, &tokens).unwrap();
    assert!(cost > 0.0);
}

#[test]
fn unknown_models_return_no_cost() {
    assert_eq!(
        PriceCatalog::built_in().estimate(
            "future-unknown-model",
            1_785_283_200,
            &TokenBreakdown::default()
        ),
        None
    );
}
```

- [ ] **Step 5: Run pricing tests and verify RED**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml sessions::pricing::tests
```

Expected: FAIL because `PriceCatalog` does not exist.

- [ ] **Step 6: Implement versioned pricing**

Implement a static catalog with explicit aliases and effective dates. Store prices in
`model_price_versions` on startup. Compute:

```rust
let uncached_input = (tokens.input_tokens - tokens.cached_input_tokens).max(0);
let usd = uncached_input as f64 / 1_000_000.0 * price.input_per_million
    + tokens.cached_input_tokens as f64 / 1_000_000.0 * price.cached_input_per_million
    + tokens.output_tokens as f64 / 1_000_000.0 * price.output_per_million;
```

Reasoning output is a subset of output for the Codex event format and must not be charged a
second time. Keep current public prices in one constant table with source URL and catalog version
comments; updating values requires a separate tested catalog commit.

- [ ] **Step 7: Run aggregation and pricing tests**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml sessions::
cargo test --manifest-path app/src-tauri/Cargo.toml storage::
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add app/src-tauri/src/sessions/pricing.rs app/src-tauri/src/sessions/importer.rs app/src-tauri/src/storage/repository.rs
git commit -m "feat: aggregate local session usage"
```

## Task 5: Run the Session Service and Merge Its Snapshot

**Files:**
- Create: `app/src-tauri/src/sessions/service.rs`
- Modify: `app/src-tauri/src/sessions/mod.rs`
- Modify: `app/src-tauri/src/domain/dashboard.rs`
- Modify: `app/src-tauri/src/monitor.rs`
- Modify: `app/src-tauri/src/commands.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Test: `app/src-tauri/src/sessions/service.rs`
- Test: `app/src-tauri/src/monitor.rs`

- [ ] **Step 1: Write a failing service test**

Use a paused Tokio clock and temporary Codex home:

```rust
#[tokio::test]
async fn rescan_publishes_sessions_without_waiting_for_account_refresh() {
    let repository = Arc::new(AccountRepository::open_in_memory().unwrap());
    let service = SessionService::new(repository, fixture_codex_home());
    let mut updates = service.subscribe();

    service.rescan().await.unwrap();
    updates.changed().await.unwrap();

    assert_eq!(updates.borrow().sessions.len(), 1);
}
```

- [ ] **Step 2: Run the service test and verify RED**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml sessions::service::tests
```

Expected: FAIL because `SessionService` does not exist.

- [ ] **Step 3: Implement `SessionService`**

Expose:

```rust
pub struct SessionService {
    importer: Mutex<SessionImporter>,
    repository: Arc<AccountRepository>,
    snapshot: watch::Sender<LocalSessionView>,
    scan_lock: Mutex<()>,
}

impl SessionService {
    pub fn subscribe(&self) -> watch::Receiver<LocalSessionView>;
    pub fn snapshot(&self) -> LocalSessionView;
    pub async fn rescan(&self) -> Result<LocalSessionView, AppError>;
    pub async fn run(self: Arc<Self>, shutdown: watch::Receiver<bool>);
}
```

`run` performs an initial scan, uses `notify::recommended_watcher` to trigger a debounced scan
after changes below the two session directories, and performs a reconciliation scan every
10 minutes. A failed scan updates `diagnostics.last_error` while retaining the last successful
session rows.

- [ ] **Step 4: Add sessions to `DashboardSnapshot`**

Replace `session_details_available` with:

```rust
pub local_sessions: LocalSessionView,
```

Update `Default`, all monitor tests, and `AccountMonitor` to hold an
`Arc<SessionService>`. `refresh()` uses `session_service.snapshot()`; a session watch task updates
only `local_sessions` in the current dashboard snapshot and emits the new snapshot without
waiting for account I/O.

- [ ] **Step 5: Add a manual rescan command**

Add and register:

```rust
#[tauri::command]
pub async fn rescan_sessions(
    state: tauri::State<'_, AppState>,
) -> Result<LocalSessionView, String> {
    state.sessions.rescan().await.map_err(|error| error.to_string())
}
```

Extend `AppState` with `sessions: Arc<SessionService>`.

- [ ] **Step 6: Run service and monitor tests**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml sessions::service
cargo test --manifest-path app/src-tauri/Cargo.toml monitor::
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/src/sessions app/src-tauri/src/domain/dashboard.rs app/src-tauri/src/monitor.rs app/src-tauri/src/commands.rs app/src-tauri/src/lib.rs
git commit -m "feat: publish live local session summaries"
```

## Task 6: Render Real Session Details in React

**Files:**
- Modify: `app/src/types/dashboard.ts`
- Modify: `app/src/lib/backend.ts`
- Modify: `app/src/components/SessionDetails.tsx`
- Modify: `app/src/components/Dashboard.tsx`
- Modify: `app/src/components/Dashboard.test.tsx`
- Modify: `app/src/styles/app.css`

- [ ] **Step 1: Replace the phase-one empty-state test with failing session tests**

Add a `localSessions` fixture and tests:

```tsx
test("renders one row per top-level session with child usage already included", () => {
  renderDashboard({
    ...snapshot,
    localSessions: {
      diagnostics: {
        scannedFiles: 4,
        skippedLines: 0,
        lastImportedAt: 1_785_330_000,
        lastError: null,
      },
      sessions: [{
        sessionId: "root-1",
        title: "example-project · 7月29日",
        projectPath: "/tmp/example-project",
        lastActiveAt: 1_785_330_000,
        primaryModel: "gpt-5.6-codex",
        tokens: {
          inputTokens: 1_300,
          cachedInputTokens: 500,
          outputTokens: 280,
          reasoningOutputTokens: 70,
        },
        equivalentCostUsd: 0.02,
        childSessionCount: 1,
      }],
    },
  });

  expect(screen.getByRole("row", { name: "example-project · 7月29日" })).toBeVisible();
  expect(screen.getAllByRole("row")).toHaveLength(1);
  expect(screen.queryByText("根会话")).not.toBeInTheDocument();
});

test("distinguishes import failure from a genuinely empty local history", () => {
  renderDashboard({
    ...snapshot,
    localSessions: {
      sessions: [],
      diagnostics: {
        scannedFiles: 0,
        skippedLines: 0,
        lastImportedAt: null,
        lastError: "permission denied",
      },
    },
  });
  expect(screen.getByText("无法读取本机会话记录")).toBeVisible();
  expect(screen.getByRole("button", { name: "重新扫描" })).toBeVisible();
});
```

- [ ] **Step 2: Run the React test and verify RED**

Run:

```bash
cd app
pnpm test -- src/components/Dashboard.test.tsx
```

Expected: FAIL because `localSessions` and the real table do not exist.

- [ ] **Step 3: Add frontend types and preview data**

Mirror the Rust camel-case models exactly in `types/dashboard.ts`, replace
`sessionDetailsAvailable`, and add two realistic top-level rows to `previewSnapshot`.

Add:

```ts
rescanSessions: () =>
  isWebPreview()
    ? Promise.resolve(previewSnapshot.localSessions)
    : invoke<LocalSessionView>("rescan_sessions"),
```

- [ ] **Step 4: Implement the expandable session table**

`SessionDetails` receives `view: LocalSessionView` and `onRescan`. Use a semantic table with
one `<tr>` per top-level session. The row button controls a detail row showing token categories.
Format Token counts with `Intl.NumberFormat("zh-CN", { notation: "compact" })`; show cost as
`≈ US$0.02`, or `价格未知`.

The visible columns are 会话, 项目, 模型, Token, 等效费用, 最后活动. Child count appears as a
small `含 1 个子任务` badge, never as an independent row.

- [ ] **Step 5: Style table, expansion, empty, and error states**

Add `.session-table`, `.session-row`, `.session-breakdown`, `.session-child-badge`,
`.session-error`, and responsive rules. At widths below 760 px, hide project and model columns
but keep them in the expanded detail.

- [ ] **Step 6: Run the targeted test**

Run:

```bash
cd app
pnpm test -- src/components/Dashboard.test.tsx
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add app/src/types/dashboard.ts app/src/lib/backend.ts app/src/components/SessionDetails.tsx app/src/components/Dashboard.tsx app/src/components/Dashboard.test.tsx app/src/styles/app.css
git commit -m "feat: show aggregated local session details"
```

## Task 7: Correct Quota Progress Semantics

**Files:**
- Modify: `app/src/components/QuotaCard.tsx`
- Modify: `app/src/components/Dashboard.test.tsx`
- Modify: `app/src/styles/app.css`

- [ ] **Step 1: Write a failing progress semantics test**

```tsx
test("fills the quota bar left to right using consumed percentage", () => {
  const { container } = renderDashboard({
    ...snapshot,
    primaryQuota: {
      ...snapshot.primaryQuota!,
      usedPercent: 25,
      remainingPercent: 75,
    },
  });

  const fill = container.querySelector<HTMLElement>(".quota-track > span");
  expect(fill).toHaveStyle({ width: "25%" });
  expect(screen.getByText("已消耗 25%")).toBeVisible();
  expect(screen.getByText("剩余 75%")).toBeVisible();
});
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cd app
pnpm test -- src/components/Dashboard.test.tsx -t "fills the quota bar"
```

Expected: FAIL because the exact “已消耗” and remaining scale labels are absent.

- [ ] **Step 3: Implement the approved horizontal bar**

Clamp the width:

```tsx
const usedPercent = Math.min(100, Math.max(0, quota.usedPercent));
```

Keep the large value as remaining, set the fill width to `usedPercent`, set
`aria-valuenow={usedPercent}`, and render:

```tsx
<div className="quota-scale">
  <span>已消耗 {Math.round(usedPercent)}%</span>
  <span>剩余 {Math.round(quota.remainingPercent)}%</span>
</div>
```

Add `role="progressbar"`, `aria-valuemin={0}`, and `aria-valuemax={100}`. Apply warning and
critical classes at 70% and 90% used.

- [ ] **Step 4: Run the targeted and full dashboard tests**

Run:

```bash
cd app
pnpm test -- src/components/Dashboard.test.tsx
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src/components/QuotaCard.tsx app/src/components/Dashboard.test.tsx app/src/styles/app.css
git commit -m "fix: make quota bar represent consumed usage"
```

## Task 8: Expand the Native Menu Bar

**Files:**
- Modify: `app/src-tauri/src/tray.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Modify: `app/src-tauri/src/commands.rs`
- Modify: `app/src/lib/backend.ts`
- Modify: `app/src/components/Dashboard.tsx`
- Test: `app/src-tauri/src/tray.rs`
- Test: `app/src/components/Dashboard.test.tsx`

- [ ] **Step 1: Write failing pure formatter tests**

```rust
#[test]
fn formats_complete_menu_status_from_one_snapshot() {
    let state = menu_state(&snapshot_with_quota_and_sessions());
    assert_eq!(state.quota, "剩余 75% · 已消耗 25%");
    assert_eq!(state.progress, "消耗  ███░░░░░░░  25%");
    assert_eq!(state.reset, "重置  8 月 5 日 12:20");
    assert_eq!(state.forecast, "预测  重置前不会耗尽");
    assert_eq!(state.sessions, "本机会话  3 个");
}

#[test]
fn formats_missing_account_data_without_hiding_local_sessions() {
    let state = menu_state(&snapshot_with_local_sessions_only());
    assert_eq!(state.quota, "等待账号额度");
    assert_eq!(state.sessions, "本机会话  3 个");
}
```

The ten-segment text bar is intentional: Tauri’s native `MenuItem` does not expose a custom
progress view, so this preserves native menu behavior without introducing a fragile custom
popover window.

- [ ] **Step 2: Run tray tests and verify RED**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml tray::tests
```

Expected: FAIL because `menu_state` does not exist.

- [ ] **Step 3: Refactor tray construction into `TrayController`**

Store handles for dynamic items:

```rust
pub struct TrayController {
    tray: TrayIcon,
    quota: MenuItem,
    progress: MenuItem,
    reset: MenuItem,
    forecast: MenuItem,
    today_tokens: MenuItem,
    sessions: MenuItem,
    updated: MenuItem,
    refresh: MenuItem,
}

impl TrayController {
    pub fn apply(&self, snapshot: &DashboardSnapshot) -> tauri::Result<()>;
}
```

Build the menu in this order: status items, separator, summary items, separator, `打开面板`,
`查看最近会话`, `立即刷新`, separator, `设置…`, `退出 Codex Monitor`. Disable all status
and summary items. Use the existing snapshot subscription to call `apply`.

- [ ] **Step 4: Implement actions**

- `show`: show, center only when first opened, and focus the main window.
- `sessions`: show/focus the window and emit `dashboard://focus-section` with `"sessions"`.
- `refresh`: start account refresh and session rescan concurrently; disable the item until both
  futures finish, then re-enable it.
- `settings`: show/focus the window and emit `dashboard://focus-section` with `"settings"`.
- `quit`: send shutdown and exit.

Register a frontend listener for `dashboard://focus-section`. For `sessions`, call
`document.getElementById("sessions-heading")?.scrollIntoView({ behavior: "smooth" })` and add
a temporary `.section-focused` class. For `settings`, focus the existing settings navigation
button; until the settings screen is implemented, the menu label is `设置（稍后）` and disabled,
rather than presenting a non-working action.

- [ ] **Step 5: Write and run the focus test**

```tsx
test("marks the session section as a focus target", () => {
  renderDashboard(snapshot);
  expect(screen.getByRole("heading", { name: "会话详情" }).closest("section"))
    .toHaveAttribute("data-section", "sessions");
});
```

Run:

```bash
cd app
pnpm test -- src/components/Dashboard.test.tsx
cargo test --manifest-path src-tauri/Cargo.toml tray::tests
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/tray.rs app/src-tauri/src/lib.rs app/src-tauri/src/commands.rs app/src/lib/backend.ts app/src/components/Dashboard.tsx app/src/components/Dashboard.test.tsx
git commit -m "feat: expand menu bar status and actions"
```

## Task 9: Replace the App and Tray Identity

**Files:**
- Create: `app/brand/monitor-window.svg`
- Create: `app/brand/tray-template.svg`
- Create: `app/src/components/BrandMark.tsx`
- Modify: `app/src/components/AppSidebar.tsx`
- Modify: `app/src-tauri/src/tray.rs`
- Replace: `app/src-tauri/icons/*`
- Replace: `app/public/favicon.svg`
- Test: `app/src/components/Dashboard.test.tsx`

- [ ] **Step 1: Write a failing brand test**

```tsx
test("uses the remaining-window brand instead of a waveform", () => {
  renderDashboard();
  expect(screen.getByLabelText("Codex Monitor 余量窗口")).toBeVisible();
  expect(screen.queryByTestId("waveform-brand")).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cd app
pnpm test -- src/components/Dashboard.test.tsx -t "remaining-window brand"
```

Expected: FAIL because the sidebar still uses Phosphor `Waveform`.

- [ ] **Step 3: Create the vector masters and React mark**

`monitor-window.svg` uses:

- a 1024×1024 rounded-square canvas;
- deep green gradient `#16231F` to `#244B3D`;
- an off-white rounded window outline;
- a mint `#70D5A5` rising fill shape;
- no sound-wave bars, letters, or percentage text.

`tray-template.svg` uses the same window outline and a single rising fill silhouette in solid
black on transparency at 44×44. `BrandMark.tsx` inlines the same geometry with
`aria-label="Codex Monitor 余量窗口"`.

- [ ] **Step 4: Generate platform icons**

Run from `app`:

```bash
pnpm tauri icon brand/monitor-window.svg
```

Expected: Tauri regenerates `.icns`, `.ico`, PNG, and Windows Store assets in
`src-tauri/icons`.

Rasterize the tray derivative with macOS Quick Look only for the generated artifact:

```bash
qlmanage -t -s 44 -o /tmp/codex-monitor-tray brand/tray-template.svg
sips -s format png /tmp/codex-monitor-tray/tray-template.svg.png --out src-tauri/icons/trayTemplate.png
```

Keep `brand/tray-template.svg` as the editable source. Change `tray.rs` to include
`trayTemplate.png` with `icon_as_template(true)`.

- [ ] **Step 5: Replace sidebar and favicon**

Use `<BrandMark />` in `AppSidebar`, copy the vector geometry into `public/favicon.svg`, and
remove the `Waveform` import.

- [ ] **Step 6: Run brand test and inspect generated assets**

Run:

```bash
cd app
pnpm test -- src/components/Dashboard.test.tsx -t "remaining-window brand"
file src-tauri/icons/icon.icns src-tauri/icons/icon.ico src-tauri/icons/trayTemplate.png
```

Expected: test PASS; `file` identifies valid ICNS, Windows icon, and PNG files.

- [ ] **Step 7: Commit**

```bash
git add app/brand app/src/components/BrandMark.tsx app/src/components/AppSidebar.tsx app/public/favicon.svg app/src-tauri/icons app/src-tauri/src/tray.rs app/src/components/Dashboard.test.tsx
git commit -m "feat: introduce remaining-window identity"
```

## Task 10: End-to-End Verification and Documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-07-29-session-tray-brand-upgrade.md`

- [ ] **Step 1: Document data scope and privacy**

Add a README section stating:

- account quota and account daily Token cover all devices;
- session details are local to the current device;
- child agents are rolled into their top-level session;
- prompts, responses, tool output, and reasoning are not stored in SQLite;
- equivalent cost is an API-price estimate, not a bill.

- [ ] **Step 2: Run Rust formatting and lint**

Run:

```bash
cargo fmt --manifest-path app/src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path app/src-tauri/Cargo.toml --all-targets -- -D warnings
```

Expected: both commands exit 0 with no warnings.

- [ ] **Step 3: Run all Rust tests**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml
```

Expected: all tests PASS with zero failures.

- [ ] **Step 4: Run all frontend checks**

Run:

```bash
cd app
pnpm test
pnpm lint
pnpm build
```

Expected: Vitest reports zero failures, oxlint exits 0, and Vite produces `dist/`.

- [ ] **Step 5: Verify against a copy of real local data**

Create a temporary Codex home and copy three representative files into it: one top-level session,
one child session, and one archived session. Launch with `CODEX_HOME` pointing to the temporary
directory, then verify:

1. exactly one row per top-level session;
2. child usage is included once;
3. a second rescan does not change totals;
4. the UI says “本机记录”;
5. account quota remains unchanged and says “所有设备”.

Do not run tests directly against or modify the user’s real Codex session directory.

- [ ] **Step 6: Verify the native macOS app**

Run:

```bash
cd app
pnpm tauri dev
```

Verify:

- tray title shows remaining percentage;
- tray menu shows quota, textual consumed bar, reset, forecast, Token, sessions, refresh, and quit;
- “查看最近会话” opens and focuses the session section;
- the colored Dock icon and monochrome menu icon match the approved remaining-window design;
- quota fill is 25% wide when the UI reports 25% consumed.

- [ ] **Step 7: Mark checklist items complete and commit**

Update this plan’s checkboxes only for steps backed by fresh command or manual evidence, then:

```bash
git add README.md docs/superpowers/plans/2026-07-29-session-tray-brand-upgrade.md
git commit -m "docs: document local session monitoring"
```
