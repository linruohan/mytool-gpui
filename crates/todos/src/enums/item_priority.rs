#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum ItemPriority {
    HIGH = 1,
    MEDIUM = 2,
    LOW = 3,
    #[default]
    NONE = 4,
}
impl ItemPriority {
    // 从 i32 转换
    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => ItemPriority::HIGH,
            2 => ItemPriority::MEDIUM,
            3 => ItemPriority::LOW,
            4 => ItemPriority::NONE,
            _ => ItemPriority::NONE,
        }
    }

    // 获取所有优先级变体
    pub fn all() -> Vec<Self> {
        vec![ItemPriority::HIGH, ItemPriority::MEDIUM, ItemPriority::LOW, ItemPriority::NONE]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ItemPriority::HIGH => "Priority 1: High",
            ItemPriority::MEDIUM => "Priority 2: Medium",
            ItemPriority::LOW => "Priority 3: Low",
            ItemPriority::NONE => "Priority 4: NONE",
        }
    }

    pub fn parse(value: Option<&str>) -> ItemPriority {
        match value {
            Some("p1") => ItemPriority::HIGH,
            Some("p2") => ItemPriority::MEDIUM,
            Some("p3") => ItemPriority::LOW,
            Some("p4") => ItemPriority::NONE,
            _ => ItemPriority::NONE,
        }
    }

    pub fn get_color(&self) -> u32 {
        match self {
            ItemPriority::HIGH => 0xff7066,
            ItemPriority::MEDIUM => 0xff9914,
            ItemPriority::LOW => 0x5297ff,
            ItemPriority::NONE => {
                // "#fafafa"
                0x333333
            },
        }
    }
}
