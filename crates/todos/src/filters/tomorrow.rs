use crate::BaseObject;

pub struct Tomorrow {
    pub base: BaseObject,
}
impl Default for Tomorrow {
    fn default() -> Self {
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
