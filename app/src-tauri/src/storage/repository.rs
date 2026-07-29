use super::migrations;
use crate::{
    domain::{
        account::{AccountUsageResult, RateLimitsResult, RateWindow},
        session::{LocalSessionView, SessionDiagnostics, SessionSummary, TokenBreakdown},
    },
    error::AppError,
    sessions::{
        parser::{ParsedFile, ParsedSessionMetadata},
        pricing::PriceCatalog,
    },
};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    collections::{HashMap, HashSet},
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

    pub fn local_session_view(&self, now: i64) -> Result<LocalSessionView, AppError> {
        let connection = self.lock()?;
        let metadata = load_session_metadata(&connection)?;
        let events = load_session_events(&connection)?;
        let parents = metadata
            .iter()
            .map(|(id, item)| (id.clone(), item.parent_session_id.clone()))
            .collect::<HashMap<_, _>>();
        let catalog = PriceCatalog::built_in();
        let mut groups: HashMap<String, SessionAccumulator> = HashMap::new();

        for (session_id, item) in &metadata {
            let root_id = resolve_root(session_id, &parents);
            let group = groups
                .entry(root_id.clone())
                .or_insert_with(|| SessionAccumulator::new(root_id));
            group.last_active_at = group.last_active_at.max(item.last_active_at);
            if session_id != &group.root_id {
                group.child_ids.insert(session_id.clone());
            }
        }

        for event in events {
            let root_id = resolve_root(&event.session_id, &parents);
            let group = groups
                .entry(root_id.clone())
                .or_insert_with(|| SessionAccumulator::new(root_id));
            group.tokens.input_tokens += event.tokens.input_tokens;
            group.tokens.cached_input_tokens += event.tokens.cached_input_tokens;
            group.tokens.output_tokens += event.tokens.output_tokens;
            group.tokens.reasoning_output_tokens += event.tokens.reasoning_output_tokens;
            group.last_active_at = group.last_active_at.max(event.occurred_at);
            if event.session_id != group.root_id {
                group.child_ids.insert(event.session_id.clone());
            }
            if let Some(model) = event.model.as_ref() {
                *group.model_weights.entry(model.clone()).or_default() += event.tokens.total();
                match catalog.estimate(model, event.occurred_at, &event.tokens) {
                    Some(cost) => group.cost_usd += cost,
                    None => group.has_unknown_cost = true,
                }
            } else {
                group.has_unknown_cost = true;
            }
        }

        let mut sessions = groups
            .into_values()
            .map(|group| {
                let root_metadata = metadata.get(&group.root_id);
                let project_path = root_metadata.and_then(|item| item.cwd.clone());
                let started_at = root_metadata
                    .map(|item| item.started_at)
                    .unwrap_or(group.last_active_at);
                let primary_model = group
                    .model_weights
                    .into_iter()
                    .max_by_key(|(_, weight)| *weight)
                    .map(|(model, _)| model);
                SessionSummary {
                    session_id: group.root_id,
                    title: session_title(project_path.as_deref(), started_at),
                    project_path,
                    last_active_at: group.last_active_at,
                    primary_model,
                    tokens: group.tokens,
                    equivalent_cost_usd: (!group.has_unknown_cost).then_some(group.cost_usd),
                    child_session_count: group.child_ids.len() as i64,
                }
            })
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| {
            right
                .last_active_at
                .cmp(&left.last_active_at)
                .then_with(|| left.session_id.cmp(&right.session_id))
        });

        let skipped_lines = connection.query_row(
            "SELECT COUNT(*) FROM session_source_files WHERE last_error IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        let scanned_files = connection.query_row(
            "SELECT COUNT(DISTINCT path) FROM session_source_files",
            [],
            |row| row.get(0),
        )?;
        Ok(LocalSessionView {
            sessions,
            diagnostics: SessionDiagnostics {
                scanned_files,
                skipped_lines,
                last_imported_at: Some(now),
                last_error: None,
            },
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, AppError> {
        self.connection
            .lock()
            .map_err(|_| AppError::Unavailable("account database lock was poisoned".into()))
    }
}

#[derive(Debug)]
struct SessionMetadataRow {
    parent_session_id: Option<String>,
    started_at: i64,
    last_active_at: i64,
    cwd: Option<String>,
}

#[derive(Debug)]
struct SessionEventRow {
    session_id: String,
    occurred_at: i64,
    model: Option<String>,
    tokens: TokenBreakdown,
}

struct SessionAccumulator {
    root_id: String,
    last_active_at: i64,
    tokens: TokenBreakdown,
    child_ids: HashSet<String>,
    model_weights: HashMap<String, i64>,
    cost_usd: f64,
    has_unknown_cost: bool,
}

impl SessionAccumulator {
    fn new(root_id: String) -> Self {
        Self {
            root_id,
            last_active_at: 0,
            tokens: TokenBreakdown::default(),
            child_ids: HashSet::new(),
            model_weights: HashMap::new(),
            cost_usd: 0.0,
            has_unknown_cost: false,
        }
    }
}

fn load_session_metadata(
    connection: &Connection,
) -> Result<HashMap<String, SessionMetadataRow>, AppError> {
    let mut statement = connection.prepare(
        "SELECT session_id, parent_session_id, started_at, last_active_at, cwd
         FROM session_metadata",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            SessionMetadataRow {
                parent_session_id: row.get(1)?,
                started_at: row.get(2)?,
                last_active_at: row.get(3)?,
                cwd: row.get(4)?,
            },
        ))
    })?;
    rows.collect::<Result<HashMap<_, _>, _>>()
        .map_err(AppError::from)
}

fn load_session_events(connection: &Connection) -> Result<Vec<SessionEventRow>, AppError> {
    let mut statement = connection.prepare(
        "SELECT session_id, occurred_at, model, input_tokens, cached_input_tokens,
                output_tokens, reasoning_output_tokens
         FROM session_usage_events",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(SessionEventRow {
            session_id: row.get(0)?,
            occurred_at: row.get(1)?,
            model: row.get(2)?,
            tokens: TokenBreakdown {
                input_tokens: row.get(3)?,
                cached_input_tokens: row.get(4)?,
                output_tokens: row.get(5)?,
                reasoning_output_tokens: row.get(6)?,
            },
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

fn resolve_root(session_id: &str, parents: &HashMap<String, Option<String>>) -> String {
    let mut current = session_id.to_owned();
    let mut visited = HashSet::new();
    while visited.insert(current.clone()) {
        let Some(Some(parent)) = parents.get(&current) else {
            break;
        };
        if !parents.contains_key(parent) {
            break;
        }
        current = parent.clone();
    }
    current
}

fn session_title(project_path: Option<&str>, started_at: i64) -> String {
    let project = project_path
        .and_then(|path| Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Codex");
    let date = chrono::DateTime::<chrono::Utc>::from_timestamp(started_at, 0)
        .map(|value| {
            value
                .with_timezone(&chrono::Local)
                .format("%-m月%-d日")
                .to_string()
        })
        .unwrap_or_else(|| "未知日期".to_owned());
    format!("{project} · {date}")
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
    use crate::sessions::parser::{parse_reader, PARSER_VERSION};

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

    fn import_fixture(repository: &AccountRepository, bytes: &[u8], path: &str) {
        let parsed = parse_reader(bytes, path, 0).unwrap();
        let state = SourceFileState {
            path: path.to_owned(),
            generation: 0,
            file_identity: None,
            byte_offset: parsed.next_offset as i64,
            observed_size: parsed.next_offset as i64,
            modified_at: 0,
            parser_version: PARSER_VERSION,
            session_id: parsed.current_session_id.clone(),
            current_model: parsed.current_model.clone(),
            last_error: None,
        };
        repository.import_session_file(&state, &parsed).unwrap();
    }

    #[test]
    fn rolls_child_usage_into_one_top_level_session_row() {
        let repository = AccountRepository::open_in_memory().unwrap();
        import_fixture(
            &repository,
            include_bytes!("../../tests/fixtures/sessions/root.jsonl"),
            "root.jsonl",
        );
        import_fixture(
            &repository,
            include_bytes!("../../tests/fixtures/sessions/child.jsonl"),
            "child.jsonl",
        );

        let sessions = repository.local_session_view(2_000).unwrap().sessions;

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "root-1");
        assert_eq!(sessions[0].child_session_count, 1);
        assert_eq!(sessions[0].tokens.input_tokens, 1_300);
        assert_eq!(sessions[0].tokens.output_tokens, 280);
    }

    #[test]
    fn an_orphan_is_visible_until_its_parent_arrives() {
        let repository = AccountRepository::open_in_memory().unwrap();
        import_fixture(
            &repository,
            include_bytes!("../../tests/fixtures/sessions/child.jsonl"),
            "child.jsonl",
        );
        let before = repository.local_session_view(1_000).unwrap();
        assert_eq!(before.sessions[0].session_id, "child-1");

        import_fixture(
            &repository,
            include_bytes!("../../tests/fixtures/sessions/root.jsonl"),
            "root.jsonl",
        );
        let after = repository.local_session_view(2_000).unwrap();
        assert_eq!(after.sessions.len(), 1);
        assert_eq!(after.sessions[0].session_id, "root-1");
    }
}
