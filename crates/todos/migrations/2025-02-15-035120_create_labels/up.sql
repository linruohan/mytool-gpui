-- Your SQL goes here
CREATE TABLE IF NOT EXISTS labels (
    id TEXT PRIMARY KEY,
    name TEXT,
    color TEXT,
    item_order INTEGER,
    is_deleted INTEGER,
    is_favorite INTEGER,
    backend_type TEXT,
    source_id TEXT,
    CONSTRAINT unique_label UNIQUE (name)
);