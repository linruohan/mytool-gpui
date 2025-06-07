-- Your SQL goes here
CREATE TABLE IF NOT EXISTS reminders (
    id TEXT PRIMARY KEY,
    notify_uid INTEGER,
    item_id TEXT,
    service TEXT,
    reminder_type TEXT,
    due TEXT,
    mm_offset INTEGER,
    is_deleted INTEGER,
    FOREIGN KEY (item_id) REFERENCES Items (id) ON DELETE CASCADE
);