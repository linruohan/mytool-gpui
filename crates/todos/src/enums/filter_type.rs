use std::fmt;
use strum::EnumString;
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
            FilterType::INBOX => "mailbox-symbolic",
            FilterType::TODAY => "star-outline-thick-symbolic",
            FilterType::SCHEDULED => "month-symbolic",
            FilterType::PINBOARD => "pin-symbolic",
            FilterType::LABELS => "tag-outline-symbolic",
            FilterType::COMPLETED => "check-round-outline-symbolic",
        }
    }

    pub fn get_color(&self, dark: bool) -> &str {
        match self {
            FilterType::INBOX => {
                if dark {
                    "#99c1f1"
                } else {
                    "#3584e4"
                }
            }
            FilterType::TODAY => "#33d17a",
            FilterType::SCHEDULED => {
                if dark {
                    "#dc8add"
                } else {
                    "#9141ac"
                }
            }
            FilterType::PINBOARD => {
                if dark {
                    "#f66151"
                } else {
                    "#ed333b"
                }
            }
            FilterType::LABELS => {
                if dark {
                    "#cdab8f"
                } else {
                    "#986a44"
                }
            }
            FilterType::COMPLETED => {
                if dark {
                    "#ffbe6f"
                } else {
                    "#ff7800"
                }
            }
        }
    }
    pub fn to_lowercase(&self) -> String {
        match self {
            FilterType::INBOX => "inbox".to_string(),
            FilterType::TODAY => "today".to_string(),
            FilterType::SCHEDULED => "scheduled".to_string(),
            FilterType::PINBOARD => "pinboard".to_string(),
            FilterType::LABELS => "labels".to_string(),
            FilterType::COMPLETED => "completed".to_string(),
        }
    }
}

impl fmt::Display for FilterType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_lowercase())
    }
}
