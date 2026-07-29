use crate::{
    domain::dashboard::DashboardSnapshot,
    forecast::ForecastStatus,
    monitor::AccountMonitor,
    sessions::service::SessionService,
    tray_icon::{render_tray_icon, TrayDialState},
};
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Emitter, Manager,
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

fn live_tray_icon(state: TrayDialState) -> (Image<'static>, bool) {
    (render_tray_icon(state, 44), true)
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

pub fn menu_state(snapshot: &DashboardSnapshot) -> MenuState {
    let (quota, progress, reset) = snapshot.primary_quota.as_ref().map_or_else(
        || {
            (
                "等待账号额度".to_owned(),
                "消耗  —".to_owned(),
                "重置  —".to_owned(),
            )
        },
        |quota| {
            let used = quota.used_percent.clamp(0.0, 100.0);
            let filled = (used / 10.0).round() as usize;
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));
            let reset = chrono::DateTime::from_timestamp(quota.resets_at, 0)
                .map(|value| {
                    value
                        .with_timezone(&chrono::Local)
                        .format("重置  %-m 月 %-d 日 %H:%M")
                        .to_string()
                })
                .unwrap_or_else(|| "重置  —".into());
            (
                format!("剩余 {:.0}% · 已消耗 {:.0}%", quota.remaining_percent, used),
                format!("消耗  {bar}  {used:.0}%"),
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
    let today_tokens = snapshot
        .account_usage
        .as_ref()
        .and_then(|usage| {
            usage
                .daily_usage_buckets
                .iter()
                .max_by_key(|item| &item.start_date)
        })
        .map(|bucket| format!("今日 Token  {}", format_token_count(bucket.tokens)))
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
    let quota = MenuItem::with_id(app, "quota", "等待账号额度", false, None::<&str>)?;
    let progress = MenuItem::with_id(app, "progress", "消耗  —", false, None::<&str>)?;
    let reset = MenuItem::with_id(app, "reset", "重置  —", false, None::<&str>)?;
    let forecast = MenuItem::with_id(app, "forecast", "预测  正在积累样本", false, None::<&str>)?;
    let separator_one = PredefinedMenuItem::separator(app)?;
    let today_tokens = MenuItem::with_id(app, "today", "今日 Token  —", false, None::<&str>)?;
    let sessions_count =
        MenuItem::with_id(app, "session-count", "本机会话  0 个", false, None::<&str>)?;
    let updated = MenuItem::with_id(app, "updated", "最近更新  —", false, None::<&str>)?;
    let separator_two = PredefinedMenuItem::separator(app)?;
    let show = MenuItem::with_id(app, "show", "打开仪表盘", true, None::<&str>)?;
    let show_sessions = MenuItem::with_id(app, "sessions", "查看最近会话", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "立即刷新", true, None::<&str>)?;
    let separator_three = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, "settings", "设置（稍后）", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 Codex Monitor", true, None::<&str>)?;
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
            &show_sessions,
            &refresh,
            &separator_three,
            &settings,
            &quit,
        ],
    )?;
    let menu_monitor = monitor.clone();
    let menu_sessions = session_service.clone();
    let refresh_item = refresh.clone();
    let (initial_icon, initial_as_template) = live_tray_icon(TrayDialState {
        used_percent: None,
        stale: false,
    });

    let tray = TrayIconBuilder::with_id("codex-monitor")
        .icon(initial_icon)
        .icon_as_template(initial_as_template)
        .tooltip("Codex Monitor")
        .title(tray_title(None, false))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => {
                show_main_window(app);
            }
            "sessions" => {
                show_main_window(app);
                let _ = app.emit("dashboard://focus-section", "sessions");
            }
            "refresh" => {
                let monitor = menu_monitor.clone();
                let sessions = menu_sessions.clone();
                let refresh = refresh_item.clone();
                let _ = refresh.set_enabled(false);
                tauri::async_runtime::spawn(async move {
                    let _ = tokio::join!(monitor.refresh(), sessions.rescan());
                    let _ = refresh.set_enabled(true);
                });
            }
            "quit" => app.exit(0),
            _ => {}
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
            let (icon, as_template) = live_tray_icon(tray_dial_state(&snapshot));
            let _ = tray_updates.set_icon_with_as_template(Some(icon), as_template);
            let tooltip = match remaining {
                Some(value) => format!("Codex Monitor · 剩余 {value:.0}%"),
                None => "Codex Monitor · 额度暂不可用".into(),
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

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
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
                equivalent_cost_usd: None,
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
    fn live_icon_updates_remain_macos_templates() {
        let (image, as_template) = live_tray_icon(TrayDialState {
            used_percent: Some(25.0),
            stale: false,
        });

        assert_eq!(image.width(), 44);
        assert_eq!(image.height(), 44);
        assert!(as_template);
    }

    #[test]
    fn formats_complete_menu_status_from_one_snapshot() {
        let state = menu_state(&snapshot_with_quota_and_sessions());
        assert_eq!(state.quota, "剩余 75% · 已消耗 25%");
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
        assert_eq!(state.quota, "等待账号额度");
        assert_eq!(state.sessions, "本机会话  3 个");
    }
}
