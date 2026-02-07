/// 通用弹窗列表混入模块
///
/// 提供弹窗列表组件的通用功能，包括：
/// - 搜索功能
/// - 项目列表管理
/// - 弹窗状态管理
use gpui::Entity;
use gpui_component::input::InputState;

/// 弹窗列表的通用搜索功能
pub struct PopoverSearchMixin {
    pub search_input: Entity<InputState>,
    pub search_query: String,
    pub popover_open: bool,
}

impl PopoverSearchMixin {
    /// 创建新的搜索混入
    pub fn new(search_input: Entity<InputState>) -> Self {
        Self { search_input, search_query: String::new(), popover_open: false }
    }

    /// 更新搜索查询
    pub fn update_search_query(&mut self, query: String) {
        self.search_query = query;
    }

    /// 清空搜索查询
    pub fn clear_search(&mut self) {
        self.search_query.clear();
    }

    /// 切换弹窗状态
    pub fn toggle_popover(&mut self) {
        self.popover_open = !self.popover_open;
    }

    /// 打开弹窗
    pub fn open_popover(&mut self) {
        self.popover_open = true;
    }

    /// 关闭弹窗
    pub fn close_popover(&mut self) {
        self.popover_open = false;
    }
}

/// 通用的过滤函数类型
pub type FilterFn<T> = fn(&T, &str) -> bool;

/// 通用的项目列表管理
pub struct PopoverListMixin<T: Clone + 'static> {
    pub items: Vec<T>,
    pub filter_fn: FilterFn<T>,
}

impl<T: Clone + 'static> PopoverListMixin<T> {
    /// 创建新的列表混入
    pub fn new(filter_fn: FilterFn<T>) -> Self {
        Self { items: Vec::new(), filter_fn }
    }

    /// 设置项目列表
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
    }

    /// 添加项目
    pub fn add_item(&mut self, item: T) {
        self.items.push(item);
    }

    /// 移除项目
    pub fn remove_item(&mut self, predicate: impl Fn(&T) -> bool) {
        self.items.retain(|item| !predicate(item));
    }

    /// 获取过滤后的项目
    pub fn get_filtered(&self, query: &str) -> Vec<T> {
        if query.is_empty() {
            self.items.clone()
        } else {
            self.items.iter().filter(|item| (self.filter_fn)(item, query)).cloned().collect()
        }
    }

    /// 清空所有项目
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_popover_search_mixin() {
        let mixin = PopoverSearchMixin {
            search_input: unsafe { std::mem::zeroed() },
            search_query: String::new(),
            popover_open: false,
        };

        assert_eq!(mixin.search_query, "");
        assert!(!mixin.popover_open);
    }

    #[test]
    fn test_popover_list_mixin() {
        let filter_fn: FilterFn<String> =
            |item, query| item.to_lowercase().contains(&query.to_lowercase());

        let mut mixin = PopoverListMixin::new(filter_fn);
        mixin.set_items(vec!["apple".to_string(), "banana".to_string(), "cherry".to_string()]);

        let filtered = mixin.get_filtered("app");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0], "apple");

        mixin.add_item("apricot".to_string());
        let filtered = mixin.get_filtered("ap");
        assert_eq!(filtered.len(), 2);

        mixin.remove_item(|item| item == "apple");
        assert_eq!(mixin.items.len(), 3);
    }
}
