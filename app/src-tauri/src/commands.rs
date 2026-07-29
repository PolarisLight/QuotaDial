use crate::{domain::dashboard::DashboardSnapshot, monitor::AccountMonitor};
use std::sync::Arc;
use tokio::sync::watch;

pub struct AppState {
    pub monitor: Arc<AccountMonitor>,
    pub shutdown: watch::Sender<bool>,
}

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
    state
        .monitor
        .refresh()
        .await
        .map_err(|error| error.to_string())
}
