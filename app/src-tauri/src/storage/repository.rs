use super::migrations;
use crate::{
    domain::account::{AccountUsageResult, RateLimitsResult, RateWindow},
    error::AppError,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

#[derive(Debug, Clone, PartialEq)]
pub struct RateObservation {
    pub observed_at: i64,
    pub limit_id: String,
    pub window_kind: String,
    pub used_percent: f64,
    pub window_duration_mins: i64,
    pub resets_at: i64,
}

pub struct AccountRepository {
    connection: Mutex<Connection>,
}

impl AccountRepository {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        let mut connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrations::run(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn open_in_memory() -> Result<Self, AppError> {
        let mut connection = Connection::open_in_memory()?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrations::run(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn insert_rate_limits(
        &self,
        observed_at: i64,
        value: &RateLimitsResult,
        raw: &serde_json::Value,
    ) -> Result<(), AppError> {
        let payload = serde_json::to_string(raw)?;
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;

        for (limit_id, bucket) in value.buckets() {
            for (window_kind, window) in [
                ("primary", bucket.primary.as_ref()),
                ("secondary", bucket.secondary.as_ref()),
            ] {
                let Some(window) = window else {
                    continue;
                };
                insert_rate_window(
                    &transaction,
                    observed_at,
                    &limit_id,
                    window_kind,
                    window,
                    bucket.plan_type.as_deref(),
                    &payload,
                )?;
            }
        }

        transaction.commit()?;
        Ok(())
    }

    pub fn insert_account_usage(
        &self,
        observed_at: i64,
        value: &AccountUsageResult,
        raw: &serde_json::Value,
    ) -> Result<(), AppError> {
        let summary = value.summary.as_ref();
        let daily_buckets = value
            .daily_usage_buckets
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        self.lock()?.execute(
            "INSERT OR IGNORE INTO account_usage_observations(
                observed_at,
                lifetime_tokens,
                peak_daily_tokens,
                daily_buckets_json,
                payload_json
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                observed_at,
                summary.and_then(|item| item.lifetime_tokens),
                summary.and_then(|item| item.peak_daily_tokens),
                daily_buckets,
                serde_json::to_string(raw)?
            ],
        )?;
        Ok(())
    }

    pub fn current_segment(
        &self,
        limit_id: &str,
        window_kind: &str,
    ) -> Result<Vec<RateObservation>, AppError> {
        let connection = self.lock()?;
        let resets_at = connection
            .query_row(
                "SELECT resets_at
                 FROM account_rate_limit_observations
                 WHERE limit_id = ?1 AND window_kind = ?2
                 ORDER BY observed_at DESC
                 LIMIT 1",
                params![limit_id, window_kind],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        let Some(resets_at) = resets_at else {
            return Ok(Vec::new());
        };

        let mut statement = connection.prepare(
            "SELECT observed_at, limit_id, window_kind, used_percent,
                    window_duration_mins, resets_at
             FROM account_rate_limit_observations
             WHERE limit_id = ?1 AND window_kind = ?2 AND resets_at = ?3
             ORDER BY observed_at ASC",
        )?;
        let rows = statement.query_map(params![limit_id, window_kind, resets_at], |row| {
            Ok(RateObservation {
                observed_at: row.get(0)?,
                limit_id: row.get(1)?,
                window_kind: row.get(2)?,
                used_percent: row.get(3)?,
                window_duration_mins: row.get(4)?,
                resets_at: row.get(5)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, AppError> {
        self.connection
            .lock()
            .map_err(|_| AppError::Unavailable("account database lock was poisoned".into()))
    }
}

fn insert_rate_window(
    transaction: &rusqlite::Transaction<'_>,
    observed_at: i64,
    limit_id: &str,
    window_kind: &str,
    window: &RateWindow,
    plan_type: Option<&str>,
    payload: &str,
) -> Result<(), AppError> {
    transaction.execute(
        "INSERT OR IGNORE INTO account_rate_limit_observations(
            observed_at,
            limit_id,
            window_kind,
            used_percent,
            window_duration_mins,
            resets_at,
            plan_type,
            payload_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            observed_at,
            limit_id,
            window_kind,
            window.used_percent,
            window.window_duration_mins,
            window.resets_at,
            plan_type,
            payload
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::{AccountUsageResult, RateLimitsResult};

    fn rate_limits(resets_at: i64, used_percent: f64) -> (RateLimitsResult, serde_json::Value) {
        let raw = serde_json::json!({
            "rateLimitsByLimitId": {
                "codex": {
                    "limitId": "codex",
                    "limitName": null,
                    "primary": {
                        "usedPercent": used_percent,
                        "windowDurationMins": 10080,
                        "resetsAt": resets_at
                    },
                    "secondary": null,
                    "rateLimitReachedType": null
                }
            }
        });
        (serde_json::from_value(raw.clone()).unwrap(), raw)
    }

    #[test]
    fn inserting_same_observation_twice_is_idempotent() {
        let repository = AccountRepository::open_in_memory().unwrap();
        let (value, raw) = rate_limits(20_000, 12.0);

        repository.insert_rate_limits(1_000, &value, &raw).unwrap();
        repository.insert_rate_limits(1_000, &value, &raw).unwrap();

        let count: i64 = repository
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM account_rate_limit_observations",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn loads_only_current_forecast_segment() {
        let repository = AccountRepository::open_in_memory().unwrap();
        for (observed_at, resets_at, used_percent) in [
            (1_000, 20_000, 10.0),
            (2_000, 20_000, 12.0),
            (3_000, 30_000, 1.0),
            (4_000, 30_000, 3.0),
        ] {
            let (value, raw) = rate_limits(resets_at, used_percent);
            repository
                .insert_rate_limits(observed_at, &value, &raw)
                .unwrap();
        }

        let segment = repository.current_segment("codex", "primary").unwrap();
        assert_eq!(
            segment
                .iter()
                .map(|point| point.observed_at)
                .collect::<Vec<_>>(),
            vec![3_000, 4_000]
        );
    }

    #[test]
    fn stores_nullable_account_usage_fields() {
        let repository = AccountRepository::open_in_memory().unwrap();
        let raw = serde_json::json!({
            "summary": { "lifetimeTokens": null, "peakDailyTokens": 45678 },
            "dailyUsageBuckets": null
        });
        let value: AccountUsageResult = serde_json::from_value(raw.clone()).unwrap();

        repository
            .insert_account_usage(1_000, &value, &raw)
            .unwrap();

        let (lifetime, buckets): (Option<i64>, Option<String>) = repository
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT lifetime_tokens, daily_buckets_json FROM account_usage_observations",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(lifetime, None);
        assert_eq!(buckets, None);
    }
}
