use gpui::{AppContext, Context, Entity, FocusHandle, Subscription, Window};

use crate::{ItemInfoState, ItemRowState};

/// 所有 Board 类型的基础结构体
pub struct BoardBase {
    pub _subscriptions: Vec<Subscription>,
    pub focus_handle: FocusHandle,
    pub active_index: Option<usize>,
    pub item_rows: Vec<Entity<ItemRowState>>,
    pub item_info: Entity<ItemInfoState>,
    pub no_section_items: Vec<(usize, std::sync::Arc<todos::entity::ItemModel>)>,
    pub section_items_map:
        std::collections::HashMap<String, Vec<(usize, std::sync::Arc<todos::entity::ItemModel>)>>,
}

impl BoardBase {
    /// 创建一个新的 BoardBase 实例
    pub fn new(window: &mut Window, cx: &mut Context<impl gpui::Render>) -> Self {
        let item = std::sync::Arc::new(todos::entity::ItemModel::default());
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
        let item_rows = vec![];
        let no_section_items = vec![];
        let section_items_map = std::collections::HashMap::new();

        Self {
            focus_handle: cx.focus_handle(),
            _subscriptions: vec![],
            active_index: Some(0),
            item_rows,
            item_info,
            no_section_items,
            section_items_map,
        }
    }

    /// 更新项目列表和部分映射
    pub fn update_items<T>(&mut self, items: &[T])
    where
        T: Into<std::sync::Arc<todos::entity::ItemModel>> + Clone,
    {
        // 重新计算no_section_items和section_items_map
        self.no_section_items.clear();
        self.section_items_map.clear();

        for (i, item) in items.iter().enumerate() {
            let item_model: std::sync::Arc<todos::entity::ItemModel> = item.clone().into();
            match item_model.section_id.as_deref() {
                None | Some("") => self.no_section_items.push((i, item_model)),
                Some(sid) => {
                    self.section_items_map
                        .entry(sid.to_string())
                        .or_default()
                        .push((i, item_model));
                },
            }
        }

        // 更新活动索引
        if let Some(ix) = self.active_index {
            if ix >= self.item_rows.len() {
                self.active_index = if self.item_rows.is_empty() { None } else { Some(0) };
            }
        } else if !self.item_rows.is_empty() {
            self.active_index = Some(0);
        }
    }
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
