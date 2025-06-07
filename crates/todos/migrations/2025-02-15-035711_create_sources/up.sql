-- Your SQL goes here
CREATE TABLE IF NOT EXISTS sources (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,
    display_name TEXT,
    added_at TEXT,
    updated_at TEXT,
    is_visible INTEGER,
    child_order INTEGER,
    sync_server INTEGER,
    last_sync TEXT,
    data TEXT
);