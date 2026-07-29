use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::watch;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PaceMode {
    Suggested,
    RecentRate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: ThemePreference,
    pub pace_mode: PaceMode,
    pub account_refresh_mins: u64,
    pub session_scan_mins: u64,
    #[serde(default = "default_monthly_subscription_usd")]
    pub monthly_subscription_usd: f64,
    pub launch_at_login: bool,
    pub quota_warning_enabled: bool,
    pub warning_remaining_percent: u8,
    pub quota_critical_enabled: bool,
    pub critical_remaining_percent: u8,
    pub reset_notification_enabled: bool,
    pub stale_notification_enabled: bool,
    pub stale_after_mins: u64,
}

fn default_monthly_subscription_usd() -> f64 {
    20.0
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            pace_mode: PaceMode::Suggested,
            account_refresh_mins: 1,
            session_scan_mins: 10,
            monthly_subscription_usd: default_monthly_subscription_usd(),
            launch_at_login: false,
            quota_warning_enabled: true,
            warning_remaining_percent: 25,
            quota_critical_enabled: true,
            critical_remaining_percent: 10,
            reset_notification_enabled: true,
            stale_notification_enabled: true,
            stale_after_mins: 15,
        }
    }
}

impl AppSettings {
    pub fn account_refresh_duration(&self) -> Duration {
        Duration::from_secs(self.account_refresh_mins * 60)
    }

    pub fn session_scan_duration(&self) -> Duration {
        Duration::from_secs(self.session_scan_mins * 60)
    }

    pub fn validate(&self) -> Result<(), String> {
        if ![1, 5, 15].contains(&self.account_refresh_mins) {
            return Err("账号刷新间隔无效".into());
        }
        if ![5, 10, 30].contains(&self.session_scan_mins) {
            return Err("会话扫描间隔无效".into());
        }
        if !self.monthly_subscription_usd.is_finite()
            || self.monthly_subscription_usd <= 0.0
            || self.monthly_subscription_usd > 10_000.0
        {
            return Err("月订阅价格无效".into());
        }
        if self.quota_warning_enabled
            && self.quota_critical_enabled
            && self.critical_remaining_percent >= self.warning_remaining_percent
        {
            return Err("紧急阈值必须低于提醒阈值".into());
        }
        if self.warning_remaining_percent > 100
            || self.critical_remaining_percent > 100
            || self.stale_after_mins == 0
        {
            return Err("通知设置无效".into());
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct SettingsRuntime {
    sender: watch::Sender<AppSettings>,
}

impl SettingsRuntime {
    pub fn new(initial: AppSettings) -> Self {
        let (sender, _) = watch::channel(initial);
        Self { sender }
    }

    pub fn current(&self) -> AppSettings {
        self.sender.borrow().clone()
    }

    pub fn update(&self, value: AppSettings) {
        self.sender.send_replace(value);
    }

    pub fn subscribe(&self) -> watch::Receiver<AppSettings> {
        self.sender.subscribe()
    }
}
