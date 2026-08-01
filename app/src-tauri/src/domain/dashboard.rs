use super::account::{AccountUsageResult, DailyUsageBucket};
use super::session::LocalSessionView;
use crate::forecast::{ExhaustionForecast, ForecastStatus};
use crate::quota_trend::{QuotaHistoryPoint, QuotaPace};
use std::collections::HashSet;

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuotaView {
    pub limit_id: String,
    pub label: String,
    pub window_kind: String,
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub window_duration_mins: i64,
    pub resets_at: i64,
    pub plan_type: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountUsageView {
    pub lifetime_tokens: Option<i64>,
    pub peak_daily_tokens: Option<i64>,
    pub daily_usage_buckets: Vec<DailyUsageBucket>,
}

impl From<AccountUsageResult> for AccountUsageView {
    fn from(value: AccountUsageResult) -> Self {
        let summary = value.summary;
        Self {
            lifetime_tokens: summary.as_ref().and_then(|item| item.lifetime_tokens),
            peak_daily_tokens: summary.as_ref().and_then(|item| item.peak_daily_tokens),
            daily_usage_buckets: value.daily_usage_buckets.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub observed_at: i64,
    pub is_stale: bool,
    pub connection_error: Option<String>,
    pub account_usage_error: Option<String>,
    pub primary_quota: Option<QuotaView>,
    pub other_quotas: Vec<QuotaView>,
    pub account_usage: Option<AccountUsageView>,
    pub forecast: Option<ExhaustionForecast>,
    pub quota_history: Vec<QuotaHistoryPoint>,
    pub quota_pace: Option<QuotaPace>,
    pub local_sessions: LocalSessionView,
}

impl Default for DashboardSnapshot {
    fn default() -> Self {
        Self {
            observed_at: 0,
            is_stale: true,
            connection_error: None,
            account_usage_error: None,
            primary_quota: None,
            other_quotas: Vec::new(),
            account_usage: None,
            forecast: None,
            quota_history: Vec::new(),
            quota_pace: None,
            local_sessions: LocalSessionView::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrayPanelSnapshot {
    pub observed_at: i64,
    pub is_stale: bool,
    pub connection_error: Option<String>,
    pub primary_quota: Option<QuotaView>,
    pub forecast_status: Option<ForecastStatus>,
    pub latest_daily_tokens: Option<i64>,
    pub project_count: usize,
    pub session_count: usize,
}

impl From<&DashboardSnapshot> for TrayPanelSnapshot {
    fn from(snapshot: &DashboardSnapshot) -> Self {
        let project_count = snapshot
            .local_sessions
            .sessions
            .iter()
            .map(|session| normalized_project_key(session.project_path.as_deref()))
            .collect::<HashSet<_>>()
            .len();
        let latest_daily_tokens = snapshot.account_usage.as_ref().and_then(|usage| {
            usage
                .daily_usage_buckets
                .iter()
                .max_by_key(|bucket| &bucket.start_date)
                .map(|bucket| bucket.tokens)
        });

        Self {
            observed_at: snapshot.observed_at,
            is_stale: snapshot.is_stale,
            connection_error: snapshot.connection_error.clone(),
            primary_quota: snapshot.primary_quota.clone(),
            forecast_status: snapshot.forecast.as_ref().map(|forecast| forecast.status),
            latest_daily_tokens,
            project_count,
            session_count: snapshot.local_sessions.sessions.len(),
        }
    }
}

fn normalized_project_key(project_path: Option<&str>) -> String {
    project_path
        .map(|path| path.replace('\\', "/"))
        .map(|path| path.trim_end_matches('/').to_lowercase())
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| "__unknown__".to_string())
}

#[cfg(test)]
mod tray_panel_snapshot_tests {
    use super::*;
    use crate::domain::session::{SessionSummary, TokenBreakdown};

    fn session(id: usize, path: Option<&str>) -> SessionSummary {
        SessionSummary {
            session_id: format!("session-{id}"),
            title: format!("Session {id}"),
            project_path: path.map(str::to_string),
            last_active_at: id as i64,
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
        }
    }

    #[test]
    fn compacts_dashboard_data_without_serializing_session_details() {
        let mut snapshot = DashboardSnapshot::default();
        snapshot.local_sessions.sessions = (0..1_000)
            .map(|index| session(index, Some("E:\\Research\\vispfn")))
            .collect();

        let compact = TrayPanelSnapshot::from(&snapshot);
        let compact_json = serde_json::to_string(&compact).unwrap();
        let full_json = serde_json::to_string(&snapshot).unwrap();

        assert_eq!(compact.project_count, 1);
        assert_eq!(compact.session_count, 1_000);
        assert!(!compact_json.contains("session-999"));
        assert!(compact_json.len() * 20 < full_json.len());
    }

    #[test]
    fn normalizes_windows_paths_and_uses_latest_daily_bucket() {
        let mut snapshot = DashboardSnapshot::default();
        snapshot.local_sessions.sessions = vec![
            session(1, Some("E:\\Research\\vispfn\\")),
            session(2, Some("e:/research/VISPFN")),
            session(3, None),
        ];
        snapshot.account_usage = Some(AccountUsageView {
            lifetime_tokens: None,
            peak_daily_tokens: None,
            daily_usage_buckets: vec![
                DailyUsageBucket {
                    start_date: "2026-08-01".into(),
                    tokens: 80,
                },
                DailyUsageBucket {
                    start_date: "2026-07-31".into(),
                    tokens: 70,
                },
            ],
        });

        let compact = TrayPanelSnapshot::from(&snapshot);
        assert_eq!(compact.project_count, 2);
        assert_eq!(compact.latest_daily_tokens, Some(80));
    }
}
