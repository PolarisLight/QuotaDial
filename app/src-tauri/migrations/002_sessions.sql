CREATE TABLE IF NOT EXISTS session_source_files (
  path TEXT NOT NULL,
  generation INTEGER NOT NULL,
  file_identity TEXT,
  byte_offset INTEGER NOT NULL DEFAULT 0,
  observed_size INTEGER NOT NULL DEFAULT 0,
  modified_at INTEGER NOT NULL DEFAULT 0,
  parser_version INTEGER NOT NULL,
  last_error TEXT,
  PRIMARY KEY(path, generation)
);

CREATE TABLE IF NOT EXISTS session_metadata (
  session_id TEXT PRIMARY KEY,
  parent_session_id TEXT,
  started_at INTEGER NOT NULL,
  last_active_at INTEGER NOT NULL,
  cwd TEXT,
  model TEXT,
  source_path TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_session_metadata_parent
ON session_metadata(parent_session_id);

CREATE TABLE IF NOT EXISTS session_usage_events (
  fingerprint TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  occurred_at INTEGER NOT NULL,
  model TEXT,
  input_tokens INTEGER NOT NULL CHECK(input_tokens >= 0),
  cached_input_tokens INTEGER NOT NULL CHECK(cached_input_tokens >= 0),
  output_tokens INTEGER NOT NULL CHECK(output_tokens >= 0),
  reasoning_output_tokens INTEGER NOT NULL CHECK(reasoning_output_tokens >= 0),
  source_path TEXT NOT NULL,
  source_offset INTEGER NOT NULL,
  FOREIGN KEY(session_id) REFERENCES session_metadata(session_id)
);

CREATE INDEX IF NOT EXISTS idx_session_usage_session_time
ON session_usage_events(session_id, occurred_at);

CREATE TABLE IF NOT EXISTS model_price_versions (
  model_pattern TEXT NOT NULL,
  effective_from INTEGER NOT NULL,
  input_per_million REAL NOT NULL,
  cached_input_per_million REAL NOT NULL,
  output_per_million REAL NOT NULL,
  catalog_version TEXT NOT NULL,
  PRIMARY KEY(model_pattern, effective_from)
);
