pub enum ItemPriority {
    HIGHT = 4,
    MEDIUM = 3,
    LOW = 2,
    NONE = 1,
}
impl ItemPriority {
    pub fn parse(value: Option<&str>) -> ItemPriority {
        match value {
            Some("p1") => ItemPriority::HIGHT,
            Some("p2") => ItemPriority::MEDIUM,
            Some("p3") => ItemPriority::LOW,
            Some("p4") => ItemPriority::NONE,
            _ => ItemPriority::NONE,
        }
    }
}
