use std::{cell::Cell, sync::Arc};

use gpui::{AppContext, Context, Entity, FocusHandle, Subscription, Window};
use gpui_component::{IndexPath, WindowExt};
use sea_orm::sqlx::types::uuid;
use todos::entity::SectionModel;

use crate::{
    ItemInfoState, ItemRowState,
    todo_actions::{add_section, update_section},
    todo_state::TodoStore,
};

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

    /// 从 item_rows 获取选中任务（与列表显示一致，避免重复查询 Store）
    pub fn get_selected_item_from_index(
        &self,
        ix: IndexPath,
        cx: &gpui::App,
    ) -> Option<Arc<todos::entity::ItemModel>> {
        self.item_rows
            .get(ix.row)
            .map(|row| row.read(cx).item.clone())
    }

    /// 显示新建/编辑任务对话框
    pub fn show_item_dialog<V: gpui::Render>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<V>,
        is_edit: bool,
        section_id: Option<String>,
    ) {
        let item_info = if is_edit {
            if let Some(active_index) = self.active_index {
                if let Some(item_row) = self.item_rows.get(active_index) {
                    item_row.update(cx, |row, cx| {
                        row.ensure_item_info(window, cx);
                    });
                    item_row
                        .read(cx)
                        .item_info
                        .clone()
                        .unwrap_or_else(|| self.item_info.clone())
                } else {
                    self.item_info.clone()
                }
            } else {
                self.item_info.clone()
            }
        } else {
            let mut ori_item = todos::entity::ItemModel::default();
            if let Some(sid) = section_id {
                ori_item.section_id = Some(sid);
            }
            self.item_info.update(cx, |state, cx| {
                state.set_item(Arc::new(ori_item.clone()), window, cx);
                cx.notify();
            });
            self.item_info.clone()
        };

        let config = crate::ui::components::ItemDialogConfig::new(
            if is_edit { "Edit Item" } else { "New Item" },
            if is_edit { "Save" } else { "Add" },
            is_edit,
        );

        crate::ui::components::show_item_dialog(window, cx, item_info, config, |_item, _cx| {});
    }

    /// 显示新建/编辑分区对话框
    pub fn show_section_dialog<V: gpui::Render>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<V>,
        section_id: Option<String>,
        is_edit: bool,
    ) {
        let sections = cx.global::<TodoStore>().sections.clone();
        let ori_section = if is_edit {
            sections
                .iter()
                .find(|s| s.id == section_id.clone().unwrap_or_default())
                .map(|s| s.as_ref().clone())
                .unwrap_or_default()
        } else {
            todos::entity::SectionModel::default()
        };

        let name_input = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx).placeholder("Section Name")
        });
        if is_edit {
            name_input.update(cx, |is, cx| {
                is.set_value(ori_section.name.clone(), window, cx);
                cx.notify();
            });
        }

        let config = crate::ui::components::SectionDialogConfig::new(
            if is_edit { "Edit Section" } else { "New Section" },
            if is_edit { "Save" } else { "Add" },
            is_edit,
        )
        .with_overlay(false);

        crate::ui::components::show_section_dialog(
            window,
            cx,
            name_input,
            config,
            move |name, cx| {
                let section = Arc::new(SectionModel { name, ..ori_section.clone() });
                if is_edit {
                    update_section(section, cx);
                } else {
                    add_section(section, cx);
                }
            },
        );
    }

    /// 显示删除分区确认对话框
    pub fn show_section_delete_dialog<V: gpui::Render>(
        window: &mut Window,
        cx: &mut Context<V>,
        section_id: String,
    ) {
        let sections = cx.global::<TodoStore>().sections.clone();
        let section_some = sections.iter().find(|s| s.id == section_id).cloned();
        if let Some(section) = section_some {
            crate::ui::components::show_section_delete_dialog(
                window,
                cx,
                "Are you sure to delete the section?",
                move |cx| {
                    crate::todo_actions::delete_section(section.clone(), cx);
                },
            );
        }
    }

    /// 复制分区
    pub fn duplicate_section<V: gpui::Render>(
        &self,
        window: &mut Window,
        cx: &mut Context<V>,
        section_id: String,
    ) {
        let sections = cx.global::<TodoStore>().sections.clone();
        if let Some(section) = sections.iter().find(|s| s.id == section_id) {
            let mut new_section = section.as_ref().clone();
            new_section.id = uuid::Uuid::new_v4().to_string();
            new_section.name = format!("{} (copy)", new_section.name);
            add_section(Arc::new(new_section), cx);
            window.push_notification("Section duplicated successfully.", cx);
        }
    }

    /// 归档分区
    pub fn archive_section<V: gpui::Render>(
        &self,
        window: &mut Window,
        cx: &mut Context<V>,
        section_id: String,
    ) {
        let sections = cx.global::<TodoStore>().sections.clone();
        if let Some(section) = sections.iter().find(|s| s.id == section_id) {
            let mut updated_section = section.as_ref().clone();
            updated_section.is_archived = true;
            update_section(Arc::new(updated_section), cx);
            window.push_notification("Section archived successfully.", cx);
        }
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

    /// 当前 ChangeMask 是否影响指定视图
    #[inline]
    pub fn store_change_affects(
        cx: &gpui::App,
        affects: fn(&crate::core::state::ChangeMask) -> bool,
    ) -> bool {
        affects(cx.global::<TodoStore>().peek_change_mask())
    }

    /// 首次渲染兜底：列表为空但 Store 已有数据时强制刷新一次
    pub fn bootstrap_pending_if_needed(
        pending_refresh: &Cell<bool>,
        item_rows_empty: bool,
        has_store_data: bool,
    ) {
        if !pending_refresh.get() && item_rows_empty && has_store_data {
            pending_refresh.set(true);
        }
    }

    /// 若有 pending 则清零并返回 true，否则返回 false
    pub fn take_pending_refresh(pending_refresh: &Cell<bool>) -> bool {
        if !pending_refresh.get() {
            return false;
        }
        pending_refresh.set(false);
        true
    }

    /// 延迟注册 TodoStore 观察者 + 首屏 bootstrap，并消费 pending。
    ///
    /// 返回 `true` 时调用方应继续执行本 Board 的刷新逻辑。
    /// 订阅会推入 `subscriptions`，避免 `observe_global` 立即 drop 失效。
    #[allow(clippy::too_many_arguments)]
    pub fn begin_pending_refresh<V: 'static>(
        observer_registered: &Cell<bool>,
        pending_refresh: &Cell<bool>,
        subscriptions: &mut Vec<Subscription>,
        item_rows_empty: bool,
        has_store_data: bool,
        cx: &mut Context<V>,
        affects: fn(&crate::core::state::ChangeMask) -> bool,
        pending_of: fn(&V) -> &Cell<bool>,
    ) -> bool {
        if !observer_registered.get() {
            observer_registered.set(true);
            let subscription = cx.observe_global::<TodoStore>(move |this, cx| {
                if Self::store_change_affects(cx, affects) {
                    pending_of(this).set(true);
                    cx.notify();
                }
            });
            subscriptions.push(subscription);
        }

        Self::bootstrap_pending_if_needed(pending_refresh, item_rows_empty, has_store_data);
        Self::take_pending_refresh(pending_refresh)
    }

    /// 修正 active_index，避免越界
    pub fn clamp_active_index(&mut self) {
        if let Some(ix) = self.active_index {
            if ix >= self.item_rows.len() {
                self.active_index = if self.item_rows.is_empty() { None } else { Some(0) };
            }
        } else if !self.item_rows.is_empty() {
            self.active_index = Some(0);
        }
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

    /// 🚀 6.5优化：Diff 更新 item_rows，避免全量重建 Entity
    ///
    /// 以 item.id 为 key，保留未变行的 Entity，仅插入/删除/移动变更项。
    /// 大幅减少大列表下的分配与订阅成本。
    ///
    /// 注意：此方法需要与 item_rows 同步维护的 item_id 列表配合使用。
    /// 调用方需要确保 `item_row_ids` 与 `item_rows` 一一对应。
    ///
    /// # 参数
    /// - `new_items`: 新的任务列表（已按显示顺序排列）
    /// - `item_row_ids`: 当前 item_rows 对应的 item id 列表（与 item_rows 一一对应）
    /// - `window`: 窗口句柄
    /// - `cx`: 上下文
    pub fn diff_update_item_rows(
        &mut self,
        new_items: &[Arc<todos::entity::ItemModel>],
        item_row_ids: &mut Vec<String>,
        window: &mut Window,
        cx: &mut Context<impl gpui::Render>,
    ) {
        use std::collections::HashMap;

        // 快速路径：如果当前为空，直接全量创建
        if self.item_rows.is_empty() {
            self.item_rows = new_items
                .iter()
                .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
                .collect();
            *item_row_ids = new_items.iter().map(|item| item.id.clone()).collect();
            return;
        }

        // 快速路径：如果新列表为空，直接清空
        if new_items.is_empty() {
            self.item_rows.clear();
            item_row_ids.clear();
            return;
        }

        // 1. 建立旧 item_rows 的 id -> Entity 映射
        let mut old_rows_map: HashMap<String, Entity<ItemRowState>> = HashMap::new();
        for (id, row) in item_row_ids.drain(..).zip(self.item_rows.drain(..)) {
            old_rows_map.insert(id, row);
        }

        // 2. 按新列表顺序重建 item_rows，复用已存在的 Entity
        let mut new_rows = Vec::with_capacity(new_items.len());
        let mut new_ids = Vec::with_capacity(new_items.len());
        for item in new_items.iter() {
            if let Some(old_entity) = old_rows_map.remove(&item.id) {
                // 复用旧 Entity（保留订阅和状态）
                new_rows.push(old_entity);
            } else {
                // 创建新 Entity
                new_rows.push(cx.new(|cx| ItemRowState::new(item.clone(), window, cx)));
            }
            new_ids.push(item.id.clone());
        }

        // 3. 剩余的 old_rows_map 中的 Entity 将被丢弃（自动释放）
        self.item_rows = new_rows;
        *item_row_ids = new_ids;
    }
}

/// 用于通用渲染的 Board 视图 trait（可设置当前选中项索引）
pub trait BoardView: gpui::Render {
    fn set_active_index(&mut self, index: Option<usize>);
}
