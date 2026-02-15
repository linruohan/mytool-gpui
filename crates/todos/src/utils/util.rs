use std::{borrow::Cow, collections::HashMap, fmt::Error};

use rand::Rng;
use uuid::Uuid;

use crate::{constants, objects::Color};

/// 工具类，提供颜色管理、文本处理等通用功能
#[derive(Default)]
pub struct Util {
    pub colors: HashMap<String, Color>,
}

impl Util {
    /// 获取默认工具实例
    pub fn get_default() -> Util {
        Self { colors: HashMap::new() }
    }

    /// 获取所有预定义颜色
    pub fn get_colors(&self) -> HashMap<String, Color> {
        let mut colors = HashMap::new();
        if self.colors.is_empty() {
            colors.insert("berry_red".to_string(), Color::new(30, "Berry Red", "#b8256f"));
            colors.insert("red".to_string(), Color::new(31, "Red", "#db4035"));
            colors.insert("orange".to_string(), Color::new(32, "Orange", "#ff9933"));
            colors.insert("yellow".to_string(), Color::new(33, "Olive Green", "#fad000"));
            colors.insert("olive_green".to_string(), Color::new(34, "Yellow", "#afb83b"));
            colors.insert("lime_green".to_string(), Color::new(35, "Lime Green", "#7ecc49"));
            colors.insert("green".to_string(), Color::new(36, "Green", "#299438"));
            colors.insert("mint_green".to_string(), Color::new(37, "Mint Green", "#6accbc"));
            colors.insert("teal".to_string(), Color::new(38, "Teal", "#158fad"));
            colors.insert("sky_blue".to_string(), Color::new(39, "Sky Blue", "#14aaf5"));
            colors.insert("light_blue".to_string(), Color::new(40, "Light Blue", "#96c3eb"));
            colors.insert("blue".to_string(), Color::new(41, "Blue", "#4073ff"));
            colors.insert("grape".to_string(), Color::new(42, "Grape", "#884dff"));
            colors.insert("violet".to_string(), Color::new(43, "Violet", "#af38eb"));
            colors.insert("lavender".to_string(), Color::new(44, "Lavender", "#eb96eb"));
            colors.insert("magenta".to_string(), Color::new(45, "Magenta", "#e05194"));
            colors.insert("salmon".to_string(), Color::new(46, "Salmon", "#ff8d85"));
            colors.insert("charcoal".to_string(), Color::new(47, "Charcoal", "#808080"));
            colors.insert("grey".to_string(), Color::new(48, "Grey", "#b8b8b8"));
            colors.insert("taupe".to_string(), Color::new(49, "Taupe", "#ccac93"));
        }
        colors
    }

    /// 根据颜色键获取颜色名称
    pub fn get_color_name(&self, key: String) -> String {
        if let Some(color) = self.get_colors().get(&key) {
            return color.name.clone();
        }
        "".to_string()
    }

    /// 根据颜色键获取颜色十六进制值
    pub fn get_color(&self, key: String) -> String {
        if let Some(color) = self.get_colors().get(&key) {
            return color.hexadecimal.clone();
        }
        key
    }

    /// 根据颜色键获取 u32 颜色值（用于 GPUI）
    pub fn get_color_u32_by_key(&self, key: String) -> u32 {
        let num = self.get_colors().get(&key).and_then(|color| {
            let color_str = color.hexadecimal.trim();
            if let Some(stripped) = color_str.strip_prefix("0x") {
                u32::from_str_radix(stripped, 16).ok()
            } else if let Some(stripped) = color_str.strip_prefix('#') {
                u32::from_str_radix(stripped, 16).ok()
            } else {
                color_str.parse::<u32>().ok()
            }
        });
        num.unwrap_or(0x000000) // 默认黑色
    }

    /// 获取随机颜色
    pub fn get_random_color(&self) -> String {
        use rand::Rng;
        let mut returned = "berry_red".to_string();
        let random_int = rand::thread_rng().gen_range(30..51);
        for (k, v) in self.get_colors() {
            if v.id == random_int {
                returned = k;
                break;
            }
        }
        returned
    }

    /// 生成唯一 ID
    pub fn generate_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// 编码文本（URL 编码）
    pub fn get_encode_text(text: String) -> String {
        text.replace("&", "%26").replace("#", "%23")
    }

    /// 转义 HTML 特殊字符
    pub fn escape_text(text: String) -> String {
        let mut output = String::with_capacity(text.len() * 2); // 预分配空间
        for c in text.chars() {
            match c {
                '<' => output.push_str("&lt;"),
                '>' => output.push_str("&gt;"),
                '&' => output.push_str("&amp;"),
                '\'' => output.push_str("&apos;"),
                '"' => output.push_str("&quot;"),
                _ => output.push(c),
            }
        }
        output
    }

    /// 获取短名称（截断并添加省略号）
    pub fn get_short_name(&self, name: &str, size: usize) -> String {
        let mut size_default = size;
        if size_default == 0 {
            size_default = constants::SHORT_NAME_SIZE;
        }
        match size_default {
            s if s < name.len() => format!("{}...", &name[0..s]),
            _ => name.to_string(),
        }
    }

    /// 获取优先级标题
    pub fn get_priority_title(&self, priority: i32) -> String {
        match priority {
            constants::PRIORITY_1 => "Priority 1: high".to_string(),
            constants::PRIORITY_2 => "Priority 2: medium".to_string(),
            constants::PRIORITY_3 => "Priority 3: low".to_string(),
            _ => "Priority 4: none".to_string(),
        }
    }

    /// 获取优先级关键词
    pub fn get_priority_keywords(&self, priority: i32) -> String {
        match priority {
            constants::PRIORITY_1 => format!("{};{}", "p1", "high"),
            constants::PRIORITY_2 => format!("{};{}", "p2", "medium"),
            constants::PRIORITY_3 => format!("{};{}", "p3", "low"),
            constants::PRIORITY_4 => format!("{};{}", "p4", "none"),
            _ => "".to_string(),
        }
    }

    /// 检查 URL 是否存在于列表中
    fn url_exists(&self, url: String, urls: Vec<RegexMarkdown>) -> bool {
        for m in urls {
            if url == m.extra {
                return true;
            }
        }
        false
    }

    /// 获取提醒偏移时间（分钟）
    pub fn get_reminders_mm_offset(&self) -> i32 {
        let value = 4;
        match value {
            0 => 0,
            1 => 10,
            2 => 30,
            3 => 45,
            4 => 60,
            5 => 120,
            6 => 180,
            _ => 0,
        }
    }

    /// 获取提醒偏移时间文本
    pub fn get_reminders_mm_offset_text(&self, value: i32) -> &'static str {
        match value {
            0 => "At due time",
            10 => "10 minutes before",
            30 => "30 minutes before",
            45 => "45 minutes before",
            60 => "1 hour before",
            120 => "2 hours before",
            180 => "3 hours before",
            _ => "",
        }
    }
}

/// Markdown 正则匹配结果
pub struct RegexMarkdown {
    pub matchs: String,
    pub text: String,
    pub extra: String,
}

impl RegexMarkdown {
    pub fn new(matchs: String, text: String, extra: String) -> RegexMarkdown {
        Self { matchs, text, extra }
    }
}

use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// 截断字符串并添加省略号
pub fn truncate_at(input: &str, max: i32) -> String {
    let max_len: usize = max as usize;
    if input.len() > max_len {
        let truncated = &input[..(max_len - 3)];
        return format!("{truncated}...");
    };

    input.to_string()
}

/// 验证数据库路径，如果不存在则创建
pub fn verify_db_path(db_folder: &str) -> Result<(), Error> {
    if !Path::new(db_folder).exists() {
        // 检查文件夹是否存在
        match fs::create_dir(db_folder) {
            Ok(_) => println!("Folder '{db_folder}' created."),
            Err(e) => eprintln!("Error creating folder: {e}"),
        }
    }
    Ok(())
}

/// 获取用户主目录
fn get_home() -> String {
    match env::var("HOME") {
        Ok(home) => home,
        Err(_) => {
            eprintln!("Unable to get home directory");
            String::new()
        },
    }
}

/// 获取项目目录路径
pub fn get_project_dirs() -> (PathBuf, PathBuf) {
    let home = get_home();
    let data_dir = Path::new(&home).join(".local/share/planify");
    let config_dir = Path::new(&home).join(".config/planify");
    (data_dir, config_dir)
}
