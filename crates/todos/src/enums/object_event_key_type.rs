pub enum ObjectEventKeyType {
    CONTENT,
    DESCRIPTION,
    DUE,
    PRIORITY,
    LABELS,
    PINNED,
    CHECKED,
    PROJECT,
    SECTION,
}
impl ObjectEventKeyType {
    pub fn parse(value: Option<&str>) -> ObjectEventKeyType {
        match value {
            Some("content") => ObjectEventKeyType::CONTENT,
            Some("description") => ObjectEventKeyType::DESCRIPTION,
            Some("due") => ObjectEventKeyType::DUE,
            Some("priority") => ObjectEventKeyType::PRIORITY,
            Some("label") => ObjectEventKeyType::LABELS,
            Some("pinned") => ObjectEventKeyType::PINNED,
            Some("checked") => ObjectEventKeyType::CHECKED,
            Some("project") => ObjectEventKeyType::PROJECT,
            Some("section") => ObjectEventKeyType::SECTION,
            _ => ObjectEventKeyType::CONTENT,
        }
    }
    pub fn get_label(&self) -> &str {
        match self {
            ObjectEventKeyType::CONTENT => "Content",
            ObjectEventKeyType::DESCRIPTION => "Description",
            ObjectEventKeyType::DUE => "Scheduled",
            ObjectEventKeyType::PRIORITY => "Priority",
            ObjectEventKeyType::LABELS => "Labels",
            ObjectEventKeyType::PINNED => "Pin",
            _ => "",
        }
    }
}
