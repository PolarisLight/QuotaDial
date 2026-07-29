use super::migrations;
use crate::{
    domain::account::{AccountUsageResult, RateLimitsResult, RateWindow},
    error::AppError,
    sessions::parser::{ParsedFile, ParsedSessionMetadata},
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

#[derive(Debug, Clone, PartialEq)]
pub struct SourceFileState {
    pub path: String,
    pub generation: i64,
    pub file_identity: Option<String>,
    pub byte_offset: i64,
    pub observed_size: i64,
    pub modified_at: i64,
    pub parser_version: i64,
    pub session_id: Option<String>,
    pub current_model: Option<String>,
    pub last_error: Option<String>,
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

    pub fn latest_source_state(&self, path: &str) -> Result<Option<SourceFileState>, AppError> {
        self.lock()?
            .query_row(
                "SELECT path, generation, file_identity, byte_offset, observed_size,
                        modified_at, parser_version, session_id, current_model, last_error
                 FROM session_source_files
                 WHERE path = ?1
                 ORDER BY generation DESC
                 LIMIT 1",
                [path],
                |row| {
                    Ok(SourceFileState {
                        path: row.get(0)?,
                        generation: row.get(1)?,
                        file_identity: row.get(2)?,
                        byte_offset: row.get(3)?,
                        observed_size: row.get(4)?,
                        modified_at: row.get(5)?,
                        parser_version: row.get(6)?,
                        session_id: row.get(7)?,
                        current_model: row.get(8)?,
                        last_error: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(AppError::from)
    }

    pub fn import_session_file(
        &self,
        state: &SourceFileState,
        parsed: &ParsedFile,
    ) -> Result<i64, AppError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        if let Some(metadata) = parsed.metadata.as_ref() {
            upsert_session_metadata(&transaction, metadata)?;
        }

        let mut inserted = 0_i64;
        for event in &parsed.events {
            inserted += transaction.execute(
                "INSERT OR IGNORE INTO session_usage_events(
                    fingerprint, session_id, occurred_at, model,
                    input_tokens, cached_input_tokens, output_tokens,
                    reasoning_output_tokens, source_path, source_offset
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    event.fingerprint,
                    event.session_id,
                    event.occurred_at,
                    event.model,
                    event.tokens.input_tokens,
                    event.tokens.cached_input_tokens,
                    event.tokens.output_tokens,
                    event.tokens.reasoning_output_tokens,
                    event.source_path,
                    event.source_offset as i64,
                ],
            )? as i64;
        }

        transaction.execute(
            "INSERT INTO session_source_files(
                path, generation, file_identity, byte_offset, observed_size,
                modified_at, parser_version, session_id, current_model, last_error
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(path, generation) DO UPDATE SET
                file_identity = excluded.file_identity,
                byte_offset = excluded.byte_offset,
                observed_size = excluded.observed_size,
                modified_at = excluded.modified_at,
                parser_version = excluded.parser_version,
                session_id = excluded.session_id,
                current_model = excluded.current_model,
                last_error = excluded.last_error",
            params![
                state.path,
                state.generation,
                state.file_identity,
                state.byte_offset,
                state.observed_size,
                state.modified_at,
                state.parser_version,
                state.session_id,
                state.current_model,
                state.last_error,
            ],
        )?;
        transaction.commit()?;
        Ok(inserted)
    }

    pub fn session_event_count(&self) -> Result<i64, AppError> {
        self.lock()?
            .query_row("SELECT COUNT(*) FROM session_usage_events", [], |row| {
                row.get(0)
            })
            .map_err(AppError::from)
    }

    pub fn source_generation_count(&self) -> Result<i64, AppError> {
        self.lock()?
            .query_row("SELECT COUNT(*) FROM session_source_files", [], |row| {
                row.get(0)
            })
            .map_err(AppError::from)
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, AppError> {
        self.connection
            .lock()
            .map_err(|_| AppError::Unavailable("account database lock was poisoned".into()))
    }
}

fn upsert_session_metadata(
    transaction: &rusqlite::Transaction<'_>,
    metadata: &ParsedSessionMetadata,
) -> Result<(), AppError> {
    transaction.execute(
        "INSERT INTO session_metadata(
            session_id, parent_session_id, started_at, last_active_at,
            cwd, model, source_path
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(session_id) DO UPDATE SET
            parent_session_id = COALESCE(excluded.parent_session_id, parent_session_id),
            started_at = MIN(started_at, excluded.started_at),
            last_active_at = MAX(last_active_at, excluded.last_active_at),
            cwd = COALESCE(excluded.cwd, cwd),
            model = COALESCE(excluded.model, model),
            source_path = excluded.source_path",
        params![
            metadata.session_id,
            metadata.parent_session_id,
            metadata.started_at,
            metadata.last_active_at,
            metadata.cwd,
            metadata.model,
            metadata.source_path,
        ],
    )?;
    Ok(())
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
