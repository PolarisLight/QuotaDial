use super::account::{AccountUsageResult, DailyUsageBucket};
use super::session::LocalSessionView;
use crate::forecast::ExhaustionForecast;

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
            local_sessions: LocalSessionView::default(),
        }
    }
}
