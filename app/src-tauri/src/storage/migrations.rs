use crate::error::AppError;
use rusqlite::{params, Connection, OptionalExtension};

const ACCOUNT_MIGRATION: &str = include_str!("../../migrations/001_account.sql");

pub fn run(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection.transaction()?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        );",
    )?;

    let applied = transaction
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some();

    if !applied {
        transaction.execute_batch(ACCOUNT_MIGRATION)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, unixepoch())",
            params![1],
        )?;
    }

    transaction.commit()?;
    Ok(())
}
