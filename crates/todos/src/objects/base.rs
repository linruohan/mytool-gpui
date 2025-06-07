use super::{FilterItem, Item, Label, Project, Reminder, Section, Source};
use crate::enums::ObjectType;
use crate::objects::DueDate;
use chrono::{
    Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, ParseError, Timelike,
};
use std::{any::type_name, collections::HashMap};

pub trait ToBool {
    fn to_bool(self) -> bool;
}

impl ToBool for i32 {
    fn to_bool(self) -> bool {
        self != 0
    }
}

impl ToBool for bool {
    fn to_bool(self) -> bool {
        self
    }
}

#[macro_export]
macro_rules! generate_accessors {
    // 处理 Option<i32> 返回bool类型的字段
    (@bool $field:ident: Option<i32>) => {
        paste! {

            // 基础访问器
            pub fn $field(&self) -> bool {
                self.$field.map_or(false, |value| value != 0)
            }
            // 支持多种输入类型的设置方法
            pub fn [<set_$field>]<T : ToBool>(&mut self, value: T) -> &mut Self {
                self.$field = Some(if value.to_bool() { 1 } else { 0 });
                self
            }
        }
    };
    // 处理 Option<String> 返回DueDate类型的字段
    (@due $field:ident: Option<String>) => {
        paste! {
            pub fn $field(&self) -> DueDate {
                self.$field.as_deref().map(
                    |date_str| serde_json::from_str::<DueDate>(date_str).unwrap_or_else(|_| DueDate::default())
                ).unwrap_or(DueDate::default())
            }
            pub fn [<set_$field>](&mut self, value: &DueDate) -> &mut Self {
                self.$field = Some(value.to_string());
                self
            }
        }
    };
    // 处理 Option<String> 返回Vec<Label>类型的字段
    (@labels $field:ident: Option<String>) => {
        paste! {
            pub fn $field(&self) -> Vec<Label> {
                self.$field.as_deref().map(
                    |s| serde_json::from_str::<Vec<Label>>(s).unwrap_or_else(|_| Vec::new())
                ).unwrap_or( Vec::new())
            }
            pub fn [<set_$field>](&mut self, value: &str) -> &mut Self {
                self.$field = Some(value.to_string());
                self
            }
        }
    };
    // 处理 Option<String> 返回NaiveDateTime类型的字段
    (@nativedatetime $field:ident: Option<String>) => {
        paste! {
            pub fn $field(&self) -> NaiveDateTime {
                Local::now().naive_local()
            }
            pub fn [<set_$field>](&mut self, value: &NaiveDateTime) -> &mut Self {
                self.$field = Some(value.format("%Y-%m-%d %H:%M:%S").to_string());
                self
            }
        }
    };
    // 处理只读字段
    (readonly $field:ident: $type:ty) => {
        pub fn $field(&self) -> &$type {
            &self.$field
        }
    };

    // 处理 Option<i32> 类型字段
    ($field:ident: Option<i32>) => {
        paste! {
            pub fn $field(&self) -> i32 {
                self.$field.unwrap_or_default()
            }

            pub fn  [<set_$field>](&mut self, value: impl Into<i32>) -> &mut Self {
                self.$field = Some(value.into());
                self
            }
        }
    };

    // 处理 Option<String> 类型字段
    ($field:ident: Option<String>) => {
        paste! {
            pub fn $field(&self) -> &str {
                self.$field.as_deref().unwrap_or_default()
            }

            pub fn  [<set_$field>](&mut self, value: impl Into<String>) -> &mut Self {
                self.$field = Some(value.into());
                self
            }

            pub fn [<clear_$field>](&mut self) {
                self.$field = None;
            }
        }
    };

    // 处理普通 String 类型字段
    ($field:ident: String) => {
        paste! {
            pub fn $field(&self) -> &str {
                &self.$field
            }
            pub fn [<set_$field>](&mut self, value: impl Into<String>) -> &mut Self {
                self.$field = value.into();
                self
            }
        }
    };
}
pub trait BaseTrait {
    fn type_name(&self) -> &str {
        let full_name = type_name::<Self>();
        full_name.split("::").last().unwrap()
    }
    fn type_delete(&self) -> String {
        format!("{}_delete", self.type_name().to_lowercase())
    }
    fn type_add(&self) -> String {
        format!("{}_add", self.type_name().to_lowercase())
    }
    fn type_update(&self) -> String {
        format!("{}_update", self.type_name().to_lowercase())
    }
    fn object_type(&self) -> ObjectType {
        match self.type_name() {
            "Item" => ObjectType::ITEM,
            "Section" => ObjectType::SECTION,
            "Project" => ObjectType::PROJECT,
            "Label" => ObjectType::LABEL,
            _ => ObjectType::FILTER,
        }
    }
    fn object_type_string(&self) -> &str {
        match self.type_name() {
            "Item" => "item",
            "Section" => "section",
            "Project" => "project",
            "Label" => "label",
            _ => "filter",
        }
    }

    fn table_name(&self) -> String {
        format!("{}s", self.type_name())
    }
    fn column_order_name(&self) -> &str {
        match self.type_name() {
            "Item" => "child_order",
            "Section" => "section_order",
            "Project" => "child_order",
            "Label" => "item_order",
            _ => "",
        }
    }
    fn get_update_json(&self, uuid: String, temp_id: String) -> &str {
        ""
    }

    fn get_add_json(&self, temp_id: String, uuid: String) -> &str {
        ""
    }
    fn get_move_json(&self, new_project_id: String, uuid: String) -> &str {
        ""
    }
    fn to_json(&self) -> &str {
        ""
    }

    fn id(&self) -> &str;
    fn set_id(&mut self, id: &str);
    fn id_string(&self) -> &str {
        self.id().into()
    }

    // signal
    fn deleted(&self) {}
    fn updated(&self, update_id: String) {}
    fn archived(&self) {}
    fn unarchived(&self) {}
    fn filter_added(&mut self, filter: FilterItem) {}
    fn filter_removed(&mut self, filter: FilterItem) {}
    fn filter_updated(&mut self, filter: FilterItem) {}
}
