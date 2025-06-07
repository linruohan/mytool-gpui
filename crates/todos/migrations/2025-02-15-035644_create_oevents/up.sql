-- Your SQL goes here
CREATE TABLE IF NOT EXISTS o_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT,
    event_date DATETIME DEFAULT (datetime('now', 'localtime')),
    object_id TEXT,
    object_type TEXT,
    object_key TEXT,
    object_old_value TEXT,
    object_new_value TEXT,
    parent_item_id TEXT,
    parent_project_id TEXT
);