use crate::BaseObject;

pub struct Repeating {
    pub base: BaseObject,
}
impl Default for Repeating {
    fn default() -> Self {
        Self {
            base: BaseObject::new(
                "Repeating".to_string(),
                "repeating;filters".to_string(),
                "arrow-circular-top-right-symbolic".to_string(),
                "repeating-view".to_string(),
            ),
        }
    }
}
