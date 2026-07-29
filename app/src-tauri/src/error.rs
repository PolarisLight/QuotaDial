#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("failed to start Codex app-server: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("Codex app-server did not expose its {0} pipe")]
    MissingPipe(&'static str),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid app-server JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("app-server RPC error {code:?}: {message}")]
    Rpc { code: Option<i64>, message: String },
    #[error("Codex app-server disconnected")]
    Disconnected,
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("{0}")]
    Unavailable(String),
}
