pub mod app_server;
pub mod commands;
pub mod domain;
pub mod error;
pub mod forecast;
pub mod monitor;
pub mod notifications;
pub mod quota_trend;
pub mod sessions;
pub mod settings;
#[cfg(test)]
mod settings_contract_tests;
pub mod storage;
pub mod tray;
pub mod tray_icon;

use commands::AppState;
use domain::dashboard::TrayPanelSnapshot;
use monitor::{AccountMonitor, CodexAccountSource};
use sessions::{discovery::codex_home, service::SessionService};
use std::{ffi::OsStr, sync::Arc};
use storage::repository::AccountRepository;
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_notification::NotificationExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let started_by_autostart = is_autostart_launch(std::env::args_os());
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_snapshot,
            commands::get_tray_snapshot,
            commands::refresh_account,
            commands::refresh_tray_snapshot,
            commands::rescan_sessions,
            commands::get_app_settings,
            commands::save_app_settings,
            commands::open_dashboard,
            commands::hide_tray_panel,
            commands::quit_app
        ])
        .on_window_event(|window, event| {
            if tray::should_hide_on_close(window.label()) {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            if window.label() == "tray-panel" && matches!(event, tauri::WindowEvent::Focused(false))
            {
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data)?;
            let repository = Arc::new(AccountRepository::open(
                &app_data.join("account-observations.sqlite3"),
            )?);
            let settings = settings::SettingsRuntime::new(repository.load_settings()?);
            if settings.current().launch_at_login {
                app.autolaunch().enable()?;
            }
            let source = Arc::new(CodexAccountSource::new());
            let sessions = Arc::new(SessionService::new(repository.clone(), codex_home()?));
            let monitor = Arc::new(AccountMonitor::new(
                source,
                repository.clone(),
                sessions.clone(),
            ));
            let (shutdown, _) = tokio::sync::watch::channel(false);
            let _tray = tray::build(app.handle(), monitor.clone(), sessions.clone())?;
            if !started_by_autostart {
                if let Some(window) = app.get_webview_window("main") {
                    window.show()?;
                    window.set_focus()?;
                }
            }

            // Subscribe before the monitor starts so its first completed refresh cannot
            // race past the webview event forwarder.
            let mut snapshots = monitor.subscribe();
            let monitor_task = monitor.clone();
            tauri::async_runtime::spawn(
                monitor_task.run_with_settings(shutdown.subscribe(), settings.subscribe()),
            );
            let session_task = sessions.clone();
            tauri::async_runtime::spawn(
                session_task.run_with_settings(shutdown.subscribe(), settings.subscribe()),
            );

            let session_monitor = monitor.clone();
            let mut session_updates = sessions.subscribe();
            tauri::async_runtime::spawn(async move {
                while session_updates.changed().await.is_ok() {
                    session_monitor.apply_local_sessions(session_updates.borrow().clone());
                }
            });

            let app_handle = app.handle().clone();
            let notification_repository = repository.clone();
            let notification_settings = settings.clone();
            tauri::async_runtime::spawn(async move {
                let mut tracker = notifications::NotificationTracker::default();
                while snapshots.changed().await.is_ok() {
                    let snapshot = snapshots.borrow().clone();
                    let _ = app_handle.emit_to("main", "dashboard://updated", snapshot.clone());
                    let _ = app_handle.emit_to(
                        "tray-panel",
                        "tray://updated",
                        TrayPanelSnapshot::from(&snapshot),
                    );
                    let now = chrono::Utc::now().timestamp();
                    for event in tracker.evaluate(&snapshot, &notification_settings.current(), now)
                    {
                        if notification_repository
                            .notification_was_delivered(&event.key)
                            .unwrap_or(false)
                        {
                            continue;
                        }
                        if app_handle
                            .notification()
                            .builder()
                            .title(event.title)
                            .body(event.body)
                            .show()
                            .is_ok()
                        {
                            let _ = notification_repository
                                .mark_notification_delivered(&event.key, now);
                        }
                    }
                }
            });

            app.manage(AppState {
                monitor,
                sessions,
                repository,
                settings,
                shutdown,
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn is_autostart_launch<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    args.into_iter()
        .any(|argument| argument.as_ref() == OsStr::new("--autostart"))
}

#[cfg(test)]
mod startup_tests {
    use super::is_autostart_launch;

    #[test]
    fn recognizes_hidden_autostart_launches() {
        assert!(is_autostart_launch(["quotadial", "--autostart"]));
        assert!(!is_autostart_launch(["quotadial"]));
    }
}
