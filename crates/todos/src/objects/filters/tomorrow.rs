use crate::BaseObject;

pub struct Tomorrow {
    pub base: BaseObject,
}
impl Tomorrow {
    pub fn default() -> Tomorrow {
        Self {
            base: BaseObject::new(
                "Tomorrow".to_string(),
                format!("{};{};{}", "tomorrow", "filters", "date"),
                "today-calendar-symbolic".to_string(),
                "tomorrow-view".to_string(),
            ),
        }
    }
}
