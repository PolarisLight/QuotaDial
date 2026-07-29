# Settings, Pace, and Session Sorting Design

Date: 2026-07-30  
Status: Approved

## Purpose

Complete the first usable settings experience and make the quota pace indicator
actionable without removing the existing short-term burn-rate calculation.

This change also adds explicit session sorting and replaces development copy in
the sidebar footer with product version and copyright information.

## Confirmed Product Behavior

### Pace indicator

The quota chart toolbar provides a two-option segmented control:

- `配速建议`
- `近期消耗率`

`配速建议` is the default. The choice is saved as the user's default display
mode.

The suggested pace follows the Codex Pacer interpretation:

```text
suggested pace ratio = remaining quota percent / remaining time percent
```

The ratio is displayed as guidance rather than as a daily consumption rate. For
example:

```text
明显偏快 · 建议降至 56%
```

The status thresholds are:

- below 85%: fast;
- 85% through 115%: normal;
- above 115%: slow.

The `近期消耗率` mode retains the existing post-reset observation calculation
and labels the result explicitly:

```text
近期消耗率 · 95.8%/天
```

It is a short-term rate extrapolated to one day, not a claim about actual
calendar-day consumption.

### Session sorting

Session details remain sorted by most recent activity by default.

A compact sorting menu in the session panel header provides:

- `最近活动`;
- `Token 最多`;
- `Token 最少`.

Sorting is performed over root-session rows after child usage has already been
merged. Expanding a row does not change the selected sort order.

### Settings navigation

The disabled sidebar item becomes a real navigation destination. The main
content switches between:

- usage overview;
- settings.

The tray menu `设置` item opens the main window and navigates to the same
settings destination. There is no separate settings window.

### Settings

The settings page contains:

1. Appearance
   - system, light, or dark theme.
2. Quota presentation
   - default pace mode: suggested pace or recent burn rate.
3. Refresh
   - account quota interval: 1, 5, or 15 minutes;
   - local session scan interval: 5, 10, or 30 minutes.
4. Startup
   - launch at login, disabled by default.
5. Notifications
   - weekly quota remaining threshold, default 25%;
   - critical remaining threshold, default 10%;
   - quota reset notification;
   - data-stale notification, default after 15 minutes.

Each notification category can be disabled independently. Threshold controls
must preserve `critical < warning`, and invalid values are rejected before
persistence.

Settings are saved locally and survive app restarts. The Tauri application uses
the persisted settings as the authoritative state. Browser preview uses a
local-storage adapter with the same shape so that the settings UI remains
interactive outside Tauri.

### Startup and notifications

Launch-at-login uses the cross-platform Tauri autostart integration and must
perform a real operating-system registration on macOS and Windows.

Desktop notifications use the Tauri notification integration. The application
requests notification permission only when notifications are enabled.

Quota threshold notifications are deduplicated by quota window and threshold.
Reset notifications are deduplicated by the new window identity. Stale-data
notifications fire once per stale episode and become eligible again only after
fresh data has been observed.

### Sidebar footer

The development copy:

```text
账号级监控
覆盖所有设备
```

is replaced with:

```text
Codex Monitor v0.1.0
© 2026 yetform
```

`yetform` links to `https://yetform.cyhao.space/`. The footer does not show
`All rights reserved`.

The displayed version comes from the application package version rather than a
separate hard-coded product constant. Browser preview may use the same build
version as a fallback.

## Architecture

### Shared settings model

Introduce one serialized `AppSettings` model shared across the Tauri command
boundary. Rust validates and persists the model in SQLite. The frontend obtains
the current settings at startup and saves complete, validated updates.

Runtime consumers observe settings changes:

- the account monitor updates its polling interval;
- the local session service updates its scan interval;
- the notification evaluator updates thresholds and enabled categories;
- the UI updates theme and default pace mode;
- the autostart adapter reconciles operating-system state after a save.

### Frontend structure

`Dashboard` owns the active destination and loaded settings. It passes explicit
navigation callbacks to the sidebar and listens for a tray navigation event.

The settings form is a focused component rather than being added to the
dashboard component. Display-only pace calculations live in a pure helper so
that both modes can be tested without rendering the full dashboard.

Session sorting is also implemented through a pure comparison helper and a
small controlled menu in the session panel.

### Notification evaluation

Notification eligibility is a pure state transition:

```text
previous notification state + current snapshot + settings
  -> notifications to send + next notification state
```

Persisted notification state prevents duplicates across restarts. Notification
delivery failures are logged and do not interrupt quota monitoring.

## Error Handling

- A settings save failure leaves the last confirmed settings active and shows
  an inline error.
- Failure to register autostart does not pretend the setting succeeded.
- Denied notification permission disables notification delivery and presents a
  clear explanation in settings.
- Unsupported browser-preview system actions remain visibly marked as preview
  behavior while display settings continue to work.
- Invalid threshold relationships are rejected before writing to storage.

## Testing

### Rust

- settings validation and persistence round trip;
- dynamic interval selection;
- autostart reconciliation boundary;
- threshold, reset, and stale notification deduplication;
- notification state persistence.

### React

- suggested-pace formula and status thresholds;
- switching between pace modes and saving the default;
- default recent-session order;
- Token descending and ascending sort options;
- sidebar navigation to settings;
- tray navigation event to settings;
- settings validation, save success, and save failure;
- version and yetform footer content.

### Manual verification

- exercise both pace modes against live quota data;
- sort and expand session rows;
- open settings from both the sidebar and tray menu;
- restart the app and verify settings persistence;
- enable and disable launch at login;
- grant and deny notification permission;
- inspect light and dark modes in the browser preview and the macOS bundle.

