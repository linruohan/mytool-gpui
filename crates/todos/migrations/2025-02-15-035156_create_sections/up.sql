-- Your SQL goes here
CREATE TABLE IF NOT EXISTS sections (
    id TEXT PRIMARY KEY,
    name TEXT,
    archived_at TEXT,
    added_at TEXT,
    project_id TEXT,
    section_order INTEGER,
    collapsed INTEGER,
    is_deleted INTEGER,
    is_archived INTEGER,
    color TEXT,
    description TEXT,
    hidded INTEGER,
    FOREIGN KEY (project_id) REFERENCES Projects (id) ON DELETE CASCADE
);