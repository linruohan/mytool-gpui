use chrono::{Local, NaiveDateTime};
pub struct Queue {
    pub uuid: Option<String>,
    pub object_id: Option<String>,
    pub temp_id: Option<String>,
    pub query: Option<String>,
    pub args: Option<String>,
    pub date_added: Option<String>,
}
impl Queue {
    pub fn date_added(&self) -> NaiveDateTime {
        self.date_added
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| Local::now().naive_local())
    }

    pub fn set_reminder_type(&mut self, date_added: &NaiveDateTime) {
        self.date_added = Some(date_added.to_string());
    }
}
