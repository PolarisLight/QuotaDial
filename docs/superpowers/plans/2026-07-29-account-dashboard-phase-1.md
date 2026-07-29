# Account Dashboard Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a packaged macOS-first Tauri application that reads authoritative Codex account quota and daily Token activity, stores observations locally, forecasts quota exhaustion, and renders the approved dashboard plus menu-bar status.

**Architecture:** A Rust core owns the Codex app-server child process, JSONL protocol, SQLite persistence, forecast calculation, refresh loop, and Tauri commands/events. React consumes a single `DashboardSnapshot` read model and contains no quota inference logic. This phase deliberately ends at an account-level vertical slice; local session JSONL accounting, per-session cost calculation, notifications, and Windows device validation each receive a separate follow-up plan.

**Tech Stack:** Tauri 2, Rust stable, Tokio, Serde, Rusqlite with bundled SQLite, React, TypeScript, Vite, Vitest, Testing Library, native CSS, Phosphor icons.

---

## Delivery Boundary

This plan produces working software with:

- Real `account/rateLimits/read` data, including every `rateLimitsByLimitId` bucket.
- Real `account/usage/read` summary and daily buckets when supported.
- SQLite observation history.
- A reset-aware, multi-device quota exhaustion forecast.
- Approved macOS-style dashboard with loading, empty, stale, and error states.
- A macOS menu-bar item that exposes remaining quota and opens the dashboard.

This plan does not implement:

- Local Codex JSONL import.
- Parent/child session aggregation.
- Equivalent API cost.
- Threshold notifications.
- Windows packaging validation.

The UI must show a clear “Session details will appear after local accounting is enabled” empty
state instead of fabricated rows or costs.

## File Map

```text
app/
  package.json                         Frontend scripts and dependencies
  vite.config.ts                       Vite and Vitest configuration
  src/
    main.tsx                           React entry
    App.tsx                            Route-free application shell
    styles/tokens.css                  Light/dark/accessibility design tokens
    styles/app.css                     Dashboard layout and responsive rules
    types/dashboard.ts                 Rust-to-TypeScript read-model contract
    lib/backend.ts                     Typed Tauri invoke/event adapter
    hooks/useDashboard.ts              Loading, refresh, event, stale state
    components/AppSidebar.tsx          Sidebar navigation and connection state
    components/QuotaCard.tsx           Primary account quota card
    components/UsageForecastPanel.tsx  Account Token trend and quota forecast
    components/SessionDetails.tsx      Honest phase-1 empty state
    components/Dashboard.tsx           Screen composition
    test/setup.ts                      DOM matcher setup
    test/runtime.test.ts               Vitest/jsdom scaffold smoke test
    components/*.test.tsx              Component behavior tests
  src-tauri/
    Cargo.toml                         Rust dependencies and Tauri features
    tauri.conf.json                    Window and bundle configuration
    capabilities/default.json          Minimum Tauri permissions
    migrations/001_account.sql         Account observation schema
    src/
      lib.rs                           Tauri composition root
      main.rs                          Binary entry
      error.rs                         Serializable application error
      domain/
        mod.rs
        account.rs                     App-server response/domain types
        dashboard.rs                   Frontend read model
      app_server/
        mod.rs
        protocol.rs                    JSONL request/response routing
        process.rs                     `codex app-server` child lifecycle
      storage/
        mod.rs
        migrations.rs                  Transactional migration runner
        repository.rs                  Observation reads/writes
      forecast.rs                      Reset-aware burn-rate estimator
      monitor.rs                       Refresh loop and snapshot assembly
      commands.rs                      Tauri commands
      tray.rs                          Menu-bar integration
```

## Reference Contracts

- Tauri project creation: https://v2.tauri.app/start/create-project/
- Tauri testing: https://v2.tauri.app/develop/tests/
- Tauri tray API: https://v2.tauri.app/reference/javascript/api/namespacetray/
- Codex app-server protocol: https://learn.chatgpt.com/docs/app-server
- Stable calls used here:
  - `initialize`, followed by `initialized`
  - `account/rateLimits/read`
  - `account/rateLimits/updated`
  - `account/usage/read`

### Task 1: Scaffold the Tauri React Application

**Files:**
- Create: `app/` from the official Vite React TypeScript template
- Create: `app/src-tauri/` from `tauri init`
- Modify: `app/package.json`
- Modify: `app/vite.config.ts`
- Modify: `app/src-tauri/Cargo.toml`
- Modify: `app/src-tauri/tauri.conf.json`
- Create: `app/src/test/setup.ts`
- Create: `app/src/test/runtime.test.ts`

- [ ] **Step 1: Create the React project**

Run from the repository root:

```bash
pnpm create vite@latest app --template react-ts
cd app
pnpm install
```

Expected: Vite creates `app/src/main.tsx`; `pnpm install` exits `0`.

- [ ] **Step 2: Add frontend and Tauri dependencies**

Run:

```bash
pnpm add @tauri-apps/api @phosphor-icons/react
pnpm add -D @tauri-apps/cli vitest jsdom @testing-library/react @testing-library/jest-dom
```

Expected: dependencies appear in `app/package.json`.

- [ ] **Step 3: Initialize Tauri**

Run:

```bash
pnpm tauri init
```

Use these prompt answers:

```text
App name: Codex Monitor
Window title: Codex Monitor
Web assets location: ../dist
Dev server URL: http://localhost:5173
Frontend dev command: pnpm dev
Frontend build command: pnpm build
```

Expected: `app/src-tauri/tauri.conf.json` and Rust source files are created.

- [ ] **Step 4: Add Rust dependencies**

Run from `app/src-tauri`:

```bash
cargo add tokio --features process,io-util,macros,rt-multi-thread,sync,time,test-util
cargo add serde --features derive
cargo add serde_json thiserror chrono
cargo add rusqlite --features bundled
cargo add uuid --features v4
```

Expected: every dependency is present once in `Cargo.toml`.

- [ ] **Step 5: Configure Vitest**

Modify `app/vite.config.ts`:

```ts
/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    clearMocks: true,
  },
});
```

Create `app/src/test/setup.ts`:

```ts
import "@testing-library/jest-dom/vitest";
```

Create `app/src/test/runtime.test.ts`:

```ts
import { expect, test } from "vitest";

test("boots the jsdom test runtime", () => {
  expect(document.body).toBeDefined();
});
```

Add scripts to `app/package.json`:

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "test": "vitest run",
    "test:watch": "vitest",
    "tauri": "tauri"
  }
}
```

- [ ] **Step 6: Configure the desktop window**

Set the main window in `app/src-tauri/tauri.conf.json`:

```json
{
  "productName": "Codex Monitor",
  "version": "0.1.0",
  "identifier": "com.codexmonitor.app",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [{
      "label": "main",
      "title": "Codex Monitor",
      "width": 1220,
      "height": 820,
      "minWidth": 900,
      "minHeight": 640,
      "center": true,
      "transparent": false
    }]
  },
  "bundle": { "active": true, "targets": "all" }
}
```

- [ ] **Step 7: Verify the clean scaffold**

Run:

```bash
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all three commands exit `0`; Vitest reports no failing tests.

- [ ] **Step 8: Commit**

```bash
git add app
git commit -m "build: scaffold Tauri React application"
```

### Task 2: Define and Parse Account Responses

**Files:**
- Create: `app/src-tauri/src/domain/mod.rs`
- Create: `app/src-tauri/src/domain/account.rs`
- Create: `app/src-tauri/src/domain/dashboard.rs`
- Test: inline Rust tests in `account.rs`

- [ ] **Step 1: Write failing deserialization tests**

Create test fixtures inside `domain/account.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multi_bucket_rate_limits() {
        let json = r#"{
          "rateLimits": {
            "limitId": "codex",
            "limitName": null,
            "primary": {
              "usedPercent": 18,
              "windowDurationMins": 10080,
              "resetsAt": 1785903626
            },
            "secondary": null,
            "rateLimitReachedType": null
          },
          "rateLimitsByLimitId": {
            "codex": {
              "limitId": "codex",
              "limitName": null,
              "primary": {
                "usedPercent": 18,
                "windowDurationMins": 10080,
                "resetsAt": 1785903626
              },
              "secondary": null,
              "rateLimitReachedType": null
            }
          },
          "rateLimitResetCredits": { "availableCount": 1, "credits": null }
        }"#;
        let parsed: RateLimitsResult = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.by_limit_id["codex"].primary.as_ref().unwrap().used_percent, 18.0);
        assert_eq!(parsed.reset_credits.unwrap().available_count, 1);
    }

    #[test]
    fn parses_nullable_account_usage() {
        let json = r#"{
          "summary": { "lifetimeTokens": null, "peakDailyTokens": 45678 },
          "dailyUsageBuckets": [
            { "startDate": "2026-07-28", "tokens": 12345 }
          ]
        }"#;
        let parsed: AccountUsageResult = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.summary.unwrap().lifetime_tokens, None);
        assert_eq!(parsed.daily_usage_buckets.unwrap()[0].tokens, 12345);
    }
}
```

- [ ] **Step 2: Run tests and verify failure**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml domain::account::tests
```

Expected: FAIL because `RateLimitsResult` and `AccountUsageResult` do not exist.

- [ ] **Step 3: Implement the exact wire types**

Create `domain/account.rs`:

```rust
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RateWindow {
    pub used_percent: f64,
    pub window_duration_mins: i64,
    pub resets_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitBucket {
    pub limit_id: String,
    pub limit_name: Option<String>,
    pub primary: Option<RateWindow>,
    pub secondary: Option<RateWindow>,
    pub rate_limit_reached_type: Option<String>,
    pub plan_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResetCredits {
    pub available_count: i64,
    pub credits: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitsResult {
    pub rate_limits: Option<RateLimitBucket>,
    #[serde(default)]
    pub rate_limits_by_limit_id: BTreeMap<String, RateLimitBucket>,
    pub rate_limit_reset_credits: Option<ResetCredits>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub lifetime_tokens: Option<i64>,
    pub peak_daily_tokens: Option<i64>,
    pub longest_running_turn_sec: Option<i64>,
    pub current_streak_days: Option<i64>,
    pub longest_streak_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsageBucket {
    pub start_date: String,
    pub tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountUsageResult {
    pub summary: Option<UsageSummary>,
    pub daily_usage_buckets: Option<Vec<DailyUsageBucket>>,
}
```

Export modules from `domain/mod.rs`:

```rust
pub mod account;
pub mod dashboard;
```

- [ ] **Step 4: Add stable bucket normalization**

Add:

```rust
impl RateLimitsResult {
    pub fn buckets(&self) -> BTreeMap<String, RateLimitBucket> {
        if !self.rate_limits_by_limit_id.is_empty() {
            return self.rate_limits_by_limit_id.clone();
        }
        self.rate_limits
            .clone()
            .map(|bucket| BTreeMap::from([(bucket.limit_id.clone(), bucket)]))
            .unwrap_or_default()
    }
}
```

Add a test asserting the fallback `rateLimits` bucket is returned when
`rateLimitsByLimitId` is absent.

- [ ] **Step 5: Run tests**

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml domain::account::tests
```

Expected: all account-domain tests PASS.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/domain
git commit -m "feat: model Codex account usage responses"
```

### Task 3: Implement the JSONL App-Server Protocol

**Files:**
- Create: `app/src-tauri/src/error.rs`
- Create: `app/src-tauri/src/app_server/mod.rs`
- Create: `app/src-tauri/src/app_server/protocol.rs`
- Create: `app/src-tauri/src/app_server/process.rs`
- Test: inline tests in `protocol.rs`

- [ ] **Step 1: Write protocol tests**

Create tests using `tokio::io::duplex`:

```rust
#[tokio::test]
async fn sends_initialize_before_account_calls() {
    let (client, server) = tokio::io::duplex(4096);
    let (server_read, server_write) = tokio::io::split(server);
    let peer = RpcPeer::from_stream(client);
    let server_task = tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let mut lines = BufReader::new(server_read).lines();
        assert_eq!(
            lines.next_line().await.unwrap().unwrap(),
            r#"{"method":"initialize","id":1,"params":{"clientInfo":{"name":"codex_monitor","title":"Codex Monitor","version":"0.1.0"}}}"#
        );
        server_write
            .write_all(b"{\"id\":1,\"result\":{}}\n")
            .await
            .unwrap();
        assert_eq!(
            lines.next_line().await.unwrap().unwrap(),
            r#"{"method":"initialized","params":{}}"#
        );
        assert_eq!(
            lines.next_line().await.unwrap().unwrap(),
            r#"{"method":"account/rateLimits/read","id":2}"#
        );
        server_write
            .write_all(b"{\"id\":2,\"result\":{\"rateLimits\":{\"limitId\":\"codex\"}}}\n")
            .await
            .unwrap();
    });

    peer.initialize().await.unwrap();
    let value = peer.request("account/rateLimits/read", None).await.unwrap();

    assert_eq!(value["rateLimits"]["limitId"], "codex");
    server_task.await.unwrap();
}
```

The fake server must assert these exact client messages in order:

```json
{"method":"initialize","id":1,"params":{"clientInfo":{"name":"codex_monitor","title":"Codex Monitor","version":"0.1.0"}}}
{"method":"initialized","params":{}}
{"method":"account/rateLimits/read","id":2}
```

- [ ] **Step 2: Verify the test fails**

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml app_server::protocol::tests
```

Expected: FAIL because `RpcPeer` is undefined.

- [ ] **Step 3: Implement request routing**

In `protocol.rs`, define:

```rust
pub struct RpcNotification {
    pub method: String,
    pub params: serde_json::Value,
}

pub struct RpcPeer {
    writer: tokio::sync::Mutex<Box<dyn tokio::io::AsyncWrite + Unpin + Send>>,
    pending: std::sync::Arc<tokio::sync::Mutex<
        std::collections::HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>
    >>,
    next_id: std::sync::atomic::AtomicU64,
    notifications: tokio::sync::broadcast::Sender<RpcNotification>,
}
```

Implement:

```rust
impl RpcPeer {
    pub fn new<R, W>(reader: R, writer: W) -> Self
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
        W: tokio::io::AsyncWrite + Unpin + Send + 'static;
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static;
    pub async fn initialize(&self) -> Result<(), AppError>;
    pub async fn request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, AppError>;
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<RpcNotification>;
}
```

Rules:

- Write one compact JSON object followed by `\n`.
- `new` starts the stdout reader task; `from_stream` splits a duplex stream and delegates
  to `new`.
- Omit the `"jsonrpc":"2.0"` field.
- Route objects with `id` to the matching oneshot sender.
- Route objects with `method` and no `id` to the broadcast channel.
- Convert an app-server `error` object into `AppError::Rpc`.
- Return `AppError::Disconnected` to every pending request when stdout closes.

- [ ] **Step 4: Implement the process adapter**

In `process.rs`:

```rust
pub async fn spawn_codex_app_server() -> Result<(RpcPeer, tokio::process::Child), AppError> {
    let mut child = tokio::process::Command::new("codex")
        .args(["app-server", "--listen", "stdio://"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(AppError::Spawn)?;

    let stdin = child.stdin.take().ok_or(AppError::MissingPipe("stdin"))?;
    let stdout = child.stdout.take().ok_or(AppError::MissingPipe("stdout"))?;
    let peer = RpcPeer::new(stdout, stdin);
    peer.initialize().await?;
    Ok((peer, child))
}
```

`AppError` must derive `thiserror::Error` and contain `Spawn`, `MissingPipe`,
`Io`, `Json`, `Rpc`, `Disconnected`, `Database`, and `Unavailable` variants.

- [ ] **Step 5: Run protocol and full Rust tests**

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml app_server::protocol::tests
cargo test --manifest-path app/src-tauri/Cargo.toml
```

Expected: both commands PASS.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/app_server app/src-tauri/src/error.rs
git commit -m "feat: connect to Codex app-server"
```

### Task 4: Persist Account Observations

**Files:**
- Create: `app/src-tauri/migrations/001_account.sql`
- Create: `app/src-tauri/src/storage/mod.rs`
- Create: `app/src-tauri/src/storage/migrations.rs`
- Create: `app/src-tauri/src/storage/repository.rs`
- Test: inline repository tests using an in-memory database

- [ ] **Step 1: Write the migration**

Create `001_account.sql`:

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS account_rate_limit_observations (
  id INTEGER PRIMARY KEY,
  observed_at INTEGER NOT NULL,
  limit_id TEXT NOT NULL,
  window_kind TEXT NOT NULL CHECK (window_kind IN ('primary', 'secondary')),
  used_percent REAL NOT NULL CHECK (used_percent >= 0 AND used_percent <= 100),
  window_duration_mins INTEGER NOT NULL,
  resets_at INTEGER NOT NULL,
  plan_type TEXT,
  payload_json TEXT NOT NULL,
  UNIQUE(observed_at, limit_id, window_kind)
);

CREATE INDEX IF NOT EXISTS idx_rate_observations_lookup
ON account_rate_limit_observations(limit_id, window_kind, observed_at);

CREATE TABLE IF NOT EXISTS account_usage_observations (
  id INTEGER PRIMARY KEY,
  observed_at INTEGER NOT NULL UNIQUE,
  lifetime_tokens INTEGER,
  peak_daily_tokens INTEGER,
  daily_buckets_json TEXT,
  payload_json TEXT NOT NULL
);
```

- [ ] **Step 2: Write repository tests**

Tests must verify:

```rust
#[test]
fn inserting_same_observation_twice_is_idempotent() { /* row count remains 1 */ }

#[test]
fn loads_only_current_forecast_segment() {
    /* a changed resets_at value excludes older rows */
}

#[test]
fn stores_nullable_account_usage_fields() { /* null remains null */ }
```

- [ ] **Step 3: Verify failure**

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml storage::repository::tests
```

Expected: FAIL because repository functions do not exist.

- [ ] **Step 4: Implement repository APIs**

Expose:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct RateObservation {
    pub observed_at: i64,
    pub limit_id: String,
    pub window_kind: String,
    pub used_percent: f64,
    pub window_duration_mins: i64,
    pub resets_at: i64,
}

pub struct AccountRepository {
    connection: std::sync::Mutex<rusqlite::Connection>,
}

impl AccountRepository {
    pub fn open(path: &std::path::Path) -> Result<Self, AppError>;
    pub fn open_in_memory() -> Result<Self, AppError>;
    pub fn insert_rate_limits(
        &self,
        observed_at: i64,
        value: &RateLimitsResult,
        raw: &serde_json::Value,
    ) -> Result<(), AppError>;
    pub fn insert_account_usage(
        &self,
        observed_at: i64,
        value: &AccountUsageResult,
        raw: &serde_json::Value,
    ) -> Result<(), AppError>;
    pub fn current_segment(
        &self,
        limit_id: &str,
        window_kind: &str,
    ) -> Result<Vec<RateObservation>, AppError>;
}
```

Run migrations in a single transaction and record version `1` only after the SQL succeeds.

- [ ] **Step 5: Run tests**

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml storage::repository::tests
```

Expected: all storage tests PASS.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/migrations app/src-tauri/src/storage
git commit -m "feat: persist account usage observations"
```

### Task 5: Build the Reset-Aware Exhaustion Forecast

**Files:**
- Create: `app/src-tauri/src/forecast.rs`
- Test: inline tests in `forecast.rs`

- [ ] **Step 1: Write forecast behavior tests**

Define tests for these exact outcomes:

```rust
#[test]
fn predicts_exhaustion_before_reset_from_stable_samples() {
    let points = samples(200_000, &[(0, 10.0), (3600, 14.0), (7200, 18.0), (10800, 22.0)]);
    let result = forecast(&points, 10800, 200_000).unwrap();
    assert!(matches!(result.status, ForecastStatus::DepletesBeforeReset));
    assert!((result.rate_percent_per_hour - 4.0).abs() < 0.01);
}

#[test]
fn reports_not_expected_to_deplete_before_reset() {
    let points = samples(20_000, &[(0, 10.0), (7200, 11.0), (14400, 12.0)]);
    let result = forecast(&points, 14400, 20_000).unwrap();
    assert_eq!(result.status, ForecastStatus::SurvivesWindow);
}

#[test]
fn rejects_samples_across_reset_boundaries() { /* mismatched resets_at returns None */ }

#[test]
fn requires_three_samples_spanning_thirty_minutes() { /* returns None */ }

#[test]
fn a_falling_used_percent_starts_a_new_segment() { /* old samples are excluded */ }

fn samples(resets_at: i64, points: &[(i64, f64)]) -> Vec<RateObservation> {
    points
        .iter()
        .map(|(observed_at, used_percent)| RateObservation {
            observed_at: *observed_at,
            limit_id: "codex".into(),
            window_kind: "primary".into(),
            used_percent: *used_percent,
            window_duration_mins: 10_080,
            resets_at,
        })
        .collect()
}
```

- [ ] **Step 2: Verify failure**

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml forecast::tests
```

Expected: FAIL because `forecast` is undefined.

- [ ] **Step 3: Implement the estimator**

Use a Theil-Sen slope:

```rust
fn median_pairwise_slope(points: &[RateObservation]) -> Option<f64> {
    let mut slopes = Vec::new();
    for (i, left) in points.iter().enumerate() {
        for right in points.iter().skip(i + 1) {
            let hours = (right.observed_at - left.observed_at) as f64 / 3600.0;
            if hours > 0.0 {
                slopes.push((right.used_percent - left.used_percent) / hours);
            }
        }
    }
    slopes.sort_by(f64::total_cmp);
    slopes.get(slopes.len() / 2).copied()
}
```

Expose `forecast(points, now, resets_at) -> Option<ExhaustionForecast>`; it must reject
input whose latest segment does not match the supplied `resets_at`.

Public output:

```rust
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExhaustionForecast {
    pub status: ForecastStatus,
    pub rate_percent_per_hour: f64,
    pub exhausts_at: Option<i64>,
    pub confidence: ForecastConfidence,
    pub sample_count: usize,
    pub span_seconds: i64,
}
```

Rules:

- Require at least 3 samples spanning at least 1,800 seconds.
- Use only the latest monotonic segment with identical `limit_id`, `window_kind`,
  `window_duration_mins`, and `resets_at`.
- Treat slopes at or below `0.05` percentage points per hour as no measurable burn.
- Project `hours = (100 - latest.used_percent) / slope`.
- Return `SurvivesWindow` if projected exhaustion is at or after `resets_at`.
- Confidence is `High` for at least 8 samples spanning 4 hours with relative median
  absolute slope deviation at or below `0.25`.
- Confidence is `Medium` for at least 4 samples spanning 1 hour with relative deviation
  at or below `0.60`.
- Otherwise return `Low`.

- [ ] **Step 4: Run forecast tests**

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml forecast::tests
```

Expected: all forecast tests PASS.

- [ ] **Step 5: Run formatter and lints**

```bash
cargo fmt --manifest-path app/src-tauri/Cargo.toml --check
cargo clippy --manifest-path app/src-tauri/Cargo.toml --all-targets -- -D warnings
```

Expected: both commands exit `0`.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/forecast.rs
git commit -m "feat: forecast account quota exhaustion"
```

### Task 6: Assemble the Monitor Service and Tauri Commands

**Files:**
- Create: `app/src-tauri/src/monitor.rs`
- Create: `app/src-tauri/src/commands.rs`
- Complete: `app/src-tauri/src/domain/dashboard.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Test: inline monitor tests with a fake account source

- [ ] **Step 1: Define a replaceable account source**

```rust
#[async_trait::async_trait]
pub trait AccountSource: Send + Sync {
    async fn read_rate_limits(&self) -> Result<(RateLimitsResult, serde_json::Value), AppError>;
    async fn read_account_usage(&self) -> Result<(AccountUsageResult, serde_json::Value), AppError>;
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<RpcNotification>;
}
```

Run:

```bash
cargo add async-trait --manifest-path app/src-tauri/Cargo.toml
```

- [ ] **Step 2: Write monitor tests**

Tests must prove:

- Startup performs both account reads.
- A rate-limit notification triggers an immediate refresh.
- The periodic loop refreshes after 60 seconds under paused Tokio time.
- A failed usage read keeps quota data and marks account Token data unavailable.
- A disconnected app-server keeps the last snapshot and sets `is_stale = true`.

Use `#[tokio::test(start_paused = true)]` and `tokio::time::advance`.
Tokio’s `test-util` feature was enabled in Task 1.

- [ ] **Step 3: Define the frontend read model**

In `domain/dashboard.rs`:

```rust
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub observed_at: i64,
    pub is_stale: bool,
    pub connection_error: Option<String>,
    pub primary_quota: Option<QuotaView>,
    pub other_quotas: Vec<QuotaView>,
    pub account_usage: Option<AccountUsageView>,
    pub forecast: Option<ExhaustionForecast>,
    pub session_details_available: bool,
}
```

`QuotaView` must expose `limit_id`, `label`, `used_percent`, `remaining_percent`,
`window_duration_mins`, `resets_at`, and `plan_type`.

- [ ] **Step 4: Implement monitor behavior**

`AccountMonitor` owns:

```rust
pub struct AccountMonitor {
    source: std::sync::Arc<dyn AccountSource>,
    repository: std::sync::Arc<AccountRepository>,
    snapshot: tokio::sync::watch::Sender<DashboardSnapshot>,
}
```

Implement:

- `refresh()` for both reads and persistence.
- `run()` using `tokio::select!` over a 60-second interval, rate-limit notifications,
  and shutdown.
- App-server reconnect with delays of 1, 2, 5, 10, then 30 seconds.
- Stale threshold of 120 seconds since last successful quota read.

- [ ] **Step 5: Expose Tauri commands**

In `commands.rs`:

```rust
#[tauri::command]
pub async fn get_dashboard_snapshot(
    state: tauri::State<'_, AppState>,
) -> Result<DashboardSnapshot, String> {
    Ok(state.monitor.snapshot())
}

#[tauri::command]
pub async fn refresh_account(
    state: tauri::State<'_, AppState>,
) -> Result<DashboardSnapshot, String> {
    state.monitor.refresh().await.map_err(|error| error.to_string())
}
```

Emit `dashboard://updated` with the complete snapshot whenever the watch value changes.

- [ ] **Step 6: Run tests**

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml monitor::tests
cargo test --manifest-path app/src-tauri/Cargo.toml
```

Expected: all tests PASS.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/src
git commit -m "feat: expose live account dashboard state"
```

### Task 7: Add the Typed Frontend Bridge

**Files:**
- Create: `app/src/types/dashboard.ts`
- Create: `app/src/lib/backend.ts`
- Create: `app/src/hooks/useDashboard.ts`
- Test: `app/src/hooks/useDashboard.test.tsx`

- [ ] **Step 1: Define TypeScript contracts matching Rust**

Create `types/dashboard.ts`:

```ts
export type ForecastStatus =
  | "depletesBeforeReset"
  | "survivesWindow"
  | "noMeasurableBurn";

export type ForecastConfidence = "low" | "medium" | "high";

export interface QuotaView {
  limitId: string;
  label: string;
  usedPercent: number;
  remainingPercent: number;
  windowDurationMins: number;
  resetsAt: number;
  planType: string | null;
}

export interface AccountUsageView {
  lifetimeTokens: number | null;
  peakDailyTokens: number | null;
  dailyUsageBuckets: Array<{ startDate: string; tokens: number }>;
}

export interface ExhaustionForecast {
  status: ForecastStatus;
  ratePercentPerHour: number;
  exhaustsAt: number | null;
  confidence: ForecastConfidence;
  sampleCount: number;
  spanSeconds: number;
}

export interface DashboardSnapshot {
  observedAt: number;
  isStale: boolean;
  connectionError: string | null;
  primaryQuota: QuotaView | null;
  otherQuotas: QuotaView[];
  accountUsage: AccountUsageView | null;
  forecast: ExhaustionForecast | null;
  sessionDetailsAvailable: boolean;
}
```

- [ ] **Step 2: Write hook tests**

Mock `backend.getDashboardSnapshot`, `backend.refreshAccount`, and the event listener.
Verify:

- Initial loading state.
- Successful snapshot render.
- Event replaces the current snapshot.
- Refresh error preserves the prior snapshot and exposes a non-blocking error.
- Cleanup calls the Tauri unlisten function.

- [ ] **Step 3: Verify failure**

```bash
cd app
pnpm test -- src/hooks/useDashboard.test.tsx
```

Expected: FAIL because the hook and backend adapter do not exist.

- [ ] **Step 4: Implement the adapter and hook**

`backend.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DashboardSnapshot } from "../types/dashboard";

export const backend = {
  getDashboardSnapshot: () =>
    invoke<DashboardSnapshot>("get_dashboard_snapshot"),
  refreshAccount: () =>
    invoke<DashboardSnapshot>("refresh_account"),
  onDashboardUpdated: (handler: (snapshot: DashboardSnapshot) => void):
    Promise<UnlistenFn> =>
    listen<DashboardSnapshot>("dashboard://updated", event => handler(event.payload)),
};
```

`useDashboard.ts` must return:

```ts
{
  snapshot: DashboardSnapshot | null;
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}
```

- [ ] **Step 5: Run tests**

```bash
pnpm test -- src/hooks/useDashboard.test.tsx
```

Expected: all hook tests PASS.

- [ ] **Step 6: Commit**

```bash
git add app/src/types app/src/lib app/src/hooks
git commit -m "feat: connect dashboard UI to Tauri state"
```

### Task 8: Build the Approved Dashboard and All Data States

**Files:**
- Create: `app/src/styles/tokens.css`
- Create: `app/src/styles/app.css`
- Create: `app/src/components/AppSidebar.tsx`
- Create: `app/src/components/QuotaCard.tsx`
- Create: `app/src/components/UsageForecastPanel.tsx`
- Create: `app/src/components/SessionDetails.tsx`
- Create: `app/src/components/Dashboard.tsx`
- Create: `app/src/components/Dashboard.test.tsx`
- Modify: `app/src/App.tsx`
- Modify: `app/src/main.tsx`

- [ ] **Step 1: Write dashboard tests first**

The tests must assert:

```tsx
expect(screen.getByText("82%")).toBeVisible();
expect(screen.getByText("所有设备")).toBeVisible();
expect(screen.getByText("预计额度耗尽")).toBeVisible();
expect(screen.getByRole("heading", { name: "会话详情" })).toBeVisible();
expect(screen.queryByText("根会话")).not.toBeInTheDocument();
expect(screen.queryByText("子代理")).not.toBeInTheDocument();
```

Add separate tests for:

- Loading skeleton with no circular spinner.
- Account connection error with a visible retry button.
- Stale badge while retaining the last values.
- Missing `accountUsage` showing “账号 Token 数据暂不可用.”
- `SurvivesWindow` showing “按当前速率，本周期不会耗尽.”
- Session phase-1 empty state with no fabricated session rows.

- [ ] **Step 2: Verify failure**

```bash
cd app
pnpm test -- src/components/Dashboard.test.tsx
```

Expected: FAIL because components do not exist.

- [ ] **Step 3: Implement semantic design tokens**

`tokens.css` must define one blue accent, cool neutral surfaces, and the documented shape rule:

```css
:root {
  color-scheme: light dark;
  --canvas: #dfe5ed;
  --window: rgb(246 248 251 / 88%);
  --sidebar: rgb(226 232 240 / 74%);
  --surface: #fbfcfe;
  --text: #17191d;
  --muted: #656d79;
  --line: rgb(37 45 58 / 8%);
  --accent: #176fd1;
  --accent-soft: rgb(23 111 209 / 11%);
  --success: #257a55;
  --radius-window: 22px;
  --radius-panel: 17px;
  --radius-control: 10px;
}

@media (prefers-color-scheme: dark) {
  :root {
    --canvas: #111318;
    --window: rgb(31 34 40 / 90%);
    --sidebar: rgb(42 46 54 / 72%);
    --surface: #343840;
    --text: #f4f5f7;
    --muted: #afb5bf;
    --line: rgb(255 255 255 / 8%);
    --accent: #67a3ec;
    --accent-soft: rgb(103 163 236 / 15%);
    --success: #64bd91;
  }
}
```

Rules:

- System font only.
- Structural translucency only on window/sidebar.
- Solid data panels.
- No orange forecast accent; use graphite dash patterns.
- Animate only transform and opacity.
- Provide `prefers-reduced-motion`, `prefers-reduced-transparency`, and
  `prefers-contrast` overrides.

- [ ] **Step 4: Implement the components**

Use the approved information structure:

```tsx
<Dashboard>
  <AppSidebar />
  <main>
    <header>使用概览</header>
    <section className="account-grid">
      <QuotaCard />
      <UsageForecastPanel />
    </section>
    <SessionDetails />
  </main>
</Dashboard>
```

`QuotaCard` displays remaining percentage, used percentage, scheduled recovery, scope,
and last refresh. `UsageForecastPanel` displays account daily Tokens, local cost as
“Not available in phase 1,” forecast, burn rate, confidence, Token bars, and remaining-quota
line. `SessionDetails` renders the explicit phase-1 empty state.

Use Phosphor icons only. Do not use Unicode glyphs as navigation icons.

- [ ] **Step 5: Run frontend tests and build**

```bash
pnpm test
pnpm build
```

Expected: all tests PASS and Vite build exits `0`.

- [ ] **Step 6: Manually verify accessibility variants**

Run:

```bash
pnpm dev
```

Verify at 1220x820 and 900x640:

- Light and dark system themes.
- Reduced motion removes entry movement.
- Reduced transparency makes window/sidebar solid.
- Increased contrast strengthens borders and text.
- Keyboard focus is visible on every button.
- No label truncates at 200% browser zoom.

- [ ] **Step 7: Commit**

```bash
git add app/src
git commit -m "feat: build account usage dashboard"
```

### Task 9: Add the macOS Menu-Bar Experience

**Files:**
- Create: `app/src-tauri/src/tray.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Modify: `app/src-tauri/Cargo.toml`
- Add: `app/src-tauri/icons/trayTemplate.png`
- Test: `app/src-tauri/src/tray.rs` pure formatting tests

- [ ] **Step 1: Enable the tray feature**

Ensure `Cargo.toml` contains:

```toml
tauri = { version = "2", features = ["tray-icon", "image-png"] }
```

- [ ] **Step 2: Write tray-title tests**

```rust
#[test]
fn formats_remaining_quota_for_macos_title() {
    assert_eq!(tray_title(Some(82.4), false), "82%");
}

#[test]
fn marks_stale_quota_without_inventing_a_value() {
    assert_eq!(tray_title(Some(82.4), true), "82%?");
    assert_eq!(tray_title(None, true), "Codex?");
}
```

- [ ] **Step 3: Implement tray construction**

Use `tauri::tray::TrayIconBuilder` in setup:

```rust
let show = tauri::menu::MenuItem::with_id(app, "show", "Open Dashboard", true, None::<&str>)?;
let refresh = tauri::menu::MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;
let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit Codex Monitor", true, None::<&str>)?;
let menu = tauri::menu::Menu::with_items(app, &[&show, &refresh, &quit])?;

tauri::tray::TrayIconBuilder::with_id("codex-monitor")
    .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/trayTemplate.png"))?)
    .icon_as_template(true)
    .tooltip("Codex Monitor")
    .menu(&menu)
    .show_menu_on_left_click(true)
    .build(app)?;
```

Handle:

- `show`: show, center, and focus the main window.
- `refresh`: call the monitor refresh without blocking the UI thread.
- `quit`: exit with code `0`.
- Snapshot updates: call `tray.set_title(Some(&tray_title(...)))` on macOS and update
  tooltip text on every platform.

- [ ] **Step 4: Run Rust tests**

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml tray::tests
cargo clippy --manifest-path app/src-tauri/Cargo.toml --all-targets -- -D warnings
```

Expected: tests PASS; Clippy exits `0`.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri
git commit -m "feat: add macOS menu-bar status"
```

### Task 10: End-to-End Verification and Handoff

**Files:**
- Create: `app/README.md`
- Create: `docs/verification/phase-1-checklist.md`

- [ ] **Step 1: Document local prerequisites and commands**

`app/README.md` must include:

```text
Prerequisites:
- Rust stable
- Node.js and pnpm
- Codex CLI installed and logged into ChatGPT
- macOS 13 or newer for the first validated build

Development:
pnpm install
pnpm tauri dev

Verification:
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

- [ ] **Step 2: Run the complete automated verification**

From `app`:

```bash
pnpm test
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Expected: every command exits `0`.

- [ ] **Step 3: Run the real-account smoke test**

Run:

```bash
pnpm tauri dev
```

Verify and record evidence:

- App-server connection becomes healthy.
- Primary `codex` bucket matches a direct `account/rateLimits/read`.
- Every additional `rateLimitsByLimitId` bucket is preserved.
- Account daily Tokens match `account/usage/read` when the endpoint returns them.
- The first two observations do not fabricate a forecast.
- After enough samples, forecast scope reads “all devices.”
- Disconnecting Codex retains the last values and marks them stale.
- Menu-bar percentage opens the dashboard and Refresh triggers a new observation.
- No reset-credit card appears.
- Session section contains no fake data.

- [ ] **Step 4: Build the macOS bundle**

```bash
pnpm tauri build
```

Expected: Tauri reports a `.app` bundle and `.dmg` path under `src-tauri/target/release/bundle/`.

- [ ] **Step 5: Commit verification documentation**

```bash
git add app/README.md docs/verification/phase-1-checklist.md
git commit -m "docs: add phase one verification guide"
```

- [ ] **Step 6: Create follow-up implementation plans**

After Phase 1 is verified, use `superpowers:writing-plans` separately for:

1. Local session importer, lineage ownership, replay repair, and top-level session aggregation.
2. Model price catalog, equivalent API cost, and historical recalculation.
3. Notifications, production hardening, and macOS release packaging.
4. Windows platform adapters, packaging, and validation on the user’s second machine.
