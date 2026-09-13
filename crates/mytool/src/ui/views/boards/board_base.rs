use std::{cell::Cell, collections::HashMap, sync::Arc};

use gpui::{App, AppContext, BorrowAppContext, Context, Entity, FocusHandle, Subscription, Window};
use gpui_component::{IndexPath, WindowExt};
use sea_orm::sqlx::types::uuid;
use todos::entity::SectionModel;

use crate::{
    ItemInfoState, ItemRowState,
    todo_actions::{add_section, update_section},
    todo_state::TodoStore,
};

/// 置顶项与分区的关系。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinnedLayout {
    /// 看板：置顶项只出现在置顶区，不再进入分区。
    Exclusive,
    /// 项目面板：置顶项同时出现在所属分区里。
    Inclusive,
}

/// 按置顶 / 过期 / 今日 / 分区拆好的任务分组。
#[derive(Default)]
pub struct GroupedItems {
    pub pinned: Vec<(usize, Arc<todos::entity::ItemModel>)>,
    pub past_due: Vec<(usize, Arc<todos::entity::ItemModel>)>,
    pub due_today: Vec<(usize, Arc<todos::entity::ItemModel>)>,
    pub no_section: Vec<(usize, Arc<todos::entity::ItemModel>)>,
    pub sections: HashMap<String, Vec<(usize, Arc<todos::entity::ItemModel>)>>,
}

/// 将任务列表按置顶策略与今日看板规则分组。
pub fn group_items(
    items: &[Arc<todos::entity::ItemModel>],
    pinned: PinnedLayout,
    today_board: bool,
) -> GroupedItems {
    let mut grouped = GroupedItems::default();

    let push_sectioned =
        |grouped: &mut GroupedItems, i: usize, item_model: Arc<todos::entity::ItemModel>| {
            match item_model.section_id.as_deref() {
                None | Some("") => grouped.no_section.push((i, item_model)),
                Some(sid) => {
                    grouped.sections.entry(sid.to_string()).or_default().push((i, item_model));
                },
            }
        };

    for (i, item) in items.iter().enumerate() {
        let item_model = item.clone();
        match pinned {
            PinnedLayout::Exclusive => {
                if item_model.pinned {
                    grouped.pinned.push((i, item_model));
                    continue;
                }
                if today_board && item_model.is_past_due() {
                    grouped.past_due.push((i, item_model));
                } else if today_board && item_model.is_due_today() {
                    grouped.due_today.push((i, item_model));
                } else if !today_board || item_model.due_date().is_none() {
                    push_sectioned(&mut grouped, i, item_model);
                }
            },
            PinnedLayout::Inclusive => {
                if !item_model.checked && item_model.pinned {
                    grouped.pinned.push((i, item_model.clone()));
                }
                push_sectioned(&mut grouped, i, item_model);
            },
        }
    }

    sort_item_pairs(&mut grouped.pinned);
    sort_item_pairs(&mut grouped.past_due);
    sort_item_pairs(&mut grouped.due_today);
    sort_item_pairs(&mut grouped.no_section);
    for items in grouped.sections.values_mut() {
        sort_item_pairs(items);
    }

    grouped
}

fn sort_item_pairs(items: &mut Vec<(usize, Arc<todos::entity::ItemModel>)>) {
    items.sort_by(|a, b| {
        a.1.child_order
            .unwrap_or(i32::MAX)
            .cmp(&b.1.child_order.unwrap_or(i32::MAX))
            .then_with(|| a.1.id.cmp(&b.1.id))
    });
}

/// 在同一分组、同一父任务下按 `child_order` 交换相邻项。
pub fn reorder_sibling_pairs(
    group: &[(usize, Arc<todos::entity::ItemModel>)],
    active_index: usize,
    delta: i32,
) -> Option<Vec<Arc<todos::entity::ItemModel>>> {
    let active = group.iter().find(|(i, _)| *i == active_index)?.1.clone();
    let parent = active.parent_id.clone();
    let mut siblings: Vec<(usize, Arc<todos::entity::ItemModel>)> =
        group.iter().filter(|(_, item)| item.parent_id == parent).cloned().collect();
    sort_item_pairs(&mut siblings);
    let pos = siblings.iter().position(|(i, _)| *i == active_index)?;
    let new_pos = i32::try_from(pos).ok()?.saturating_add(delta);
    if new_pos < 0 || new_pos >= siblings.len() as i32 {
        return None;
    }
    siblings.swap(pos, new_pos as usize);
    Some(
        siblings
            .into_iter()
            .enumerate()
            .map(|(ord, (_, item))| {
                let mut item = (*item).clone();
                item.child_order = Some(ord as i32);
                Arc::new(item)
            })
            .collect(),
    )
}

/// 在多个分组中找到包含 `active_index` 的那一组再排序。
pub fn reorder_in_groups(
    groups: &[&[(usize, Arc<todos::entity::ItemModel>)]],
    active_index: usize,
    delta: i32,
) -> Option<Vec<Arc<todos::entity::ItemModel>>> {
    for group in groups {
        if group.iter().any(|(i, _)| *i == active_index) {
            return reorder_sibling_pairs(group, active_index, delta);
        }
    }
    None
}

/// 修正选中下标，避免越界；未选中时保持未选中。
pub fn clamp_active_index(active_index: &mut Option<usize>, len: usize) {
    if let Some(ix) = *active_index {
        if len == 0 || ix >= len {
            *active_index = None;
        }
    }
}

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
    /// 今日到期任务（仅 Today Board 使用）
    pub due_today_items: Vec<(usize, Arc<todos::entity::ItemModel>)>,
    /// 过期任务分组（超过今天但还未完成，仅 Today Board 使用）
    pub past_due_items: Vec<(usize, Arc<todos::entity::ItemModel>)>,
    pub is_today_board: bool,
    pub item_row_ids: Vec<String>,
    pub pending_refresh: Cell<bool>,
    pub observer_registered: Cell<bool>,
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
        let due_today_items = vec![];
        let past_due_items = vec![];

        Self {
            focus_handle: cx.focus_handle(),
            _subscriptions: vec![],
            active_index: None,
            item_rows,
            item_info,
            no_section_items,
            section_items_map,
            pinned_items,
            due_today_items,
            past_due_items,
            is_today_board: false,
            item_row_ids: Vec::new(),
            pending_refresh: Cell::new(false),
            observer_registered: Cell::new(false),
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
        self.item_rows.get(ix.row).map(|row| row.read(cx).item.clone())
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
                    item_row.read(cx).item_info.clone().unwrap_or_else(|| self.item_info.clone())
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
            if is_edit { "编辑任务" } else { "新建任务" },
            if is_edit { "保存" } else { "添加" },
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
        let ori_section = if is_edit {
            section_id
                .as_deref()
                .and_then(|id| cx.global::<TodoStore>().get_section(id))
                .map(|s| s.as_ref().clone())
                .unwrap_or_default()
        } else {
            todos::entity::SectionModel::default()
        };

        let name_input =
            cx.new(|cx| gpui_component::input::InputState::new(window, cx).placeholder("分区名称"));
        if is_edit {
            name_input.update(cx, |is, cx| {
                is.set_value(ori_section.name.clone(), window, cx);
                cx.notify();
            });
        }

        let config = crate::ui::components::SectionDialogConfig::new(
            if is_edit { "编辑分区" } else { "新建分区" },
            if is_edit { "保存" } else { "添加" },
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
        if let Some(section) = cx.global::<TodoStore>().get_section(&section_id) {
            crate::ui::components::show_section_delete_dialog(
                window,
                cx,
                "确定删除这个分区吗？",
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
        if let Some(section) = cx.global::<TodoStore>().get_section(&section_id) {
            let mut new_section = section.as_ref().clone();
            new_section.id = uuid::Uuid::new_v4().to_string();
            new_section.name = format!("{}（副本）", new_section.name);
            add_section(Arc::new(new_section), cx);
            window.push_notification("已复制分区。", cx);
        }
    }

    /// 归档分区
    pub fn archive_section<V: gpui::Render>(
        &self,
        window: &mut Window,
        cx: &mut Context<V>,
        section_id: String,
    ) {
        if let Some(section) = cx.global::<TodoStore>().get_section(&section_id) {
            let mut updated_section = section.as_ref().clone();
            updated_section.is_archived = true;
            update_section(Arc::new(updated_section), cx);
            window.push_notification("已归档分区。", cx);
        }
    }

    /// Diff 更新行实体并按当前 Board 规则分组。
    pub fn refresh_from_items(
        &mut self,
        items: &[Arc<todos::entity::ItemModel>],
        window: &mut Window,
        cx: &mut Context<impl gpui::Render>,
    ) {
        diff_update_item_rows(&mut self.item_rows, &mut self.item_row_ids, items, window, cx);
        self.update_items(items);
    }

    /// 统一看板刷新：延迟注册观察者、bootstrap、diff + 分组。
    ///
    /// 返回本次使用的列表；若无需刷新则 `None`。
    pub fn apply_store_refresh<V: gpui::Render + 'static>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<V>,
        affects: fn(&crate::core::state::ChangeMask) -> bool,
        pending_of: fn(&V) -> &Cell<bool>,
        fetch: impl Fn(&App) -> Arc<Vec<Arc<todos::entity::ItemModel>>>,
    ) -> Option<Arc<Vec<Arc<todos::entity::ItemModel>>>> {
        let has_store_data = if !self.pending_refresh.get() && self.item_rows.is_empty() {
            !fetch(cx).is_empty()
        } else {
            false
        };

        if !Self::begin_pending_refresh(
            &self.observer_registered,
            &self.pending_refresh,
            &mut self._subscriptions,
            self.item_rows.is_empty(),
            has_store_data,
            cx,
            affects,
            pending_of,
        ) {
            return None;
        }

        let items = fetch(cx);
        self.refresh_from_items(items.as_slice(), window, cx);
        Some(items)
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
            subscriptions.push(cx.observe_global::<crate::core::state::ItemSelection>(|_, cx| {
                cx.notify();
            }));
        }

        Self::bootstrap_pending_if_needed(pending_refresh, item_rows_empty, has_store_data);
        Self::take_pending_refresh(pending_refresh)
    }

    /// 当前看板各分组（用于上移/下移）。
    pub fn reorder_groups(&self) -> Vec<&[(usize, Arc<todos::entity::ItemModel>)]> {
        let mut groups: Vec<&[(usize, Arc<todos::entity::ItemModel>)]> = vec![
            &self.pinned_items,
            &self.past_due_items,
            &self.due_today_items,
            &self.no_section_items,
        ];
        for items in self.section_items_map.values() {
            groups.push(items.as_slice());
        }
        groups
    }

    /// 在当前分组内上移/下移激活任务，并写回 `child_order`。
    pub fn reorder_active<V: gpui::Render>(
        &self,
        delta: i32,
        _window: &mut Window,
        cx: &mut Context<V>,
    ) {
        let Some(active_index) = self.active_index else {
            return;
        };
        let groups = self.reorder_groups();
        let Some(updated) = reorder_in_groups(&groups, active_index, delta) else {
            return;
        };
        crate::core::actions::batch::batch_update_items(updated, cx);
    }

    /// 点击看板空白处时收起展开的任务，并取消选中
    pub fn on_background_click<V: gpui::Render>(&mut self, cx: &mut Context<V>) {
        self.collapse_open_rows(cx);
        let had_active = self.active_index.take().is_some();
        let had_multi = !cx.global::<crate::core::state::ItemSelection>().is_empty();
        if had_multi {
            cx.update_global::<crate::core::state::ItemSelection, _>(|sel, _| sel.clear());
        }
        if had_active || had_multi {
            cx.notify();
        }
    }

    /// 点击看板空白处时保存并收起所有已展开的任务
    pub fn collapse_open_rows<V: gpui::Render>(&self, cx: &mut Context<V>) {
        for row in &self.item_rows {
            row.update(cx, |row, cx| {
                row.collapse_if_open(cx);
            });
        }
    }

    /// 修正 active_index，避免越界
    pub fn clamp_active_index(&mut self) {
        clamp_active_index(&mut self.active_index, self.item_rows.len());
    }

    /// 更新项目列表和部分映射
    pub fn update_items(&mut self, items: &[Arc<todos::entity::ItemModel>]) {
        let grouped = group_items(items, PinnedLayout::Exclusive, self.is_today_board);
        self.pinned_items = grouped.pinned;
        self.past_due_items = grouped.past_due;
        self.due_today_items = grouped.due_today;
        self.no_section_items = grouped.no_section;
        self.section_items_map = grouped.sections;
        self.clamp_active_index();
    }

    /// 以 item.id 为 key 复用行实体，仅插入/删除/同步变更项。
    pub fn diff_update_item_rows(
        &mut self,
        new_items: &[Arc<todos::entity::ItemModel>],
        window: &mut Window,
        cx: &mut Context<impl gpui::Render>,
    ) {
        diff_update_item_rows(&mut self.item_rows, &mut self.item_row_ids, new_items, window, cx);
    }
}

/// 以 item.id 为 key，保留未变行的 Entity，并向复用行推送最新数据。
pub fn diff_update_item_rows<V: gpui::Render>(
    item_rows: &mut Vec<Entity<ItemRowState>>,
    item_row_ids: &mut Vec<String>,
    new_items: &[Arc<todos::entity::ItemModel>],
    window: &mut Window,
    cx: &mut Context<V>,
) {
    use std::collections::HashMap;

    if item_rows.is_empty() {
        *item_rows = new_items
            .iter()
            .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
            .collect();
        *item_row_ids = new_items.iter().map(|item| item.id.clone()).collect();
        return;
    }

    if new_items.is_empty() {
        item_rows.clear();
        item_row_ids.clear();
        return;
    }

    let mut old_rows_map: HashMap<String, Entity<ItemRowState>> = HashMap::new();
    for (id, row) in item_row_ids.drain(..).zip(item_rows.drain(..)) {
        old_rows_map.insert(id, row);
    }

    let mut new_rows = Vec::with_capacity(new_items.len());
    let mut new_ids = Vec::with_capacity(new_items.len());
    for item in new_items.iter() {
        let reuse_key = if old_rows_map.contains_key(&item.id) {
            Some(item.id.clone())
        } else {
            cx.global::<TodoStore>().temp_id_for(&item.id).cloned()
        };
        if let Some(old_id) = reuse_key
            && let Some(old_entity) = old_rows_map.remove(&old_id)
        {
            old_entity.update(cx, |row, cx| {
                row.sync_item(item.clone(), window, cx);
            });
            new_rows.push(old_entity);
        } else {
            new_rows.push(cx.new(|cx| ItemRowState::new(item.clone(), window, cx)));
        }
        new_ids.push(item.id.clone());
    }

    *item_rows = new_rows;
    *item_row_ids = new_ids;
}

/// 用于通用渲染的 Board 视图 trait（可设置当前选中项索引）
pub trait BoardView: gpui::Render {
    fn set_active_index(&mut self, index: Option<usize>);
}

#[cfg(test)]
mod tests {
    use todos::entity::ItemModel;

    use super::*;

    fn item(id: &str, pinned: bool, checked: bool, section: Option<&str>) -> Arc<ItemModel> {
        let mut model = ItemModel::default();
        model.id = id.to_string();
        model.pinned = pinned;
        model.checked = checked;
        model.section_id = section.map(str::to_string);
        Arc::new(model)
    }

    #[test]
    fn exclusive_pinned_stays_out_of_sections() {
        let items = vec![
            item("p", true, false, Some("s1")),
            item("s", false, false, Some("s1")),
            item("n", false, false, None),
        ];
        let grouped = group_items(&items, PinnedLayout::Exclusive, false);
        assert_eq!(grouped.pinned.len(), 1);
        assert_eq!(grouped.pinned[0].1.id, "p");
        assert_eq!(grouped.no_section.len(), 1);
        assert_eq!(grouped.sections.get("s1").map(|v| v.len()), Some(1));
        assert_eq!(grouped.sections.get("s1").unwrap()[0].1.id, "s");
    }

    #[test]
    fn inclusive_pinned_also_appears_in_section() {
        let items = vec![item("p", true, false, Some("s1")), item("done", true, true, Some("s1"))];
        let grouped = group_items(&items, PinnedLayout::Inclusive, false);
        assert_eq!(grouped.pinned.len(), 1);
        assert_eq!(grouped.pinned[0].1.id, "p");
        let section = grouped.sections.get("s1").expect("section s1");
        assert_eq!(section.len(), 2);
    }

    #[test]
    fn clamp_active_index_handles_empty_and_overflow() {
        let mut idx = Some(3);
        clamp_active_index(&mut idx, 0);
        assert_eq!(idx, None);

        idx = Some(5);
        clamp_active_index(&mut idx, 2);
        assert_eq!(idx, None);

        idx = None;
        clamp_active_index(&mut idx, 2);
        assert_eq!(idx, None);
    }

    #[test]
    fn reorder_swaps_child_order_among_siblings() {
        let mut a = item("a", false, false, None);
        let mut b = item("b", false, false, None);
        Arc::make_mut(&mut a).child_order = Some(0);
        Arc::make_mut(&mut b).child_order = Some(1);
        let group = vec![(0, a), (1, b)];
        let updated = reorder_sibling_pairs(&group, 0, 1).expect("swap");
        assert_eq!(updated[0].id, "b");
        assert_eq!(updated[0].child_order, Some(0));
        assert_eq!(updated[1].id, "a");
        assert_eq!(updated[1].child_order, Some(1));
    }

    #[test]
    fn group_items_sorts_by_child_order() {
        let mut first = item("z", false, false, None);
        let mut second = item("a", false, false, None);
        Arc::make_mut(&mut first).child_order = Some(0);
        Arc::make_mut(&mut second).child_order = Some(1);
        let grouped = group_items(&[second, first], PinnedLayout::Exclusive, false);
        assert_eq!(grouped.no_section[0].1.id, "z");
        assert_eq!(grouped.no_section[1].1.id, "a");
    }
}
