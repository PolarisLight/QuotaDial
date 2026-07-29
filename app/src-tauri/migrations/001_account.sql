CREATE TABLE IF NOT EXISTS account_rate_limit_observations (
  id INTEGER PRIMARY KEY,
  observed_at INTEGER NOT NULL,
  limit_id TEXT NOT NULL,
  window_kind TEXT NOT NULL CHECK (window_kind IN ('primary', 'secondary')),
  used_percent REAL NOT NULL CHECK (used_percent >= 0 AND used_percent <= 100),
  window_duration_mins INTEGER NOT NULL,
  resets_at INTEGER NOT NULL,
  plan_type TEXT,
  payload_json TEXT NOT NULL,
  UNIQUE(observed_at, limit_id, window_kind)
);

CREATE INDEX IF NOT EXISTS idx_rate_observations_lookup
ON account_rate_limit_observations(limit_id, window_kind, observed_at);

CREATE TABLE IF NOT EXISTS account_usage_observations (
  id INTEGER PRIMARY KEY,
  observed_at INTEGER NOT NULL UNIQUE,
  lifetime_tokens INTEGER,
  peak_daily_tokens INTEGER,
  daily_buckets_json TEXT,
  payload_json TEXT NOT NULL
);
