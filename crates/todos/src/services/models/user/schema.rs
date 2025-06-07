diesel::table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}
