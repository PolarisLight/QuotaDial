# Settings, Pace, and Session Sorting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a functional cross-platform settings page, selectable quota pace metrics, explicit session sorting, and production footer metadata.

**Architecture:** Persist one validated `AppSettings` record in SQLite and publish it through a watch channel to runtime consumers. Keep pace guidance and session sorting as pure frontend helpers, while Rust owns polling, autostart reconciliation, notification evaluation, and durable notification deduplication. Use the same typed frontend settings adapter for Tauri and browser preview.

**Tech Stack:** Tauri 2, Rust, Tokio watch channels, SQLite/rusqlite, React 19, TypeScript, Vitest, Testing Library, `tauri-plugin-autostart`, and `tauri-plugin-notification`.

---

## File Map

### New files

- `app/src-tauri/migrations/003_settings.sql` — singleton settings and notification-state tables.
- `app/src-tauri/src/settings.rs` — settings defaults, validation, persistence-facing type, and runtime watch channel.
- `app/src-tauri/src/notifications.rs` — pure notification eligibility state machine and delivery adapter.
- `app/src/types/settings.ts` — frontend settings contract.
- `app/src/lib/pace.ts` — pure suggested-pace and recent-rate presentation.
- `app/src/lib/sessionSort.ts` — pure root-session sorting.
- `app/src/components/SettingsPage.tsx` — settings form and save/error states.
- `app/src/components/SettingsPage.test.tsx` — settings behavior tests.
- `app/src/lib/pace.test.ts` — pace formula tests.
- `app/src/lib/sessionSort.test.ts` — sorting tests.

### Modified files

- `app/src-tauri/Cargo.toml` and `app/package.json` — Tauri plugins.
- `app/src-tauri/src/storage/migrations.rs` — register migration 3.
- `app/src-tauri/src/storage/repository.rs` — load/save settings and notification state.
- `app/src-tauri/src/error.rs` — typed invalid-settings failure.
- `app/src-tauri/src/commands.rs` — settings commands.
- `app/src-tauri/src/lib.rs` — initialize plugins, settings runtime, and notification observer.
- `app/src-tauri/src/monitor.rs` — dynamic account refresh interval.
- `app/src-tauri/src/sessions/service.rs` — dynamic local scan interval.
- `app/src-tauri/src/tray.rs` — enable settings item and emit navigation.
- `app/src/lib/backend.ts` — settings API, preview persistence, tray-settings event, and app version.
- `app/src/components/Dashboard.tsx` — destination state and settings integration.
- `app/src/components/AppSidebar.tsx` — real navigation and product footer.
- `app/src/components/UsageForecastPanel.tsx` — pace-mode state plumbing.
- `app/src/components/UsageQuotaChart.tsx` — segmented metric switch.
- `app/src/components/SessionDetails.tsx` — sorting menu.
- `app/src/components/Dashboard.test.tsx` and `app/src/components/UsageQuotaChart.test.tsx` — integration coverage.
- `app/src/App.css` — settings, segmented control, sorting menu, and footer styles.

### Generated lock files

- `app/Cargo.lock`
- `app/pnpm-lock.yaml`

---

### Task 1: Persist and validate application settings

**Files:**
- Create: `app/src-tauri/migrations/003_settings.sql`
- Create: `app/src-tauri/src/settings.rs`
- Modify: `app/src-tauri/src/storage/migrations.rs`
- Modify: `app/src-tauri/src/storage/repository.rs`
- Modify: `app/src-tauri/src/error.rs`
- Test: `app/src-tauri/src/settings.rs`
- Test: `app/src-tauri/src/storage/repository.rs`

- [ ] **Step 1: Write failing settings validation and repository round-trip tests**

Add tests that establish the contract:

```rust
#[test]
fn defaults_match_the_confirmed_product_behavior() {
    let settings = AppSettings::default();
    assert_eq!(settings.theme, ThemePreference::System);
    assert_eq!(settings.pace_mode, PaceMode::Suggested);
    assert_eq!(settings.account_refresh_mins, 1);
    assert_eq!(settings.session_scan_mins, 10);
    assert!(!settings.launch_at_login);
    assert_eq!(settings.warning_remaining_percent, 25);
    assert_eq!(settings.critical_remaining_percent, 10);
}

#[test]
fn rejects_inverted_notification_thresholds() {
    let settings = AppSettings {
        warning_remaining_percent: 10,
        critical_remaining_percent: 25,
        ..AppSettings::default()
    };
    assert_eq!(
        settings.validate().unwrap_err().to_string(),
        "critical quota threshold must be lower than warning threshold"
    );
}

#[test]
fn settings_round_trip_survives_repository_reopen() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let repository = AccountRepository::open(file.path()).unwrap();
    let expected = AppSettings {
        theme: ThemePreference::Dark,
        pace_mode: PaceMode::RecentRate,
        launch_at_login: true,
        ..AppSettings::default()
    };
    repository.save_settings(&expected).unwrap();
    drop(repository);

    let reopened = AccountRepository::open(file.path()).unwrap();
    assert_eq!(reopened.load_settings().unwrap(), expected);
}
```

- [ ] **Step 2: Run the targeted tests and verify RED**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml settings
```

Expected: compilation fails because `AppSettings`, validation, and repository methods do not exist.

- [ ] **Step 3: Add migration 3**

Create:

```sql
CREATE TABLE app_settings (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    payload_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE notification_delivery_state (
    state_key TEXT PRIMARY KEY,
    delivered_at INTEGER NOT NULL
);
```

Register it in `MIGRATIONS`:

```rust
(3, include_str!("../../migrations/003_settings.sql")),
```

- [ ] **Step 4: Implement the settings model**

Define serde-compatible enums and defaults:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference { System, Light, Dark }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PaceMode { Suggested, RecentRate }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: ThemePreference,
    pub pace_mode: PaceMode,
    pub account_refresh_mins: u64,
    pub session_scan_mins: u64,
    pub launch_at_login: bool,
    pub quota_warning_enabled: bool,
    pub warning_remaining_percent: u8,
    pub quota_critical_enabled: bool,
    pub critical_remaining_percent: u8,
    pub reset_notification_enabled: bool,
    pub stale_notification_enabled: bool,
    pub stale_after_mins: u64,
}
```

Validation accepts only the confirmed interval choices and requires:

```rust
if self.critical_remaining_percent >= self.warning_remaining_percent {
    return Err(AppError::InvalidSettings(
        "critical quota threshold must be lower than warning threshold".into()
    ));
}
```

Add the typed validation error:

```rust
#[error("invalid settings: {0}")]
InvalidSettings(String),
```

- [ ] **Step 5: Implement repository load/save methods**

Use a singleton JSON row:

```rust
pub fn load_settings(&self) -> Result<AppSettings, AppError> {
    let payload = self.lock()?.query_row(
        "SELECT payload_json FROM app_settings WHERE singleton = 1",
        [],
        |row| row.get::<_, String>(0),
    ).optional()?;
    match payload {
        Some(value) => Ok(serde_json::from_str(&value)?),
        None => Ok(AppSettings::default()),
    }
}

pub fn save_settings(&self, settings: &AppSettings) -> Result<(), AppError> {
    settings.validate()?;
    self.lock()?.execute(
        "INSERT INTO app_settings(singleton, payload_json, updated_at)
         VALUES (1, ?1, unixepoch())
         ON CONFLICT(singleton) DO UPDATE SET
           payload_json = excluded.payload_json,
           updated_at = excluded.updated_at",
        [serde_json::to_string(settings)?],
    )?;
    Ok(())
}
```

- [ ] **Step 6: Run settings tests and verify GREEN**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml settings
```

Expected: all settings and migration tests pass.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/migrations/003_settings.sql app/src-tauri/src/settings.rs app/src-tauri/src/storage/migrations.rs app/src-tauri/src/storage/repository.rs app/src-tauri/src/error.rs
git commit -m "feat: persist monitor settings"
```

---

### Task 2: Make account and session refresh intervals dynamic

**Files:**
- Modify: `app/src-tauri/src/settings.rs`
- Modify: `app/src-tauri/src/monitor.rs`
- Modify: `app/src-tauri/src/sessions/service.rs`
- Modify: `app/src-tauri/src/commands.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Test: `app/src-tauri/src/monitor.rs`
- Test: `app/src-tauri/src/sessions/service.rs`

- [ ] **Step 1: Write failing paused-time tests**

Add tests proving a settings update changes the next refresh cadence without restarting:

```rust
#[tokio::test(start_paused = true)]
async fn account_monitor_applies_updated_refresh_interval() {
    let (settings, _) = SettingsRuntime::new(AppSettings::default());
    let task = spawn_counting_account_loop(settings.subscribe());
    tokio::time::advance(Duration::from_secs(60)).await;
    assert_eq!(task.refresh_count(), 1);

    settings.update(AppSettings {
        account_refresh_mins: 5,
        ..AppSettings::default()
    });
    tokio::time::advance(Duration::from_secs(4 * 60)).await;
    assert_eq!(task.refresh_count(), 1);
    tokio::time::advance(Duration::from_secs(60)).await;
    assert_eq!(task.refresh_count(), 2);
}
```

Add the equivalent test for `session_scan_mins`.

- [ ] **Step 2: Run targeted tests and verify RED**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml updated_refresh_interval
```

Expected: FAIL because both loops use compile-time constants.

- [ ] **Step 3: Implement `SettingsRuntime`**

Wrap a Tokio watch sender:

```rust
#[derive(Clone)]
pub struct SettingsRuntime {
    sender: watch::Sender<AppSettings>,
}

impl SettingsRuntime {
    pub fn new(initial: AppSettings) -> Self {
        let (sender, _) = watch::channel(initial);
        Self { sender }
    }
    pub fn current(&self) -> AppSettings { self.sender.borrow().clone() }
    pub fn subscribe(&self) -> watch::Receiver<AppSettings> { self.sender.subscribe() }
    pub fn update(&self, value: AppSettings) { self.sender.send_replace(value); }
}
```

- [ ] **Step 4: Replace fixed intervals with resettable sleeps**

In each runtime loop, select between shutdown, settings changes, and a sleep:

```rust
loop {
    let delay = Duration::from_secs(settings.borrow().account_refresh_mins * 60);
    tokio::select! {
        _ = tokio::time::sleep(delay) => self.refresh().await,
        changed = settings.changed() => {
            if changed.is_err() { break; }
            continue;
        }
        changed = shutdown.changed() => {
            if changed.is_err() || *shutdown.borrow() { break; }
        }
    }
}
```

Use `session_scan_mins` in the session service.

- [ ] **Step 5: Add settings commands**

Expose:

```rust
#[tauri::command]
pub fn get_app_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings.current()
}

#[tauri::command]
pub fn save_app_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, AppError> {
    settings.validate()?;
    reconcile_autostart(&app, settings.launch_at_login)?;
    state.repository.save_settings(&settings)?;
    state.settings.update(settings.clone());
    Ok(settings)
}
```

Extend `AppState` with `repository` and `settings`.

- [ ] **Step 6: Run dynamic interval tests and verify GREEN**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml updated_refresh_interval
cargo test --manifest-path app/src-tauri/Cargo.toml settings
```

Expected: all targeted tests pass.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/src/settings.rs app/src-tauri/src/monitor.rs app/src-tauri/src/sessions/service.rs app/src-tauri/src/commands.rs app/src-tauri/src/lib.rs
git commit -m "feat: apply live refresh settings"
```

---

### Task 3: Add real autostart and deduplicated notifications

**Files:**
- Modify: `app/src-tauri/Cargo.toml`
- Modify: `app/package.json`
- Create: `app/src-tauri/src/notifications.rs`
- Modify: `app/src-tauri/src/storage/repository.rs`
- Modify: `app/src-tauri/src/commands.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Test: `app/src-tauri/src/notifications.rs`

- [ ] **Step 1: Write failing notification transition tests**

Cover warning, critical, reset, stale, and recovery:

```rust
#[test]
fn threshold_notification_fires_once_per_window() {
    let settings = AppSettings::default();
    let snapshot = snapshot("weekly:1785903626", 24, false);
    let first = evaluate(&NotificationState::default(), &snapshot, &settings);
    assert_eq!(first.notifications, vec![NotificationKind::QuotaWarning]);

    let repeated = evaluate(&first.next_state, &snapshot, &settings);
    assert!(repeated.notifications.is_empty());
}

#[test]
fn reset_and_stale_episodes_are_deduplicated() {
    let before = snapshot("weekly:old", 8, false);
    let after = snapshot("weekly:new", 100, false);
    let reset = evaluate(&state_after(&before), &after, &AppSettings::default());
    assert_eq!(reset.notifications, vec![NotificationKind::QuotaReset]);

    let stale = snapshot("weekly:new", 100, true);
    let first_stale = evaluate(&reset.next_state, &stale, &AppSettings::default());
    assert_eq!(first_stale.notifications, vec![NotificationKind::DataStale]);
    assert!(evaluate(&first_stale.next_state, &stale, &AppSettings::default())
        .notifications.is_empty());
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml notifications
```

Expected: compilation fails because the notification evaluator does not exist.

- [ ] **Step 3: Add plugins**

Add Rust dependencies:

```toml
tauri-plugin-autostart = "2"
tauri-plugin-notification = "2"
```

Add frontend dependencies:

```json
"@tauri-apps/plugin-autostart": "^2.0.0",
"@tauri-apps/plugin-notification": "^2.0.0"
```

Run:

```bash
cd app && pnpm install
```

- [ ] **Step 4: Implement the pure notification evaluator**

Use stable keys:

```rust
fn warning_key(window: &QuotaView, threshold: u8) -> String {
    format!("quota:{}:{}:{}", window.limit_id, window.resets_at, threshold)
}

pub fn evaluate(
    previous: &NotificationState,
    snapshot: &DashboardSnapshot,
    settings: &AppSettings,
) -> NotificationEvaluation {
    let mut next = previous.clone();
    let mut notifications = Vec::new();

    if let Some(window) = snapshot.primary_quota.as_ref() {
        let window_key = format!("{}:{}", window.limit_id, window.resets_at);
        if settings.reset_notification_enabled
            && previous.window_key.as_deref().is_some_and(|old| old != window_key)
        {
            let key = format!("reset:{window_key}");
            if next.delivered_keys.insert(key) {
                notifications.push(NotificationKind::QuotaReset);
            }
        }
        for (enabled, threshold, kind) in [
            (
                settings.quota_warning_enabled,
                settings.warning_remaining_percent,
                NotificationKind::QuotaWarning,
            ),
            (
                settings.quota_critical_enabled,
                settings.critical_remaining_percent,
                NotificationKind::QuotaCritical,
            ),
        ] {
            let key = warning_key(window, threshold);
            if enabled
                && window.remaining_percent <= f64::from(threshold)
                && next.delivered_keys.insert(key)
            {
                notifications.push(kind);
            }
        }
        next.window_key = Some(window_key);
    }

    if snapshot.is_stale && settings.stale_notification_enabled {
        if !previous.stale_episode_active {
            notifications.push(NotificationKind::DataStale);
        }
        next.stale_episode_active = true;
    } else {
        next.stale_episode_active = false;
    }

    NotificationEvaluation {
        notifications,
        next_state: next,
    }
}
```

Persist delivered keys in `notification_delivery_state`. Prune keys older than
the current and previous quota windows so the table remains bounded.

- [ ] **Step 5: Initialize and reconcile autostart**

Initialize:

```rust
.plugin(tauri_plugin_autostart::init(
    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
    Some(vec!["--autostart"]),
))
.plugin(tauri_plugin_notification::init())
```

Implement `reconcile_autostart` with `tauri_plugin_autostart::ManagerExt`,
calling `enable` or `disable`, and return an error if OS state cannot be changed.

- [ ] **Step 6: Deliver notifications from snapshot updates**

Observe monitor snapshots, evaluate state, persist keys, and send:

```rust
app_handle
    .notification()
    .builder()
    .title("Codex Monitor")
    .body(notification.body())
    .show()?;
```

Log delivery failures without stopping the snapshot emitter.

- [ ] **Step 7: Run notification tests and verify GREEN**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml notifications
cargo test --manifest-path app/src-tauri/Cargo.toml storage
```

Expected: all targeted tests pass.

- [ ] **Step 8: Commit**

```bash
git add app/src-tauri/Cargo.toml app/Cargo.lock app/package.json app/pnpm-lock.yaml app/src-tauri/src/notifications.rs app/src-tauri/src/storage/repository.rs app/src-tauri/src/commands.rs app/src-tauri/src/lib.rs
git commit -m "feat: add startup and quota notifications"
```

---

### Task 4: Build the settings page and real navigation

**Files:**
- Create: `app/src/types/settings.ts`
- Create: `app/src/components/SettingsPage.tsx`
- Create: `app/src/components/SettingsPage.test.tsx`
- Modify: `app/src/lib/backend.ts`
- Modify: `app/src/components/Dashboard.tsx`
- Modify: `app/src/components/AppSidebar.tsx`
- Modify: `app/src-tauri/src/tray.rs`
- Modify: `app/src/components/Dashboard.test.tsx`

- [ ] **Step 1: Write failing navigation and settings tests**

Test sidebar navigation, save, validation, footer, and tray navigation:

```tsx
test("opens settings from the sidebar and saves valid settings", async () => {
  render(<DashboardView {...props} />);
  await user.click(screen.getByRole("button", { name: "设置" }));
  expect(screen.getByRole("heading", { name: "设置" })).toBeVisible();
  await user.click(screen.getByLabelText("开机启动"));
  await user.click(screen.getByRole("button", { name: "保存设置" }));
  expect(backend.saveAppSettings).toHaveBeenCalledWith(
    expect.objectContaining({ launchAtLogin: true }),
  );
});

test("shows package version and yetform copyright", () => {
  render(<AppSidebar {...props} version="0.1.0" />);
  expect(screen.getByText("Codex Monitor v0.1.0")).toBeVisible();
  expect(screen.getByRole("link", { name: "© 2026 yetform" }))
    .toHaveAttribute("href", "https://yetform.cyhao.space/");
});
```

- [ ] **Step 2: Run frontend tests and verify RED**

Run:

```bash
cd app && pnpm test -- SettingsPage Dashboard
```

Expected: FAIL because settings navigation and components do not exist.

- [ ] **Step 3: Add the frontend settings contract and backend adapter**

Define:

```ts
export type ThemePreference = "system" | "light" | "dark";
export type PaceMode = "suggested" | "recentRate";

export interface AppSettings {
  theme: ThemePreference;
  paceMode: PaceMode;
  accountRefreshMins: 1 | 5 | 15;
  sessionScanMins: 5 | 10 | 30;
  launchAtLogin: boolean;
  quotaWarningEnabled: boolean;
  warningRemainingPercent: number;
  quotaCriticalEnabled: boolean;
  criticalRemainingPercent: number;
  resetNotificationEnabled: boolean;
  staleNotificationEnabled: boolean;
  staleAfterMins: number;
}
```

Add backend methods:

```ts
getAppSettings(): Promise<AppSettings>
saveAppSettings(settings: AppSettings): Promise<AppSettings>
getAppVersion(): Promise<string>
requestNotificationPermission(): Promise<boolean>
onOpenSettings(handler: () => void): Promise<UnlistenFn>
```

Browser preview persists the same model under
`codex-monitor-preview-settings`.

- [ ] **Step 4: Implement `SettingsPage`**

Use native form controls grouped into Appearance, Quota presentation, Refresh,
Startup, and Notifications. Keep a draft separate from confirmed settings.
Before save:

```ts
if (
  draft.quotaWarningEnabled &&
  draft.quotaCriticalEnabled &&
  draft.criticalRemainingPercent >= draft.warningRemainingPercent
) {
  setError("紧急阈值必须低于提醒阈值");
  return;
}
```

Request notification permission only when at least one notification category is
enabled. Show save progress, saved confirmation, and inline failure.

- [ ] **Step 5: Implement destination navigation**

Use:

```ts
type AppDestination = "overview" | "settings";
```

`Dashboard` owns this state. `AppSidebar` receives `destination` and
`onNavigate`. The tray item emits `dashboard://open-settings`; the frontend
listener selects the settings destination.

Enable the tray item:

```rust
let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
```

On click, show the window and emit `dashboard://open-settings`.

- [ ] **Step 6: Implement footer metadata**

Render:

```tsx
<p>Codex Monitor v{version}</p>
<a href="https://yetform.cyhao.space/" target="_blank" rel="noreferrer">
  © 2026 yetform
</a>
```

Use Tauri `getVersion()` in the app and `0.1.0` as the browser fallback.

- [ ] **Step 7: Run frontend tests and verify GREEN**

Run:

```bash
cd app && pnpm test -- SettingsPage Dashboard
```

Expected: all targeted tests pass.

- [ ] **Step 8: Commit**

```bash
git add app/src/types/settings.ts app/src/components/SettingsPage.tsx app/src/components/SettingsPage.test.tsx app/src/lib/backend.ts app/src/components/Dashboard.tsx app/src/components/AppSidebar.tsx app/src-tauri/src/tray.rs app/src/components/Dashboard.test.tsx
git commit -m "feat: add functional monitor settings"
```

---

### Task 5: Add selectable Pacer guidance and recent burn rate

**Files:**
- Create: `app/src/lib/pace.ts`
- Create: `app/src/lib/pace.test.ts`
- Modify: `app/src/components/UsageQuotaChart.tsx`
- Modify: `app/src/components/UsageForecastPanel.tsx`
- Modify: `app/src/components/Dashboard.tsx`
- Modify: `app/src/components/UsageQuotaChart.test.tsx`

- [ ] **Step 1: Write failing pace helper tests**

```ts
test("calculates suggested pace from remaining quota and remaining time", () => {
  expect(
    suggestedPace({
      remainingPercent: 51,
      observedAt: start + 14 * hour,
      periodStart: start,
      resetsAt: start + 7 * day,
    }),
  ).toEqual({
    ratioPercent: 56,
    status: "fast",
    copy: "明显偏快 · 建议降至 56%",
  });
});

test.each([
  [84, "fast"],
  [85, "normal"],
  [115, "normal"],
  [116, "slow"],
])("classifies %s percent as %s", (ratioPercent, status) => {
  expect(classifySuggestedPace(ratioPercent)).toBe(status);
});

test("labels the historical slope as a recent rate", () => {
  expect(recentRateCopy({ percentPerDay: 95.8, status: "fast" }))
    .toBe("近期消耗率 · 95.8%/天");
});
```

- [ ] **Step 2: Run pace tests and verify RED**

Run:

```bash
cd app && pnpm test -- src/lib/pace.test.ts
```

Expected: FAIL because the helper module does not exist.

- [ ] **Step 3: Implement pure pace helpers**

Compute remaining time from the authoritative window:

```ts
const total = resetsAt - periodStart;
const remainingTimePercent = clamp(
  ((resetsAt - observedAt) / total) * 100,
  0,
  100,
);
const ratioPercent =
  remainingTimePercent === 0
    ? remainingPercent === 0 ? 100 : 1000
    : Math.round((remainingPercent / remainingTimePercent) * 100);
```

Return concise Chinese copy and the 85/115 status.

- [ ] **Step 4: Add the chart segmented control**

Replace the single badge with two buttons:

```tsx
<div className="pace-mode-control" aria-label="节奏指标">
  <button aria-pressed={mode === "suggested"} onClick={() => onModeChange("suggested")}>
    配速建议
  </button>
  <button aria-pressed={mode === "recentRate"} onClick={() => onModeChange("recentRate")}>
    近期消耗率
  </button>
</div>
```

Render the chosen metric beside the control. Changing the mode immediately
updates the current settings draft and saves it as the default through the
settings owner.

- [ ] **Step 5: Run pace and chart tests and verify GREEN**

Run:

```bash
cd app && pnpm test -- src/lib/pace.test.ts src/components/UsageQuotaChart.test.tsx
```

Expected: all targeted tests pass.

- [ ] **Step 6: Commit**

```bash
git add app/src/lib/pace.ts app/src/lib/pace.test.ts app/src/components/UsageQuotaChart.tsx app/src/components/UsageForecastPanel.tsx app/src/components/Dashboard.tsx app/src/components/UsageQuotaChart.test.tsx
git commit -m "feat: switch quota pace metrics"
```

---

### Task 6: Add explicit session sorting

**Files:**
- Create: `app/src/lib/sessionSort.ts`
- Create: `app/src/lib/sessionSort.test.ts`
- Modify: `app/src/components/SessionDetails.tsx`
- Modify: `app/src/components/Dashboard.test.tsx`

- [ ] **Step 1: Write failing sorting tests**

```ts
test("defaults to most recently active first", () => {
  expect(sortSessions(sessions, "recent").map(item => item.sessionId))
    .toEqual(["newest", "older"]);
});

test("sorts by total input plus output tokens", () => {
  expect(sortSessions(sessions, "tokensDesc").map(item => item.sessionId))
    .toEqual(["larger", "smaller"]);
  expect(sortSessions(sessions, "tokensAsc").map(item => item.sessionId))
    .toEqual(["smaller", "larger"]);
});

test("does not mutate the backend session array", () => {
  const original = [...sessions];
  sortSessions(sessions, "tokensDesc");
  expect(sessions).toEqual(original);
});
```

- [ ] **Step 2: Run sorting tests and verify RED**

Run:

```bash
cd app && pnpm test -- src/lib/sessionSort.test.ts
```

Expected: FAIL because `sortSessions` does not exist.

- [ ] **Step 3: Implement the pure sorter**

```ts
export type SessionSort = "recent" | "tokensDesc" | "tokensAsc";

export function sortSessions(sessions: SessionSummary[], sort: SessionSort) {
  return [...sessions].sort((left, right) => {
    if (sort === "recent") return right.lastActiveAt - left.lastActiveAt;
    const delta = totalTokens(left.tokens) - totalTokens(right.tokens);
    return sort === "tokensAsc" ? delta : -delta;
  });
}
```

Use `lastActiveAt` as the tie breaker for Token modes.

- [ ] **Step 4: Add the sorting menu**

Place a labeled native select in the session heading:

```tsx
<label className="session-sort">
  <span>排序</span>
  <select value={sort} onChange={event => setSort(event.target.value as SessionSort)}>
    <option value="recent">最近活动</option>
    <option value="tokensDesc">Token 最多</option>
    <option value="tokensAsc">Token 最少</option>
  </select>
</label>
```

Map the sorted copy, not `view.sessions`.

- [ ] **Step 5: Run sorting and dashboard tests and verify GREEN**

Run:

```bash
cd app && pnpm test -- src/lib/sessionSort.test.ts src/components/Dashboard.test.tsx
```

Expected: all targeted tests pass.

- [ ] **Step 6: Commit**

```bash
git add app/src/lib/sessionSort.ts app/src/lib/sessionSort.test.ts app/src/components/SessionDetails.tsx app/src/components/Dashboard.test.tsx
git commit -m "feat: sort session usage details"
```

---

### Task 7: Polish the settings, controls, and footer

**Files:**
- Modify: `app/src/App.css`
- Modify: `app/src/components/SettingsPage.tsx`
- Modify: `app/src/components/AppSidebar.tsx`
- Modify: `app/src/components/SessionDetails.tsx`
- Modify: `app/src/components/UsageQuotaChart.tsx`

- [ ] **Step 1: Add a visual regression checklist before styling**

Record the required states in component comments or the manual verification
section:

```text
- Settings page fits 900×640 without horizontal scrolling.
- Save action and errors remain visible.
- Pace switch does not change chart height.
- Session sort fits beside the session count.
- Footer remains readable in light and dark modes.
```

- [ ] **Step 2: Implement focused styles**

Add styles for:

```css
.settings-page { display: grid; gap: 16px; max-width: 920px; }
.settings-section { border: 1px solid var(--line); border-radius: 18px; }
.pace-mode-control { display: inline-flex; padding: 3px; border-radius: 10px; }
.pace-mode-control button[aria-pressed="true"] { background: var(--surface-raised); }
.session-sort select { min-width: 132px; }
.sidebar-footer a { color: inherit; text-decoration: none; }
.sidebar-footer a:hover { text-decoration: underline; }
```

Use existing color variables and spacing. Do not create a second visual system.

- [ ] **Step 3: Run component tests**

Run:

```bash
cd app && pnpm test -- SettingsPage Dashboard UsageQuotaChart
```

Expected: all targeted tests pass.

- [ ] **Step 4: Commit**

```bash
git add app/src/App.css app/src/components/SettingsPage.tsx app/src/components/AppSidebar.tsx app/src/components/SessionDetails.tsx app/src/components/UsageQuotaChart.tsx
git commit -m "style: polish monitor settings controls"
```

---

### Task 8: Full verification and bundle smoke test

**Files:**
- Modify only if verification exposes a defect.

- [ ] **Step 1: Format and lint**

Run:

```bash
cargo fmt --manifest-path app/src-tauri/Cargo.toml --check
cd app && pnpm lint
```

Expected: both commands exit 0; existing non-blocking warnings must be reported.

- [ ] **Step 2: Run all frontend tests**

Run:

```bash
cd app && pnpm test
```

Expected: all Vitest suites pass with zero failures.

- [ ] **Step 3: Run all Rust tests**

Run:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml
```

Expected: all Rust tests pass with zero failures.

- [ ] **Step 4: Build frontend and Tauri bundle**

Run:

```bash
cd app && pnpm build
cd app && pnpm tauri build --debug
```

Expected: TypeScript/Vite build and the debug macOS bundle complete successfully.

- [ ] **Step 5: Verify the browser preview**

At `http://localhost:5173/`:

1. switch to Settings from the sidebar;
2. change theme, pace default, intervals, and thresholds;
3. save and reload to confirm preview persistence;
4. return to Overview and switch pace modes;
5. sort sessions in all three orders;
6. verify footer version and yetform link;
7. inspect 900×640 and 1220×820 in light and dark modes.

- [ ] **Step 6: Verify native-only behavior**

In the debug app:

1. use the tray Settings item;
2. toggle launch at login and confirm OS registration state;
3. grant notification permission;
4. use controlled test snapshots to cross warning and critical thresholds;
5. confirm one notification per threshold/window;
6. confirm reset and stale notifications deduplicate.

- [ ] **Step 7: Inspect final diff and commit any verification-only fixes**

Run:

```bash
git diff --check
git status --short
git log --oneline -8
```

Expected: no whitespace errors and only intended files changed.
