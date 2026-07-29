pub mod app_server;
pub mod commands;
pub mod domain;
pub mod error;
pub mod forecast;
pub mod monitor;
pub mod sessions;
pub mod storage;
pub mod tray;

use commands::AppState;
use monitor::{AccountMonitor, CodexAccountSource};
use std::sync::Arc;
use storage::repository::AccountRepository;
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_snapshot,
            commands::refresh_account
        ])
        .setup(|app| {
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
            let source = Arc::new(CodexAccountSource::new());
            let monitor = Arc::new(AccountMonitor::new(source, repository));
            let (shutdown, shutdown_receiver) = tokio::sync::watch::channel(false);
            let _tray = tray::build(app.handle(), monitor.clone())?;

            let monitor_task = monitor.clone();
            tauri::async_runtime::spawn(monitor_task.run(shutdown_receiver));

            let app_handle = app.handle().clone();
            let mut snapshots = monitor.subscribe();
            tauri::async_runtime::spawn(async move {
                while snapshots.changed().await.is_ok() {
                    let snapshot = snapshots.borrow().clone();
                    let _ = app_handle.emit("dashboard://updated", snapshot);
                }
            });

            app.manage(AppState { monitor, shutdown });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
