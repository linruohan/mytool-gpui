use crate::BaseObject;

pub struct Unlabeled {
    pub base: BaseObject,
}
impl Default for Unlabeled {
    fn default() -> Self {
        Self {
            base: BaseObject::new(
                "Unlabeled".to_string(),
                format!("{};{};{}", "no label", "unlabeled", "filters"),
                "tag-outline-remove-symbolic".to_string(),
                "unlabeled-view".to_string(),
            ),
        }
    }
}
