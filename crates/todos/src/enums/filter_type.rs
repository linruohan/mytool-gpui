use std::fmt;

use strum::{Display, EnumString};
#[derive(Debug, Clone, PartialEq, EnumString)]
#[strum(serialize_all = "camelCase")] // 自动处理连字符格式
pub enum FilterType {
    INBOX,
    TODAY,
    SCHEDULED,
    PINBOARD,
    LABELS,
    COMPLETED,
}
impl FilterType {
    pub fn get_name(&self) -> String {
        let s = self.to_string();
        format!("{}{}", &s[0..1].to_uppercase(), &s[1..])
    }

    pub fn get_icon(&self) -> &str {
        match self {
            FilterType::INBOX => return "mailbox-symbolic",
            FilterType::TODAY => return "star-outline-thick-symbolic",
            FilterType::SCHEDULED => return "month-symbolic",
            FilterType::PINBOARD => return "pin-symbolic",
            FilterType::LABELS => return "tag-outline-symbolic",
            FilterType::COMPLETED => return "check-round-outline-symbolic",
        }
    }

    pub fn get_color(&self, dark: bool) -> &str {
        match self {
            FilterType::INBOX => {
                return if dark { "#99c1f1" } else { "#3584e4" };
            }
            FilterType::TODAY => return "#33d17a",
            FilterType::SCHEDULED => {
                return if dark { "#dc8add" } else { "#9141ac" };
            }
            FilterType::PINBOARD => {
                return if dark { "#f66151" } else { "#ed333b" };
            }
            FilterType::LABELS => {
                return if dark { "#cdab8f" } else { "#986a44" };
            }
            FilterType::COMPLETED => {
                return if dark { "#ffbe6f" } else { "#ff7800" };
            }
        }
    }
}
impl fmt::Display for FilterType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string().to_lowercase())
    }
}
