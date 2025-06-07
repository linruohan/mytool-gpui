-- Your SQL goes here
CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY,
    item_id TEXT,
    file_type TEXT,
    file_name TEXT,
    file_size TEXT,
    file_path TEXT,
    FOREIGN KEY (item_id) REFERENCES Items (id) ON DELETE CASCADE
);