use crate::error::AppError;
use rusqlite::{params, Connection, OptionalExtension};

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../../migrations/001_account.sql")),
    (2, include_str!("../../migrations/002_sessions.sql")),
    (3, include_str!("../../migrations/003_settings.sql")),
];

pub fn run(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection.transaction()?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        );",
    )?;

    for (version, sql) in MIGRATIONS {
        let applied = transaction
            .query_row(
                "SELECT version FROM schema_migrations WHERE version = ?1",
                params![version],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some();

        if !applied {
            transaction.execute_batch(sql)?;
            transaction.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, unixepoch())",
                params![version],
            )?;
        }
    }

    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_two_is_idempotent_and_preserves_account_observations() {
        let mut connection = Connection::open_in_memory().unwrap();
        run(&mut connection).unwrap();
        connection
            .execute(
                "INSERT INTO account_usage_observations(
                    observed_at,
                    lifetime_tokens,
                    peak_daily_tokens,
                    daily_buckets_json,
                    payload_json
                 ) VALUES (1, 10, 5, NULL, '{}')",
                [],
            )
            .unwrap();

        run(&mut connection).unwrap();
        run(&mut connection).unwrap();

        let account_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM account_usage_observations",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let session_table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name = 'session_usage_events'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(account_count, 1);
        assert_eq!(session_table_count, 1);
    }
}
