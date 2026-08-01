use crate::{
    domain::{
        dashboard::{DashboardSnapshot, TrayPanelSnapshot},
        session::LocalSessionView,
    },
    monitor::AccountMonitor,
    sessions::service::SessionService,
    settings::{AppSettings, SettingsRuntime},
    storage::repository::AccountRepository,
};
use std::sync::Arc;
use tokio::sync::watch;

pub struct AppState {
    pub monitor: Arc<AccountMonitor>,
    pub sessions: Arc<SessionService>,
    pub repository: Arc<AccountRepository>,
    pub settings: SettingsRuntime,
    pub shutdown: watch::Sender<bool>,
}

#[tauri::command]
pub fn get_app_settings(state: tauri::State<'_, AppState>) -> AppSettings {
    state.settings.current()
}

#[tauri::command]
pub fn save_app_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    use tauri_plugin_autostart::ManagerExt;

    settings.validate()?;
    let autolaunch = app.autolaunch();
    let autolaunch_enabled = autolaunch.is_enabled().map_err(|error| error.to_string())?;
    if settings.launch_at_login != autolaunch_enabled {
        if settings.launch_at_login {
            autolaunch.enable().map_err(|error| error.to_string())?;
        } else {
            autolaunch.disable().map_err(|error| error.to_string())?;
        }
    }
    state
        .repository
        .save_settings(&settings)
        .map_err(|error| error.to_string())?;
    state.settings.update(settings.clone());
    Ok(settings)
}

#[tauri::command]
pub async fn rescan_sessions(
    state: tauri::State<'_, AppState>,
) -> Result<LocalSessionView, String> {
    state
        .sessions
        .rescan()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_dashboard_snapshot(
    state: tauri::State<'_, AppState>,
) -> Result<DashboardSnapshot, String> {
    Ok(state.monitor.snapshot())
}

#[tauri::command]
pub async fn get_tray_snapshot(
    state: tauri::State<'_, AppState>,
) -> Result<TrayPanelSnapshot, String> {
    Ok(TrayPanelSnapshot::from(&state.monitor.snapshot()))
}

#[tauri::command]
pub async fn refresh_account(
    state: tauri::State<'_, AppState>,
) -> Result<DashboardSnapshot, String> {
    state
        .monitor
        .refresh()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn refresh_tray_snapshot(
    state: tauri::State<'_, AppState>,
) -> Result<TrayPanelSnapshot, String> {
    state
        .monitor
        .refresh()
        .await
        .map(|snapshot| TrayPanelSnapshot::from(&snapshot))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_dashboard(app: tauri::AppHandle, destination: Option<String>) {
    if destination.as_deref() == Some("settings") {
        crate::tray::show_settings(&app);
    } else {
        crate::tray::show_main_window(&app);
    }
}

#[tauri::command]
pub fn hide_tray_panel(app: tauri::AppHandle) {
    crate::tray::hide_tray_panel(&app);
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}
