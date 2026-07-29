use crate::{domain::dashboard::DashboardSnapshot, settings::AppSettings};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationEvent {
    pub key: String,
    pub title: String,
    pub body: String,
}

#[derive(Default)]
pub struct NotificationTracker {
    previous_reset_at: Option<i64>,
    stale_active: bool,
}

impl NotificationTracker {
    pub fn evaluate(
        &mut self,
        snapshot: &DashboardSnapshot,
        settings: &AppSettings,
        now: i64,
    ) -> Vec<NotificationEvent> {
        let mut events = Vec::new();
        if let Some(quota) = snapshot.primary_quota.as_ref() {
            if settings.quota_critical_enabled
                && quota.remaining_percent <= f64::from(settings.critical_remaining_percent)
            {
                events.push(NotificationEvent {
                    key: format!("quota:{}:critical", quota.resets_at),
                    title: "Codex 额度紧急".into(),
                    body: format!("7 日额度仅剩 {:.0}%", quota.remaining_percent),
                });
            } else if settings.quota_warning_enabled
                && quota.remaining_percent <= f64::from(settings.warning_remaining_percent)
            {
                events.push(NotificationEvent {
                    key: format!("quota:{}:warning", quota.resets_at),
                    title: "Codex 额度提醒".into(),
                    body: format!("7 日额度剩余 {:.0}%", quota.remaining_percent),
                });
            }

            if settings.reset_notification_enabled
                && self
                    .previous_reset_at
                    .is_some_and(|previous| previous != quota.resets_at)
            {
                events.push(NotificationEvent {
                    key: format!("quota:{}:reset", quota.resets_at),
                    title: "Codex 额度已重置".into(),
                    body: "新的额度周期已经开始。".into(),
                });
            }
            self.previous_reset_at = Some(quota.resets_at);
        }

        let stale = snapshot.observed_at == 0
            || now - snapshot.observed_at >= settings.stale_after_mins as i64 * 60;
        if settings.stale_notification_enabled && stale && !self.stale_active {
            events.push(NotificationEvent {
                key: format!("stale:{}", snapshot.observed_at),
                title: "Codex 数据未更新".into(),
                body: format!(
                    "账号额度已超过 {} 分钟未成功刷新。",
                    settings.stale_after_mins
                ),
            });
        }
        self.stale_active = stale;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dashboard::QuotaView;

    fn snapshot(remaining: f64, resets_at: i64, observed_at: i64) -> DashboardSnapshot {
        DashboardSnapshot {
            observed_at,
            primary_quota: Some(QuotaView {
                limit_id: "codex".into(),
                label: "7 日额度".into(),
                window_kind: "primary".into(),
                used_percent: 100.0 - remaining,
                remaining_percent: remaining,
                window_duration_mins: 10_080,
                resets_at,
                plan_type: None,
            }),
            ..DashboardSnapshot::default()
        }
    }

    #[test]
    fn critical_threshold_wins_over_warning_and_is_keyed_to_cycle() {
        let mut tracker = NotificationTracker::default();
        let events = tracker.evaluate(&snapshot(8.0, 2_000, 1_000), &AppSettings::default(), 1_000);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, "quota:2000:critical");
    }

    #[test]
    fn reset_and_stale_are_emitted_once_per_transition() {
        let mut tracker = NotificationTracker::default();
        let settings = AppSettings::default();
        tracker.evaluate(&snapshot(80.0, 2_000, 1_000), &settings, 1_000);
        let reset = tracker.evaluate(&snapshot(100.0, 3_000, 1_100), &settings, 1_100);
        assert!(reset.iter().any(|event| event.key == "quota:3000:reset"));

        let stale = tracker.evaluate(&snapshot(100.0, 3_000, 1_100), &settings, 2_001);
        assert!(stale.iter().any(|event| event.key == "stale:1100"));
        let repeated = tracker.evaluate(&snapshot(100.0, 3_000, 1_100), &settings, 2_100);
        assert!(repeated.iter().all(|event| event.key != "stale:1100"));
    }
}
