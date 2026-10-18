ALTER TABLE session_source_files ADD COLUMN current_cwd TEXT;

ALTER TABLE session_usage_events ADD COLUMN cwd TEXT;
