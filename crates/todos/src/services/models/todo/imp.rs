use super::schema::todos;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Insertable, AsChangeset, Selectable, Serialize, Deserialize, Debug)]
#[diesel(table_name = todos)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Todo {
    pub id: i32,
    pub title: String,
    pub content: String,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = todos)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewTodo {
    pub title: String,
    pub content: String,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = todos)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateTodo {
    pub title: String,
    pub content: String,
}
