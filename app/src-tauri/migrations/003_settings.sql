CREATE TABLE IF NOT EXISTS app_settings (
  singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
  payload_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS notification_delivery_state (
  state_key TEXT PRIMARY KEY,
  delivered_at INTEGER NOT NULL
);
