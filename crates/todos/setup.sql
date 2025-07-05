CREATE TABLE IF NOT EXISTS Labels (
    id TEXT PRIMARY KEY,
    name TEXT,
    color TEXT,
    item_order INTEGER,
    is_deleted BOOLEAN,
    is_favorite BOOLEAN,
    backend_type TEXT,
    source_id TEXT,
    CONSTRAINT unique_label UNIQUE (name)
);

CREATE TABLE IF NOT EXISTS Projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    color TEXT,
    backend_type TEXT,
    inbox_project INTEGER,
    team_inbox INTEGER,
    child_order INTEGER,
    is_deleted BOOLEAN,
    is_archived BOOLEAN,
    is_favorite BOOLEAN,
    shared INTEGER,
    view_style TEXT,
    sort_order INTEGER,
    parent_id TEXT,
    collapsed BOOLEAN,
    icon_style TEXT,
    emoji TEXT,
    show_completed BOOLEAN,
    description TEXT,
    due_date TEXT,
    inbox_section_hidded INTEGER,
    sync_id TEXT,
    source_id TEXT
);

CREATE TABLE IF NOT EXISTS Sections (
    id TEXT PRIMARY KEY,
    name TEXT,
    archived_at TEXT,
    added_at TEXT,
    project_id TEXT,
    section_order INTEGER,
    collapsed BOOLEAN,
    is_deleted BOOLEAN,
    is_archived BOOLEAN,
    color TEXT,
    description TEXT,
    hidded BOOLEAN,
    FOREIGN KEY (project_id) REFERENCES Projects (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS Items (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    description TEXT,
    due TEXT,
    added_at TEXT,
    completed_at TEXT,
    updated_at TEXT,
    section_id TEXT,
    project_id TEXT,
    parent_id TEXT,
    priority INTEGER,
    child_order INTEGER,
    checked BOOLEAN,
    is_deleted BOOLEAN,
    day_order INTEGER,
    collapsed BOOLEAN,
    pinned BOOLEAN,
    labels TEXT,
    extra_data TEXT,
    item_type TEXT
);

CREATE TABLE IF NOT EXISTS Reminders (
    id TEXT PRIMARY KEY,
    notify_uid INTEGER,
    item_id TEXT,
    service TEXT,
    type TEXT,
    due TEXT,
    mm_offset INTEGER,
    is_deleted BOOLEAN,
    FOREIGN KEY (item_id) REFERENCES Items (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS Queue (
    uuid TEXT PRIMARY KEY,
    object_id TEXT,
    query TEXT,
    temp_id TEXT,
    args TEXT,
    date_added TEXT
);

CREATE TABLE IF NOT EXISTS CurTempIds (
    id TEXT PRIMARY KEY,
    temp_id TEXT,
    object TEXT
);

CREATE TABLE IF NOT EXISTS Attachments (
    id TEXT PRIMARY KEY,
    item_id TEXT,
    file_type TEXT,
    file_name TEXT,
    file_size TEXT,
    file_path TEXT,
    FOREIGN KEY (item_id) REFERENCES Items (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS OEvents (
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

CREATE TABLE IF NOT EXISTS Sources (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,
    display_name TEXT,
    added_at TEXT,
    updated_at TEXT,
    is_visible BOOLEAN,
    child_order INTEGER,
    sync_server INTEGER,
    last_sync TEXT,
    data TEXT
);

PRAGMA foreign_keys = ON;

-- # 创建触发器
CREATE TRIGGER IF NOT EXISTS after_insert_item
AFTER
INSERT
    ON Items BEGIN
INSERT
    OR IGNORE INTO OEvents (
        event_type,
        object_id,
        object_type,
        object_key,
        object_old_value,
        object_new_value,
        parent_project_id
    )
VALUES
    (
        "insert",
        NEW.id,
        "item",
        "content",
        NEW.content,
        NEW.content,
        NEW.project_id
    );

END;

CREATE TRIGGER IF NOT EXISTS after_update_content_item
AFTER
UPDATE
    ON Items FOR EACH ROW
    WHEN NEW.content != OLD.content BEGIN
INSERT
    OR IGNORE INTO OEvents (
        event_type,
        object_id,
        object_type,
        object_key,
        object_old_value,
        object_new_value,
        parent_project_id
    )
VALUES
    (
        "update",
        NEW.id,
        "item",
        "content",
        OLD.content,
        NEW.content,
        NEW.project_id
    );

END;

CREATE TRIGGER IF NOT EXISTS after_update_description_item
AFTER
UPDATE
    ON Items FOR EACH ROW
    WHEN NEW.description != OLD.description BEGIN
INSERT
    OR IGNORE INTO OEvents (
        event_type,
        object_id,
        object_type,
        object_key,
        object_old_value,
        object_new_value,
        parent_project_id
    )
VALUES
    (
        "update",
        NEW.id,
        "item",
        "description",
        OLD.description,
        NEW.description,
        NEW.project_id
    );

END;

CREATE TRIGGER IF NOT EXISTS after_update_due_item
AFTER
UPDATE
    ON Items FOR EACH ROW
    WHEN NEW.due != OLD.due BEGIN
INSERT
    OR IGNORE INTO OEvents (
        event_type,
        object_id,
        object_type,
        object_key,
        object_old_value,
        object_new_value,
        parent_project_id
    )
VALUES
    (
        "update",
        NEW.id,
        "item",
        "due",
        OLD.due,
        NEW.due,
        NEW.project_id
    );

END;

CREATE TRIGGER IF NOT EXISTS after_update_priority_item
AFTER
UPDATE
    ON Items FOR EACH ROW
    WHEN NEW.priority != OLD.priority BEGIN
INSERT
    OR IGNORE INTO OEvents (
        event_type,
        object_id,
        object_type,
        object_key,
        object_old_value,
        object_new_value,
        parent_project_id
    )
VALUES
    (
        "update",
        NEW.id,
        "item",
        "priority",
        OLD.priority,
        NEW.priority,
        NEW.project_id
    );

END;

CREATE TRIGGER IF NOT EXISTS after_update_labels_item
AFTER
UPDATE
    ON Items FOR EACH ROW
    WHEN NEW.labels != OLD.labels BEGIN
INSERT
    OR IGNORE INTO OEvents (
        event_type,
        object_id,
        object_type,
        object_key,
        object_old_value,
        object_new_value,
        parent_project_id
    )
VALUES
    (
        "update",
        NEW.id,
        "item",
        "labels",
        OLD.labels,
        NEW.labels,
        NEW.project_id
    );

END;

CREATE TRIGGER IF NOT EXISTS after_update_pinned_item
AFTER
UPDATE
    ON Items FOR EACH ROW
    WHEN NEW.pinned != OLD.pinned BEGIN
INSERT
    OR IGNORE INTO OEvents (
        event_type,
        object_id,
        object_type,
        object_key,
        object_old_value,
        object_new_value,
        parent_project_id
    )
VALUES
    (
        "update",
        NEW.id,
        "item",
        "pinned",
        OLD.pinned,
        NEW.pinned,
        NEW.project_id
    );

END;

CREATE TRIGGER IF NOT EXISTS after_update_checked_item
AFTER
UPDATE
    ON Items FOR EACH ROW
    WHEN NEW.checked != OLD.checked BEGIN
INSERT
    OR IGNORE INTO OEvents (
        event_type,
        object_id,
        object_type,
        object_key,
        object_old_value,
        object_new_value,
        parent_project_id
    )
VALUES
    (
        "update",
        NEW.id,
        "item",
        "checked",
        OLD.checked,
        NEW.checked,
        NEW.project_id
    );

END;

CREATE TRIGGER IF NOT EXISTS after_update_section_item
AFTER
UPDATE
    ON Items FOR EACH ROW
    WHEN NEW.section_id != OLD.section_id BEGIN
INSERT
    OR IGNORE INTO OEvents (
        event_type,
        object_id,
        object_type,
        object_key,
        object_old_value,
        object_new_value,
        parent_project_id
    )
VALUES
    (
        "update",
        NEW.id,
        "item",
        "section",
        OLD.section_id,
        NEW.section_id,
        NEW.project_id
    );

END;

CREATE TRIGGER IF NOT EXISTS after_update_project_item
AFTER
UPDATE
    ON Items FOR EACH ROW
    WHEN NEW.project_id != OLD.project_id BEGIN
INSERT
    OR IGNORE INTO OEvents (
        event_type,
        object_id,
        object_type,
        object_key,
        object_old_value,
        object_new_value,
        parent_project_id
    )
VALUES
    (
        "update",
        NEW.id,
        "item",
        "project",
        OLD.project_id,
        NEW.project_id,
        NEW.project_id
    );

END;