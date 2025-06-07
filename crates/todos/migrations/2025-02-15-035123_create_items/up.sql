-- Your SQL goes here
CREATE TABLE IF NOT EXISTS items (
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
    checked INTEGER,
    is_deleted INTEGER,
    day_order INTEGER,
    collapsed INTEGER,
    pinned INTEGER,
    labels TEXT,
    extra_data TEXT,
    item_type TEXT
);

PRAGMA foreign_keys = ON;

CREATE TRIGGER IF NOT EXISTS after_insert_item
AFTER
INSERT
    ON items BEGIN
INSERT
    OR IGNORE INTO o_events (
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
    ON items FOR EACH ROW
    WHEN NEW.content != OLD.content BEGIN
INSERT
    OR IGNORE INTO o_events (
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
    ON items FOR EACH ROW
    WHEN NEW.description != OLD.description BEGIN
INSERT
    OR IGNORE INTO o_events (
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
    ON items FOR EACH ROW
    WHEN NEW.due != OLD.due BEGIN
INSERT
    OR IGNORE INTO o_events (
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
    ON items FOR EACH ROW
    WHEN NEW.priority != OLD.priority BEGIN
INSERT
    OR IGNORE INTO o_events (
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
    ON items FOR EACH ROW
    WHEN NEW.labels != OLD.labels BEGIN
INSERT
    OR IGNORE INTO o_events (
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
    ON items FOR EACH ROW
    WHEN NEW.pinned != OLD.pinned BEGIN
INSERT
    OR IGNORE INTO o_events (
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
    ON items FOR EACH ROW
    WHEN NEW.checked != OLD.checked BEGIN
INSERT
    OR IGNORE INTO o_events (
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
    ON items FOR EACH ROW
    WHEN NEW.section_id != OLD.section_id BEGIN
INSERT
    OR IGNORE INTO o_events (
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
    ON items FOR EACH ROW
    WHEN NEW.project_id != OLD.project_id BEGIN
INSERT
    OR IGNORE INTO o_events (
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