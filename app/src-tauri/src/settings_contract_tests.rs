use crate::settings::{AppSettings, PaceMode, ThemePreference};
use crate::storage::repository::AccountRepository;
use std::time::Duration;

#[test]
fn desktop_capability_includes_the_tray_webview() {
    let capability: serde_json::Value =
        serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
    let windows = capability["windows"].as_array().unwrap();
    assert!(windows.iter().any(|window| window == "main"));
    assert!(windows.iter().any(|window| window == "tray-panel"));
}

#[test]
fn settings_defaults_match_the_product_contract() {
    let settings = AppSettings::default();
    assert_eq!(settings.theme, ThemePreference::System);
    assert_eq!(settings.pace_mode, PaceMode::Suggested);
    assert_eq!(settings.account_refresh_mins, 1);
    assert_eq!(settings.session_scan_mins, 10);
    assert_eq!(settings.monthly_subscription_usd, 20.0);
    assert!(!settings.launch_at_login);
    assert_eq!(settings.warning_remaining_percent, 25);
    assert_eq!(settings.critical_remaining_percent, 10);
}

#[test]
fn older_saved_settings_get_the_default_subscription_price() {
    let mut value = serde_json::to_value(AppSettings::default()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove("monthlySubscriptionUsd");
    let settings: AppSettings = serde_json::from_value(value).unwrap();
    assert_eq!(settings.monthly_subscription_usd, 20.0);
}

#[test]
fn settings_reject_inverted_notification_thresholds() {
    let settings = AppSettings {
        warning_remaining_percent: 10,
        critical_remaining_percent: 25,
        ..AppSettings::default()
    };
    assert_eq!(settings.validate().unwrap_err(), "紧急阈值必须低于提醒阈值");
}

#[test]
fn disabled_thresholds_do_not_block_other_preferences() {
    let settings = AppSettings {
        quota_warning_enabled: false,
        warning_remaining_percent: 5,
        critical_remaining_percent: 10,
        ..AppSettings::default()
    };
    assert!(settings.validate().is_ok());
}

#[test]
fn settings_expose_runtime_refresh_durations() {
    let settings = AppSettings {
        account_refresh_mins: 5,
        session_scan_mins: 30,
        ..AppSettings::default()
    };
    assert_eq!(
        settings.account_refresh_duration(),
        Duration::from_secs(300)
    );
    assert_eq!(settings.session_scan_duration(), Duration::from_secs(1_800));
}

#[test]
fn settings_and_notification_delivery_round_trip_through_storage() {
    let repository = AccountRepository::open_in_memory().unwrap();
    let settings = AppSettings {
        theme: ThemePreference::Dark,
        account_refresh_mins: 5,
        ..AppSettings::default()
    };
    repository.save_settings(&settings).unwrap();
    assert_eq!(repository.load_settings().unwrap(), settings);

    assert!(!repository
        .notification_was_delivered("cycle:warning")
        .unwrap());
    repository
        .mark_notification_delivered("cycle:warning", 1_000)
        .unwrap();
    assert!(repository
        .notification_was_delivered("cycle:warning")
        .unwrap());
}
