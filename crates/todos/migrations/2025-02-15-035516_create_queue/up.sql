-- Your SQL goes here
CREATE TABLE IF NOT EXISTS queue (
    uuid TEXT PRIMARY KEY,
    object_id TEXT,
    query TEXT,
    temp_id TEXT,
    args TEXT,
    date_added TEXT
);