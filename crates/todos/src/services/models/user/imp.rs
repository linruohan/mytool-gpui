use super::schema::users;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Deserialize;

#[derive(Queryable, PartialEq, Debug)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct User {
    created_at: NaiveDateTime,
    hair_color: Option<String>,
    id: i32,
    name: String,
    updated_at: NaiveDateTime,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = users)]
pub struct UserForm<'a> {
    name: &'a str,
    hair_color: Option<&'a str>,
}
