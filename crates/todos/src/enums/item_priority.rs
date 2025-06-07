pub enum ItemPriority {
    HIGHT = 4,
    MEDIUM = 3,
    LOW = 2,
    NONE = 1,
}
impl ItemPriority {
    pub fn parse(value: Option<&str>) -> ItemPriority {
        match value {
            Some("p1") => return ItemPriority::HIGHT,
            Some("p2") => return ItemPriority::MEDIUM,
            Some("p3") => return ItemPriority::LOW,
            Some("p4") => return ItemPriority::NONE,
            _ => return ItemPriority::NONE,
        }
    }
}
