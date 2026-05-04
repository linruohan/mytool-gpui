use std::sync::Arc;

use gpui::{AppContext, Context, Entity, FocusHandle, Subscription, Window};

use crate::{ItemInfoState, ItemRowState, todo_state::TodoStore};

/// 所有 Board 类型的基础结构体
pub struct BoardBase {
    pub _subscriptions: Vec<Subscription>,
    pub focus_handle: FocusHandle,
    pub active_index: Option<usize>,
    pub item_rows: Vec<Entity<ItemRowState>>,
    pub item_info: Entity<ItemInfoState>,
    pub no_section_items: Vec<(usize, Arc<todos::entity::ItemModel>)>,
    pub section_items_map:
        std::collections::HashMap<String, Vec<(usize, Arc<todos::entity::ItemModel>)>>,
    pub pinned_items: Vec<(usize, Arc<todos::entity::ItemModel>)>,
    pub overdue_items: Vec<(usize, Arc<todos::entity::ItemModel>)>,
    /// 过期任务分组（超过今天但还未完成，仅 Today Board 使用）
    pub past_due_items: Vec<(usize, Arc<todos::entity::ItemModel>)>,
    pub is_today_board: bool,
    /// 分区列表（用于渲染 Section 分组）
    pub sections: Vec<Arc<todos::entity::SectionModel>>,
}

impl BoardBase {
    /// 创建一个新的 BoardBase 实例
    pub fn new(window: &mut Window, cx: &mut Context<impl gpui::Render>) -> Self {
        let item = std::sync::Arc::new(todos::entity::ItemModel::default());
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
        let item_rows = vec![];
        let no_section_items = vec![];
        let section_items_map = std::collections::HashMap::new();
        let pinned_items = vec![];
        let overdue_items = vec![];
        let past_due_items = vec![];
        let sections = vec![];

        Self {
            focus_handle: cx.focus_handle(),
            _subscriptions: vec![],
            active_index: Some(0),
            item_rows,
            item_info,
            no_section_items,
            section_items_map,
            pinned_items,
            overdue_items,
            past_due_items,
            is_today_board: false,
            sections,
        }
    }

    /// 设置当前激活的索引
    pub fn set_active_index(&mut self, index: Option<usize>) {
        self.active_index = index;
    }

    /// 从 TodoStore 刷新项目列表（通用方法）
    ///
    /// # 参数
    /// - `items`: 要显示的项目列表
    /// - `window`: 窗口句柄
    /// - `cx`: 上下文
    /// - `filter`: 可选的过滤函数
    ///
    /// # 示例
    /// ```ignore
    /// let items = store.inbox_items_cached(cache);
    /// base.refresh_items(items, window, cx, Some(|item| !item.checked));
    /// ```
    pub fn refresh_items(
        &mut self,
        items: Vec<Arc<todos::entity::ItemModel>>,
        window: &mut Window,
        cx: &mut Context<impl gpui::Render>,
        filter: Option<fn(&Arc<todos::entity::ItemModel>) -> bool>,
    ) {
        let filtered_items: Vec<_> = match filter {
            Some(f) => items.into_iter().filter(f).collect(),
            None => items,
        };

        self.item_rows = filtered_items
            .iter()
            .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
            .collect();

        self.update_items_ordered(&filtered_items);
    }

    /// 检查版本号是否变化
    ///
    /// 用于优化观察者回调，避免不必要的更新
    pub fn check_version(&self, _store: &TodoStore) -> bool {
        true
    }

    /// 更新项目列表和部分映射
    pub fn update_items<T>(&mut self, items: &[T])
    where
        T: Into<Arc<todos::entity::ItemModel>> + Clone,
    {
        self.update_items_ordered(items);
    }

    /// 更新项目列表和部分映射，按照正确的顺序组织
    pub fn update_items_ordered<T>(&mut self, items: &[T])
    where
        T: Into<Arc<todos::entity::ItemModel>> + Clone,
    {
        self.pinned_items.clear();
        self.past_due_items.clear();
        self.overdue_items.clear();
        self.no_section_items.clear();
        self.section_items_map.clear();
        self.sections.clear();

        let mut past_due = vec![];
        let mut today_items = vec![];
        let mut non_pinned_non_overdue_no_section = vec![];
        let mut non_pinned_non_overdue_sections = std::collections::HashMap::new();

        for (i, item) in items.iter().enumerate() {
            let item_model: Arc<todos::entity::ItemModel> = item.clone().into();

            if item_model.pinned {
                self.pinned_items.push((i, item_model));
            } else if self.is_today_board && self.is_past_due(&item_model) {
                past_due.push((i, item_model));
            } else if self.is_today_board && item_model.is_due_today() {
                today_items.push((i, item_model));
            } else if self.is_today_board && item_model.due_date().is_none() {
                match item_model.section_id.as_deref() {
                    None | Some("") => non_pinned_non_overdue_no_section.push((i, item_model)),
                    Some(sid) => {
                        non_pinned_non_overdue_sections
                            .entry(sid.to_string())
                            .or_insert_with(Vec::new)
                            .push((i, item_model));
                    },
                }
            } else if !self.is_today_board {
                match item_model.section_id.as_deref() {
                    None | Some("") => non_pinned_non_overdue_no_section.push((i, item_model)),
                    Some(sid) => {
                        non_pinned_non_overdue_sections
                            .entry(sid.to_string())
                            .or_insert_with(Vec::new)
                            .push((i, item_model));
                    },
                }
            }
        }

        self.past_due_items = past_due;
        self.overdue_items = today_items;
        self.no_section_items = non_pinned_non_overdue_no_section;
        self.section_items_map = non_pinned_non_overdue_sections;

        if let Some(ix) = self.active_index {
            if ix >= self.item_rows.len() {
                self.active_index = if self.item_rows.is_empty() { None } else { Some(0) };
            }
        } else if !self.item_rows.is_empty() {
            self.active_index = Some(0);
        }
    }

    /// 检查任务是否为过去日期（超过今天）
    fn is_past_due(&self, item: &Arc<todos::entity::ItemModel>) -> bool {
        item.is_past_due()
    }
}

/// 用于通用渲染的 Board 视图 trait（可设置当前选中项索引）
pub trait BoardView: gpui::Render {
    fn set_active_index(&mut self, index: Option<usize>);
}

/// 所有 Board 类型的通用 trait
pub trait BoardCommon {
    /// 获取视图
    fn view(window: &mut Window, cx: &mut gpui::App) -> Entity<Self>
    where
        Self: Sized;

    /// 获取选中的项目
    fn get_selected_item(
        &self,
        ix: gpui_component::IndexPath,
        cx: &gpui::App,
    ) -> Option<std::rc::Rc<todos::entity::ItemModel>>;

    /// 显示项目对话框
    fn show_item_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        is_edit: bool,
        section_id: Option<String>,
    ) where
        Self: Sized;

    /// 显示项目删除对话框
    fn show_item_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>)
    where
        Self: Sized;
}

/// Board trait 的默认实现宏
#[macro_export]
macro_rules! impl_board_default {
    ($board:ident, $icon:expr, $colors:expr, $title:expr, $description:expr, $count_fn:expr) => {
        impl Board for $board {
            fn icon() -> IconName {
                $icon
            }

            fn colors() -> Vec<gpui::Hsla> {
                $colors
            }

            fn count(cx: &mut gpui::App) -> usize {
                $count_fn(cx)
            }

            fn title() -> &'static str {
                $title
            }

            fn description() -> &'static str {
                $description
            }

            fn zoomable() -> Option<gpui_component::dock::PanelControl> {
                None
            }

            fn new_view(window: &mut Window, cx: &mut gpui::App) -> Entity<impl gpui::Render> {
                Self::view(window, cx)
            }
        }

        impl gpui::Focusable for $board {
            fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
                self.focus_handle.clone()
            }
        }
    };
}
