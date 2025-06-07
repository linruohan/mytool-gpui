use std::str::FromStr;

use crate::generate_accessors;
use crate::schema::queue;
use chrono::{Local, NaiveDateTime};
use diesel::Queryable;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
#[derive(
    QueryableByName, Queryable, PartialEq, Insertable, Clone, Eq, Selectable, Serialize, Debug,
)]
#[diesel(table_name = queue)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Queue {
    pub uuid: Option<String>,
    pub object_id: Option<String>,
    pub temp_id: Option<String>,
    pub query: Option<String>,
    pub args: Option<String>,
    pub date_added: Option<String>,
}
impl Queue {
    generate_accessors!(uuid:Option<String>);
    generate_accessors!(object_id:Option<String>);
    generate_accessors!(temp_id:Option<String>);
    generate_accessors!(query:Option<String>);
    generate_accessors!(args:Option<String>);
    // generate_accessors!(date_added:Option<String>);
    pub fn date_added(&self) -> NaiveDateTime {
        self.date_added
            .as_ref()
            .and_then(|s| NaiveDateTime::from_str(s).ok())
            .unwrap_or(Local::now().naive_local())
    }
    pub fn set_reminder_type(&mut self, date_added: &NaiveDateTime) {
        self.date_added = Some(date_added.to_string());
    }
}
