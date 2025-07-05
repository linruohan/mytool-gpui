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
                &self.model.$field
            }
            pub fn [<set_$field>](&mut self, value: impl Into<String>) -> &mut Self {
                self.model.$field = value.into();
                self
            }
        }
    };
    // 处理普通 i32 类型字段
    ($field:ident: i32) => {
        paste! {
            pub fn $field(&self) -> i32 {
                &self.model.$field
            }
            pub fn [<set_$field>](&mut self, value: i32) -> &mut Self {
                self.model.$field = value.into();
                self
            }
        }
    };
}
