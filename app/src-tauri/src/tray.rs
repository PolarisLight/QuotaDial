use crate::{
    domain::dashboard::{DashboardSnapshot, TrayPanelSnapshot},
    forecast::ForecastStatus,
    monitor::AccountMonitor,
    sessions::service::SessionService,
    tray_icon::{render_tray_icon, render_windows_percentage_icon, TrayDialState, TrayIconStyle},
};
use chrono::Datelike;
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition,
};

pub fn tray_title(remaining_percent: Option<f64>, stale: bool) -> String {
    match remaining_percent {
        Some(remaining) if stale => format!("{remaining:.0}%?"),
        Some(remaining) => format!("{remaining:.0}%"),
        None if stale => "Codex?".into(),
        None => "Codex".into(),
    }
}

pub fn tray_dial_state(snapshot: &DashboardSnapshot) -> TrayDialState {
    TrayDialState {
        used_percent: snapshot
            .primary_quota
            .as_ref()
            .map(|quota| quota.used_percent as f32),
        stale: snapshot.is_stale,
    }
}

fn live_tray_icon(state: TrayDialState, remaining_percent: Option<f32>) -> (Image<'static>, bool) {
    if cfg!(target_os = "macos") {
        (render_tray_icon(state, 44, TrayIconStyle::Template), true)
    } else {
        (
            render_windows_percentage_icon(remaining_percent, state.stale, 32),
            false,
        )
    }
}

fn uses_native_menu_bar() -> bool {
    cfg!(target_os = "macos")
}

fn opens_panel_from_tray(button: MouseButton, state: MouseButtonState) -> bool {
    !uses_native_menu_bar()
        && state == MouseButtonState::Up
        && matches!(button, MouseButton::Left | MouseButton::Right)
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuState {
    pub quota: String,
    pub progress: String,
    pub reset: String,
    pub forecast: String,
    pub today_tokens: String,
    pub sessions: String,
    pub updated: String,
}

fn usage_progress(used_percent: f64) -> String {
    let used = used_percent.clamp(0.0, 100.0);
    let filled = (used / 10.0).round() as usize;
    format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled))
}

pub fn menu_state(snapshot: &DashboardSnapshot) -> MenuState {
    let (quota, progress, reset) = snapshot.primary_quota.as_ref().map_or_else(
        || {
            (
                "额度暂不可用".to_owned(),
                "消耗  —".to_owned(),
                "重置  —".to_owned(),
            )
        },
        |quota| {
            let used = quota.used_percent.clamp(0.0, 100.0);
            let reset = chrono::DateTime::from_timestamp(quota.resets_at, 0)
                .map(|value| {
                    value
                        .with_timezone(&chrono::Local)
                        .format("重置  %-m 月 %-d 日 %H:%M")
                        .to_string()
                })
                .unwrap_or_else(|| "重置  —".into());
            (
                format!("额度剩余 {:.0}% · 已用 {used:.0}%", quota.remaining_percent),
                format!("消耗  {}  {used:.0}%", usage_progress(used)),
                reset,
            )
        },
    );
    let forecast = match snapshot.forecast.as_ref().map(|item| item.status) {
        Some(ForecastStatus::DepletesBeforeReset) => snapshot
            .forecast
            .as_ref()
            .and_then(|item| item.exhausts_at)
            .and_then(|timestamp| chrono::DateTime::from_timestamp(timestamp, 0))
            .map(|value| {
                format!(
                    "预测  {}耗尽",
                    value
                        .with_timezone(&chrono::Local)
                        .format("%-m 月 %-d 日 %H:%M")
                )
            })
            .unwrap_or_else(|| "预测  本周期可能耗尽".into()),
        Some(ForecastStatus::SurvivesWindow) => "预测  重置前不会耗尽".into(),
        Some(ForecastStatus::NoMeasurableBurn) => "预测  当前消耗很低".into(),
        None => "预测  正在积累样本".into(),
    };
    let latest_tokens = snapshot.account_usage.as_ref().and_then(|usage| {
        usage
            .daily_usage_buckets
            .iter()
            .max_by_key(|item| &item.start_date)
    });
    let observed_date = chrono::DateTime::from_timestamp(snapshot.observed_at, 0)
        .filter(|_| snapshot.observed_at > 0)
        .map(|value| {
            value
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d")
                .to_string()
        });
    let today_tokens = latest_tokens
        .map(|bucket| {
            let label = if observed_date.as_deref() == Some(bucket.start_date.as_str()) {
                "今日 Token".to_owned()
            } else {
                chrono::NaiveDate::parse_from_str(&bucket.start_date, "%Y-%m-%d")
                    .map(|date| format!("{} 月 {} 日 Token", date.month(), date.day()))
                    .unwrap_or_else(|_| format!("{} Token", bucket.start_date))
            };
            format!("{label}  {}", format_token_count(bucket.tokens))
        })
        .unwrap_or_else(|| "今日 Token  —".into());
    let updated = chrono::DateTime::from_timestamp(snapshot.observed_at, 0)
        .filter(|_| snapshot.observed_at > 0)
        .map(|value| {
            format!(
                "最近更新  {}",
                value.with_timezone(&chrono::Local).format("%H:%M")
            )
        })
        .unwrap_or_else(|| "最近更新  —".into());
    MenuState {
        quota,
        progress,
        reset,
        forecast,
        today_tokens,
        sessions: format!("本机会话  {} 个", snapshot.local_sessions.sessions.len()),
        updated,
    }
}

pub fn should_hide_on_close(window_label: &str) -> bool {
    matches!(window_label, "main" | "tray-panel")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    ShowOverview,
    Refresh,
    Settings,
    Quit,
    None,
}

fn menu_action(id: &str) -> MenuAction {
    match id {
        "show" => MenuAction::ShowOverview,
        "refresh" => MenuAction::Refresh,
        "settings" => MenuAction::Settings,
        "quit" => MenuAction::Quit,
        _ => MenuAction::None,
    }
}

fn format_token_count(tokens: i64) -> String {
    if tokens >= 100_000_000 {
        format!("{:.1} 亿", tokens as f64 / 100_000_000.0)
    } else if tokens >= 10_000 {
        format!("{:.1} 万", tokens as f64 / 10_000.0)
    } else {
        tokens.to_string()
    }
}

pub fn build(
    app: &AppHandle,
    monitor: Arc<AccountMonitor>,
    session_service: Arc<SessionService>,
) -> tauri::Result<TrayIcon> {
    let quota = MenuItem::with_id(app, "quota", "额度暂不可用", true, None::<&str>)?;
    let progress = MenuItem::with_id(app, "progress", "消耗  —", true, None::<&str>)?;
    let reset = MenuItem::with_id(app, "reset", "重置  —", true, None::<&str>)?;
    let forecast = MenuItem::with_id(app, "forecast", "预测  正在积累样本", true, None::<&str>)?;
    let separator_one = PredefinedMenuItem::separator(app)?;
    let today_tokens = MenuItem::with_id(app, "today", "今日 Token  —", true, None::<&str>)?;
    let sessions_count =
        MenuItem::with_id(app, "session-count", "本机会话  0 个", true, None::<&str>)?;
    let updated = MenuItem::with_id(app, "updated", "最近更新  —", true, None::<&str>)?;
    let separator_two = PredefinedMenuItem::separator(app)?;
    let show = MenuItem::with_id(app, "show", "打开仪表盘", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "立即刷新", true, None::<&str>)?;
    let separator_three = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 QuotaDial", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &quota,
            &progress,
            &reset,
            &forecast,
            &separator_one,
            &today_tokens,
            &sessions_count,
            &updated,
            &separator_two,
            &show,
            &refresh,
            &separator_three,
            &settings,
            &quit,
        ],
    )?;
    let menu_monitor = monitor.clone();
    let menu_sessions = session_service.clone();
    let panel_monitor = monitor.clone();
    let refresh_item = refresh.clone();
    let (initial_icon, initial_as_template) = live_tray_icon(
        TrayDialState {
            used_percent: None,
            stale: false,
        },
        None,
    );
    let tray_builder = TrayIconBuilder::with_id("codex-monitor")
        .icon(initial_icon)
        .icon_as_template(initial_as_template)
        .tooltip("QuotaDial")
        .title(tray_title(None, false))
        .show_menu_on_left_click(uses_native_menu_bar());
    let tray_builder = if uses_native_menu_bar() {
        tray_builder.menu(&menu)
    } else {
        tray_builder
    };
    let tray = tray_builder
        .on_menu_event(move |app, event| match menu_action(event.id().as_ref()) {
            MenuAction::ShowOverview => {
                show_main_window(app);
            }
            MenuAction::Refresh => {
                let monitor = menu_monitor.clone();
                let sessions = menu_sessions.clone();
                let refresh = refresh_item.clone();
                let _ = refresh.set_enabled(false);
                tauri::async_runtime::spawn(async move {
                    let _ = tokio::join!(monitor.refresh(), sessions.rescan());
                    let _ = refresh.set_enabled(true);
                });
            }
            MenuAction::Settings => {
                show_main_window(app);
                let _ = app.emit_to("main", "dashboard://open-settings", ());
            }
            MenuAction::Quit => app.exit(0),
            MenuAction::None => {}
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                position,
                button,
                button_state,
                ..
            } = event
            {
                if opens_panel_from_tray(button, button_state) {
                    toggle_tray_panel(tray.app_handle(), position, &panel_monitor);
                }
            }
        })
        .build(app)?;

    let tray_updates = tray.clone();
    let quota_updates = quota.clone();
    let progress_updates = progress.clone();
    let reset_updates = reset.clone();
    let forecast_updates = forecast.clone();
    let today_updates = today_tokens.clone();
    let session_updates = sessions_count.clone();
    let updated_updates = updated.clone();
    let mut snapshots = monitor.subscribe();
    tauri::async_runtime::spawn(async move {
        while snapshots.changed().await.is_ok() {
            let snapshot = snapshots.borrow().clone();
            let state = menu_state(&snapshot);
            let remaining = snapshot
                .primary_quota
                .as_ref()
                .map(|quota| quota.remaining_percent);
            let title = tray_title(remaining, snapshot.is_stale);
            let (icon, as_template) = live_tray_icon(
                tray_dial_state(&snapshot),
                remaining.map(|value| value as f32),
            );
            let _ = tray_updates.set_icon_with_as_template(Some(icon), as_template);
            let tooltip = match remaining {
                Some(value) => format!("QuotaDial · 剩余 {value:.0}%"),
                None => "QuotaDial · 额度暂不可用".into(),
            };
            let _ = tray_updates.set_title(Some(title));
            let _ = tray_updates.set_tooltip(Some(tooltip));
            let _ = quota_updates.set_text(state.quota);
            let _ = progress_updates.set_text(state.progress);
            let _ = reset_updates.set_text(state.reset);
            let _ = forecast_updates.set_text(state.forecast);
            let _ = today_updates.set_text(state.today_tokens);
            let _ = session_updates.set_text(state.sessions);
            let _ = updated_updates.set_text(state.updated);
        }
    });

    Ok(tray)
}

pub fn show_main_window(app: &AppHandle) {
    hide_tray_panel(app);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn show_settings(app: &AppHandle) {
    show_main_window(app);
    let _ = app.emit_to("main", "dashboard://open-settings", ());
}

pub fn hide_tray_panel(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("tray-panel") {
        let _ = window.hide();
    }
}

fn tray_panel_position(
    click: PhysicalPosition<f64>,
    window_size: (u32, u32),
    monitor_origin: (i32, i32),
    monitor_size: (u32, u32),
) -> PhysicalPosition<i32> {
    let margin = 10;
    let gap = 12;
    let width = window_size.0 as i32;
    let height = window_size.1 as i32;
    let left = monitor_origin.0 + margin;
    let top = monitor_origin.1 + margin;
    let right = monitor_origin.0 + monitor_size.0 as i32 - margin;
    let bottom = monitor_origin.1 + monitor_size.1 as i32 - margin;
    let max_x = (right - width).max(left);
    let max_y = (bottom - height).max(top);
    let click_x = click.x.round() as i32;
    let click_y = click.y.round() as i32;
    let x = (click_x - width / 2).clamp(left, max_x);
    let preferred_above = click_y - height - gap;
    let preferred_y = if preferred_above >= top {
        preferred_above
    } else {
        click_y + gap
    };
    PhysicalPosition::new(x, preferred_y.clamp(top, max_y))
}

fn toggle_tray_panel(app: &AppHandle, click: PhysicalPosition<f64>, monitor: &AccountMonitor) {
    let Some(window) = app.get_webview_window("tray-panel") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }

    let size = window.outer_size().unwrap_or_else(|_| (360, 496).into());
    let selected_monitor = window.available_monitors().ok().and_then(|monitors| {
        monitors.into_iter().find(|monitor| {
            let origin = monitor.position();
            let size = monitor.size();
            click.x >= origin.x as f64
                && click.x < (origin.x + size.width as i32) as f64
                && click.y >= origin.y as f64
                && click.y < (origin.y + size.height as i32) as f64
        })
    });
    let position = selected_monitor.map_or_else(
        || {
            PhysicalPosition::new(
                click.x.round() as i32 - size.width as i32 / 2,
                click.y.round() as i32 - size.height as i32 - 12,
            )
        },
        |monitor| {
            tray_panel_position(
                click,
                (size.width, size.height),
                (monitor.position().x, monitor.position().y),
                (monitor.size().width, monitor.size().height),
            )
        },
    );
    let _ = window.set_position(position);
    let _ = window.show();
    let _ = app.emit_to(
        "tray-panel",
        "tray://updated",
        TrayPanelSnapshot::from(&monitor.snapshot()),
    );
    let _ = window.set_focus();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            account::DailyUsageBucket,
            dashboard::{AccountUsageView, DashboardSnapshot, QuotaView},
            session::{SessionSummary, TokenBreakdown},
        },
        forecast::{ExhaustionForecast, ForecastConfidence, ForecastStatus},
    };

    fn snapshot_with_quota_and_sessions() -> DashboardSnapshot {
        let mut snapshot = DashboardSnapshot {
            observed_at: 1_785_330_000,
            is_stale: false,
            primary_quota: Some(QuotaView {
                limit_id: "codex".into(),
                label: "7 日额度".into(),
                window_kind: "primary".into(),
                used_percent: 25.0,
                remaining_percent: 75.0,
                window_duration_mins: 10_080,
                resets_at: 1_785_903_626,
                plan_type: Some("plus".into()),
            }),
            account_usage: Some(AccountUsageView {
                lifetime_tokens: None,
                peak_daily_tokens: None,
                daily_usage_buckets: vec![DailyUsageBucket {
                    start_date: "2026-07-29".into(),
                    tokens: 31_736_527,
                }],
            }),
            forecast: Some(ExhaustionForecast {
                status: ForecastStatus::SurvivesWindow,
                rate_percent_per_hour: 0.2,
                exhausts_at: None,
                confidence: ForecastConfidence::Medium,
                sample_count: 4,
                span_seconds: 7_200,
            }),
            ..DashboardSnapshot::default()
        };
        snapshot.local_sessions.sessions = (0..3)
            .map(|index| SessionSummary {
                session_id: format!("session-{index}"),
                title: format!("会话 {index}"),
                project_path: None,
                last_active_at: 1_785_330_000,
                primary_model: None,
                tokens: TokenBreakdown::default(),
                monthly_tokens: TokenBreakdown::default(),
                equivalent_cost_usd: None,
                monthly_equivalent_cost_usd: None,
                priced_tokens: 0,
                unpriced_tokens: 0,
                monthly_priced_tokens: 0,
                monthly_unpriced_tokens: 0,
                child_session_count: 0,
            })
            .collect();
        snapshot
    }

    #[test]
    fn formats_remaining_quota_for_macos_title() {
        assert_eq!(tray_title(Some(82.4), false), "82%");
    }

    #[test]
    fn marks_stale_quota_without_inventing_a_value() {
        assert_eq!(tray_title(Some(82.4), true), "82%?");
        assert_eq!(tray_title(None, true), "Codex?");
    }

    #[test]
    fn shows_product_name_before_first_observation() {
        assert_eq!(tray_title(None, false), "Codex");
    }

    #[test]
    fn maps_snapshot_to_live_dial_state() {
        let snapshot = snapshot_with_quota_and_sessions();
        assert_eq!(
            tray_dial_state(&snapshot),
            crate::tray_icon::TrayDialState {
                used_percent: Some(25.0),
                stale: false,
            }
        );
    }

    #[test]
    fn maps_missing_quota_to_neutral_dial_state() {
        let snapshot = DashboardSnapshot {
            is_stale: true,
            ..DashboardSnapshot::default()
        };
        assert_eq!(
            tray_dial_state(&snapshot),
            crate::tray_icon::TrayDialState {
                used_percent: None,
                stale: true,
            }
        );
    }

    #[test]
    fn live_icon_uses_the_native_platform_style() {
        let (image, as_template) = live_tray_icon(
            TrayDialState {
                used_percent: Some(25.0),
                stale: false,
            },
            Some(75.0),
        );

        let expected_size = if cfg!(target_os = "macos") { 44 } else { 32 };
        assert_eq!(image.width(), expected_size);
        assert_eq!(image.height(), expected_size);
        assert_eq!(as_template, cfg!(target_os = "macos"));
    }

    #[test]
    fn formats_complete_menu_status_from_one_snapshot() {
        let state = menu_state(&snapshot_with_quota_and_sessions());
        assert_eq!(state.quota, "额度剩余 75% · 已用 25%");
        assert_eq!(state.progress, "消耗  ███░░░░░░░  25%");
        assert!(state.reset.starts_with("重置  8 月 5 日"));
        assert_eq!(state.forecast, "预测  重置前不会耗尽");
        assert_eq!(state.today_tokens, "今日 Token  3173.7 万");
        assert_eq!(state.sessions, "本机会话  3 个");
    }

    #[test]
    fn formats_missing_account_data_without_hiding_local_sessions() {
        let snapshot = DashboardSnapshot {
            local_sessions: snapshot_with_quota_and_sessions().local_sessions,
            ..DashboardSnapshot::default()
        };
        let state = menu_state(&snapshot);
        assert_eq!(state.quota, "额度暂不可用");
        assert_eq!(state.sessions, "本机会话  3 个");
    }

    #[test]
    fn labels_a_delayed_daily_bucket_with_its_actual_date() {
        let mut snapshot = snapshot_with_quota_and_sessions();
        snapshot.observed_at = 1_785_383_781;

        let state = menu_state(&snapshot);

        assert_eq!(state.today_tokens, "7 月 29 日 Token  3173.7 万");
    }

    #[test]
    fn only_the_main_window_is_hidden_instead_of_closed() {
        assert!(should_hide_on_close("main"));
        assert!(should_hide_on_close("tray-panel"));
        assert!(!should_hide_on_close("settings"));
    }

    #[test]
    fn status_rows_do_not_trigger_actions() {
        for id in [
            "quota",
            "progress",
            "reset",
            "forecast",
            "today",
            "session-count",
            "updated",
        ] {
            assert_eq!(menu_action(id), MenuAction::None);
        }
        assert_eq!(menu_action("show"), MenuAction::ShowOverview);
        assert_eq!(menu_action("sessions"), MenuAction::None);
        assert_eq!(menu_action("settings"), MenuAction::Settings);
    }

    #[test]
    fn uses_native_menu_only_on_macos() {
        assert_eq!(uses_native_menu_bar(), cfg!(target_os = "macos"));
    }

    #[test]
    fn windows_left_and_right_click_open_the_same_panel() {
        let expected = !cfg!(target_os = "macos");
        assert_eq!(
            opens_panel_from_tray(MouseButton::Left, MouseButtonState::Up),
            expected
        );
        assert_eq!(
            opens_panel_from_tray(MouseButton::Right, MouseButtonState::Up),
            expected
        );
        assert!(!opens_panel_from_tray(
            MouseButton::Middle,
            MouseButtonState::Up
        ));
        assert!(!opens_panel_from_tray(
            MouseButton::Left,
            MouseButtonState::Down
        ));
    }

    #[test]
    fn positions_windows_flyout_above_and_inside_the_work_area() {
        let position = tray_panel_position(
            PhysicalPosition::new(1_880.0, 1_060.0),
            (360, 496),
            (0, 0),
            (1_920, 1_080),
        );
        assert_eq!(position.x, 1_550);
        assert_eq!(position.y, 552);
    }

    #[test]
    fn maps_usage_to_a_ten_segment_progress_bar() {
        assert_eq!(usage_progress(0.0), "░░░░░░░░░░");
        assert_eq!(usage_progress(25.0), "███░░░░░░░");
        assert_eq!(usage_progress(50.0), "█████░░░░░");
        assert_eq!(usage_progress(75.0), "████████░░");
        assert_eq!(usage_progress(100.0), "██████████");
    }
}
