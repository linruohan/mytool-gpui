// @generated automatically by Diesel CLI.

diesel::table! {
    attachments (id) {
        id -> Nullable<Text>,
        item_id -> Nullable<Text>,
        file_type -> Nullable<Text>,
        file_name -> Nullable<Text>,
        file_size -> Nullable<Text>,
        file_path -> Nullable<Text>,
    }
}

diesel::table! {
    cur_temp_ids (id) {
        id -> Nullable<Text>,
        temp_id -> Nullable<Text>,
        object -> Nullable<Text>,
    }
}

diesel::table! {
    items (id) {
        id -> Nullable<Text>,
        content -> Text,
        description -> Nullable<Text>,
        due -> Nullable<Text>,
        added_at -> Nullable<Text>,
        completed_at -> Nullable<Text>,
        updated_at -> Nullable<Text>,
        section_id -> Nullable<Text>,
        project_id -> Nullable<Text>,
        parent_id -> Nullable<Text>,
        priority -> Nullable<Integer>,
        child_order -> Nullable<Integer>,
        checked -> Nullable<Integer>,
        is_deleted -> Nullable<Integer>,
        day_order -> Nullable<Integer>,
        collapsed -> Nullable<Integer>,
        pinned -> Nullable<Integer>,
        labels -> Nullable<Text>,
        extra_data -> Nullable<Text>,
        item_type -> Nullable<Text>,
    }
}

diesel::table! {
    labels (id) {
        id -> Nullable<Text>,
        name -> Nullable<Text>,
        color -> Nullable<Text>,
        item_order -> Nullable<Integer>,
        is_deleted -> Nullable<Integer>,
        is_favorite -> Nullable<Integer>,
        backend_type -> Nullable<Text>,
        source_id -> Nullable<Text>,
    }
}

diesel::table! {
    o_events (id) {
        id -> Nullable<Integer>,
        event_type -> Nullable<Text>,
        event_date -> Nullable<Timestamp>,
        object_id -> Nullable<Text>,
        object_type -> Nullable<Text>,
        object_key -> Nullable<Text>,
        object_old_value -> Nullable<Text>,
        object_new_value -> Nullable<Text>,
        parent_item_id -> Nullable<Text>,
        parent_project_id -> Nullable<Text>,
    }
}

diesel::table! {
    projects (id) {
        id -> Nullable<Text>,
        name -> Text,
        color -> Nullable<Text>,
        backend_type -> Nullable<Text>,
        inbox_project -> Nullable<Integer>,
        team_inbox -> Nullable<Integer>,
        child_order -> Nullable<Integer>,
        is_deleted -> Nullable<Integer>,
        is_archived -> Nullable<Integer>,
        is_favorite -> Nullable<Integer>,
        shared -> Nullable<Integer>,
        view_style -> Nullable<Text>,
        sort_order -> Nullable<Integer>,
        parent_id -> Nullable<Text>,
        collapsed -> Nullable<Integer>,
        icon_style -> Nullable<Text>,
        emoji -> Nullable<Text>,
        show_completed -> Nullable<Integer>,
        description -> Nullable<Text>,
        due_date -> Nullable<Text>,
        inbox_section_hidded -> Nullable<Integer>,
        sync_id -> Nullable<Text>,
        source_id -> Nullable<Text>,
    }
}

diesel::table! {
    queue (uuid) {
        uuid -> Nullable<Text>,
        object_id -> Nullable<Text>,
        query -> Nullable<Text>,
        temp_id -> Nullable<Text>,
        args -> Nullable<Text>,
        date_added -> Nullable<Text>,
    }
}

diesel::table! {
    reminders (id) {
        id -> Nullable<Text>,
        notify_uid -> Nullable<Integer>,
        item_id -> Nullable<Text>,
        service -> Nullable<Text>,
        reminder_type -> Nullable<Text>,
        due -> Nullable<Text>,
        mm_offset -> Nullable<Integer>,
        is_deleted -> Nullable<Integer>,
    }
}

diesel::table! {
    sections (id) {
        id -> Nullable<Text>,
        name -> Nullable<Text>,
        archived_at -> Nullable<Text>,
        added_at -> Nullable<Text>,
        project_id -> Nullable<Text>,
        section_order -> Nullable<Integer>,
        collapsed -> Nullable<Integer>,
        is_deleted -> Nullable<Integer>,
        is_archived -> Nullable<Integer>,
        color -> Nullable<Text>,
        description -> Nullable<Text>,
        hidded -> Nullable<Integer>,
    }
}

diesel::table! {
    sources (id) {
        id -> Nullable<Text>,
        source_type -> Text,
        display_name -> Nullable<Text>,
        added_at -> Nullable<Text>,
        updated_at -> Nullable<Text>,
        is_visible -> Nullable<Integer>,
        child_order -> Nullable<Integer>,
        sync_server -> Nullable<Integer>,
        last_sync -> Nullable<Text>,
        data -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    attachments,
    cur_temp_ids,
    items,
    labels,
    o_events,
    projects,
    queue,
    reminders,
    sections,
    sources,
);
