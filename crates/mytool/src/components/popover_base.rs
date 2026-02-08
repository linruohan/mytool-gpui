/// 通用弹窗组件基础模块
///
/// 提供弹窗组件的通用功能，包括：
/// - 搜索功能和弹窗状态管理
/// - 项目列表管理
/// - 通用事件处理和UI组件创建
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

/// 通用搜索事件处理函数 - 处理搜索输入的变化
pub fn handle_search_input_change(
    search_input: &Entity<InputState>,
    search_query: &mut String,
    cx: &mut gpui::Context<impl gpui::Focusable>,
) {
    *search_query = search_input.read(cx).value().to_string();
    cx.notify();
}

/// 通用弹窗状态管理函数 - 管理弹窗开关和搜索清理
pub fn manage_popover_state(popover_open: &mut bool, search_query: &mut String, new_open: bool) {
    *popover_open = new_open;
    if !new_open {
        search_query.clear();
    }
}

/// 创建通用列表项的辅助函数
/// 减少重复的列表项渲染代码
pub fn create_list_item_element<T>(
    index: usize,
    display_text: String,
    item_id: String,
    view: gpui::Entity<T>,
    on_remove: impl Fn(String, gpui::Entity<T>, &mut gpui::App) + 'static,
) -> impl gpui::IntoElement
where
    T: gpui::Focusable + 'static,
{
    use gpui::{ParentElement, Styled};
    use gpui_component::{Sizable, button::ButtonVariants};

    gpui::div()
        .flex()
        .flex_row()
        .gap_2()
        .items_center()
        .justify_between()
        .px_2()
        .py_2()
        .border_b_1()
        .child(gpui_component::label::Label::new(display_text).text_sm())
        .child(
            gpui_component::button::Button::new(format!("remove-item-{}", index))
                .small()
                .ghost()
                .compact()
                .icon(gpui_component::IconName::UserTrashSymbolic)
                .on_click(move |_event, _window, cx| {
                    on_remove(item_id.clone(), view.clone(), cx);
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

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
