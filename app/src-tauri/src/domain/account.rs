use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RateWindow {
    pub used_percent: f64,
    pub window_duration_mins: i64,
    pub resets_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitBucket {
    pub limit_id: String,
    pub limit_name: Option<String>,
    pub primary: Option<RateWindow>,
    pub secondary: Option<RateWindow>,
    pub rate_limit_reached_type: Option<String>,
    pub plan_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResetCredits {
    pub available_count: i64,
    pub credits: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitsResult {
    pub rate_limits: Option<RateLimitBucket>,
    #[serde(default, rename = "rateLimitsByLimitId")]
    pub by_limit_id: BTreeMap<String, RateLimitBucket>,
    #[serde(rename = "rateLimitResetCredits")]
    pub reset_credits: Option<ResetCredits>,
}

impl RateLimitsResult {
    pub fn buckets(&self) -> BTreeMap<String, RateLimitBucket> {
        if !self.by_limit_id.is_empty() {
            return self.by_limit_id.clone();
        }

        self.rate_limits
            .clone()
            .map(|bucket| BTreeMap::from([(bucket.limit_id.clone(), bucket)]))
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub lifetime_tokens: Option<i64>,
    pub peak_daily_tokens: Option<i64>,
    pub longest_running_turn_sec: Option<i64>,
    pub current_streak_days: Option<i64>,
    pub longest_streak_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsageBucket {
    pub start_date: String,
    pub tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountUsageResult {
    pub summary: Option<UsageSummary>,
    pub daily_usage_buckets: Option<Vec<DailyUsageBucket>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multi_bucket_rate_limits() {
        let json = r#"{
          "rateLimits": {
            "limitId": "codex",
            "limitName": null,
            "primary": {
              "usedPercent": 18,
              "windowDurationMins": 10080,
              "resetsAt": 1785903626
            },
            "secondary": null,
            "rateLimitReachedType": null
          },
          "rateLimitsByLimitId": {
            "codex": {
              "limitId": "codex",
              "limitName": null,
              "primary": {
                "usedPercent": 18,
                "windowDurationMins": 10080,
                "resetsAt": 1785903626
              },
              "secondary": null,
              "rateLimitReachedType": null
            }
          },
          "rateLimitResetCredits": { "availableCount": 1, "credits": null }
        }"#;
        let parsed: RateLimitsResult = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed.by_limit_id["codex"]
                .primary
                .as_ref()
                .unwrap()
                .used_percent,
            18.0
        );
        assert_eq!(parsed.reset_credits.unwrap().available_count, 1);
    }

    #[test]
    fn parses_nullable_account_usage() {
        let json = r#"{
          "summary": { "lifetimeTokens": null, "peakDailyTokens": 45678 },
          "dailyUsageBuckets": [
            { "startDate": "2026-07-28", "tokens": 12345 }
          ]
        }"#;
        let parsed: AccountUsageResult = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.summary.unwrap().lifetime_tokens, None);
        assert_eq!(parsed.daily_usage_buckets.unwrap()[0].tokens, 12345);
    }

    #[test]
    fn falls_back_to_legacy_rate_limit_bucket() {
        let json = r#"{
          "rateLimits": {
            "limitId": "codex",
            "limitName": null,
            "primary": null,
            "secondary": null,
            "rateLimitReachedType": null
          }
        }"#;
        let parsed: RateLimitsResult = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.buckets()["codex"].limit_id, "codex");
    }
}
