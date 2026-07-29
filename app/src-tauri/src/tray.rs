use crate::monitor::AccountMonitor;
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Manager,
};

pub fn tray_title(remaining_percent: Option<f64>, stale: bool) -> String {
    match remaining_percent {
        Some(remaining) if stale => format!("{remaining:.0}%?"),
        Some(remaining) => format!("{remaining:.0}%"),
        None if stale => "Codex?".into(),
        None => "Codex".into(),
    }
}

pub fn build(app: &AppHandle, monitor: Arc<AccountMonitor>) -> tauri::Result<TrayIcon> {
    let show = MenuItem::with_id(app, "show", "打开仪表盘", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "刷新", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 Codex Monitor", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &refresh, &quit])?;
    let menu_monitor = monitor.clone();

    let tray = TrayIconBuilder::with_id("codex-monitor")
        .icon(Image::from_bytes(include_bytes!("../icons/32x32.png"))?)
        .icon_as_template(true)
        .tooltip("Codex Monitor")
        .title(tray_title(None, false))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.center();
                    let _ = window.set_focus();
                }
            }
            "refresh" => {
                let monitor = menu_monitor.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = monitor.refresh().await;
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    let tray_updates = tray.clone();
    let mut snapshots = monitor.subscribe();
    tauri::async_runtime::spawn(async move {
        while snapshots.changed().await.is_ok() {
            let snapshot = snapshots.borrow().clone();
            let remaining = snapshot
                .primary_quota
                .as_ref()
                .map(|quota| quota.remaining_percent);
            let title = tray_title(remaining, snapshot.is_stale);
            let tooltip = match remaining {
                Some(value) => format!("Codex Monitor · 剩余 {value:.0}%"),
                None => "Codex Monitor · 额度暂不可用".into(),
            };
            let _ = tray_updates.set_title(Some(title));
            let _ = tray_updates.set_tooltip(Some(tooltip));
        }
    });

    Ok(tray)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
