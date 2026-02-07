use gpui_component::IconName;

/// 下拉菜单项
#[derive(Clone, Debug)]
pub struct DropdownMenuItem {
    pub id: String,
    pub label: String,
}

impl DropdownMenuItem {
    /// 创建新的菜单项
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self { id: id.into(), label: label.into() }
    }
}

/// 下拉菜单按钮配置
pub struct DropdownButtonConfig {
    pub id: String,
    pub icon: IconName,
    pub tooltip: String,
    pub items: Vec<DropdownMenuItem>,
    pub selected_label: Option<String>,
    pub min_width: f32,
    pub max_height: f32,
}

impl DropdownButtonConfig {
    /// 创建新的按钮配置
    pub fn new(id: impl Into<String>, icon: IconName, tooltip: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            icon,
            tooltip: tooltip.into(),
            items: Vec::new(),
            selected_label: None,
            min_width: 100.0,
            max_height: 400.0,
        }
    }

    /// 设置菜单项
    pub fn with_items(mut self, items: Vec<DropdownMenuItem>) -> Self {
        self.items = items;
        self
    }

    /// 设置选中项的标签
    pub fn with_selected_label(mut self, label: impl Into<String>) -> Self {
        self.selected_label = Some(label.into());
        self
    }

    /// 设置最小宽度
    pub fn with_min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    /// 设置最大高度
    pub fn with_max_height(mut self, height: f32) -> Self {
        self.max_height = height;
        self
    }
}

/// 按钮工厂
pub struct ButtonFactory;

impl ButtonFactory {
    /// 获取菜单项的标签
    pub fn get_menu_items(items: &[DropdownMenuItem]) -> Vec<(String, String)> {
        items.iter().map(|item| (item.id.clone(), item.label.clone())).collect()
    }

    /// 获取菜单项数量
    pub fn count_items(config: &DropdownButtonConfig) -> usize {
        config.items.len()
    }

    /// 验证配置
    pub fn validate_config(config: &DropdownButtonConfig) -> Result<(), String> {
        if config.id.is_empty() {
            return Err("Button ID cannot be empty".to_string());
        }
        if config.items.is_empty() {
            return Err("At least one menu item is required".to_string());
        }
        Ok(())
    }

    /// 创建菜单项列表
    pub fn create_menu_items<T: Into<DropdownMenuItem>>(items: Vec<T>) -> Vec<DropdownMenuItem> {
        items.into_iter().map(|item| item.into()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dropdown_menu_item() {
        let item = DropdownMenuItem::new("id1", "Label 1");
        assert_eq!(item.id, "id1");
        assert_eq!(item.label, "Label 1");
    }

    #[test]
    fn test_dropdown_button_config() {
        let config = DropdownButtonConfig::new("btn", IconName::Plus, "Add")
            .with_items(vec![
                DropdownMenuItem::new("1", "Item 1"),
                DropdownMenuItem::new("2", "Item 2"),
            ])
            .with_selected_label("Item 1")
            .with_min_width(150.0)
            .with_max_height(500.0);

        assert_eq!(config.id, "btn");
        assert_eq!(config.items.len(), 2);
        assert_eq!(config.selected_label, Some("Item 1".to_string()));
        assert_eq!(config.min_width, 150.0);
        assert_eq!(config.max_height, 500.0);
    }

    #[test]
    fn test_button_factory_get_menu_items() {
        let items =
            vec![DropdownMenuItem::new("1", "Item 1"), DropdownMenuItem::new("2", "Item 2")];
        let menu_items = ButtonFactory::get_menu_items(&items);
        assert_eq!(menu_items.len(), 2);
        assert_eq!(menu_items[0].0, "1");
        assert_eq!(menu_items[0].1, "Item 1");
    }

    #[test]
    fn test_button_factory_validate_config() {
        let mut config = DropdownButtonConfig::new("btn", IconName::Plus, "Add");

        // Should fail - no items
        assert!(ButtonFactory::validate_config(&config).is_err());

        // Should succeed - has items
        config.items.push(DropdownMenuItem::new("1", "Item 1"));
        assert!(ButtonFactory::validate_config(&config).is_ok());
    }
}
