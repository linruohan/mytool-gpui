use std::sync::Arc;

use gpui::{
    App, AppContext, BorrowAppContext, Context, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce,
    StatefulInteractiveElement, StyleRefinement, Styled, Subscription, Window, div,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Sizable, Size, StyledExt as _, collapsible::Collapsible, h_flex,
};
use todos::{entity::ItemModel, enums::item_priority::ItemPriority};

use crate::{
    ItemInfo, ItemInfoEvent, ItemInfoState, ItemListItem, SemanticColors,
    todo_actions::set_item_pinned_optimistic, todo_state::TodoStore,
};

const CONTEXT: &str = "ItemRow";

#[derive(Clone)]
pub enum ItemRowEvent {
    Updated(Arc<ItemModel>),    // 更新任务
    Added(Arc<ItemModel>),      // 新增任务
    Finished(Arc<ItemModel>),   // 状态改为完成
    UnFinished(Arc<ItemModel>), // 状态改为未完成
    Deleted(Arc<ItemModel>),    // 删除任务
    FocusRequested,             // 请求焦点
}

pub struct ItemRowState {
    pub item: Arc<ItemModel>,
    pub item_info: Option<Entity<ItemInfoState>>,
    is_open: bool,
    is_focused: bool,          // 焦点状态
    focus_handle: FocusHandle, // 焦点句柄
    _subscriptions: Vec<Subscription>,
    update_version: usize, // 用于强制重新渲染 ItemListItem
}

impl EventEmitter<ItemRowEvent> for ItemRowState {}

impl Focusable for ItemRowState {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ItemRowState {
    pub fn new(item: Arc<ItemModel>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            item,
            item_info: None,
            is_open: false,
            is_focused: false,
            focus_handle,
            _subscriptions: Vec::new(),
            update_version: 0,
        }
    }

    /// 由父列表在 diff 复用 Entity 时推送最新数据，避免每行订阅全局 Store。
    pub fn sync_item(&mut self, item: Arc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) {
        if Arc::ptr_eq(&self.item, &item) {
            return;
        }

        if let Some(item_info) = self.item_info.as_ref() {
            let pending = {
                let info = item_info.read(cx);
                info.state_manager.is_dirty()
                    || info.state_manager.save_status == crate::SaveItemStatus::Saving
            };
            if pending {
                // 编辑器里还有未落盘的改动：不要用 Store 旧数据覆盖，否则随后保存会当成“无修改”
                let local_id = item_info.read(cx).state_manager.item.id.clone();
                if local_id != item.id && local_id.starts_with("temp_") {
                    item_info.update(cx, |info, _cx| {
                        info.state_manager.update_item(|local| {
                            local.id = item.id.clone();
                        });
                    });
                }
                self.item = item_info.read(cx).state_manager.item.clone();
                cx.notify();
                return;
            }
        }

        if self.item.display_eq(&item) {
            self.item = item;
            return;
        }
        let is_label_update = self.item.labels != item.labels;
        self.item = item.clone();
        self.update_version += 1;
        if let Some(item_info) = self.item_info.as_ref() {
            item_info.update(cx, |this_info, cx| {
                this_info.update_item_without_reloading_labels(item.clone(), window, cx);
                if is_label_update {
                    this_info.refresh_labels_selection_from_item(cx);
                }
            });
        }
        cx.notify();
    }

    /// 首次展开/编辑时懒创建 ItemInfoState 并订阅其事件
    pub fn ensure_item_info(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<ItemInfoState> {
        if let Some(ref entity) = self.item_info {
            return entity.clone();
        }

        let item_info = cx.new(|cx| ItemInfoState::new(self.item.clone(), window, cx));
        let subscription = cx.subscribe(&item_info, |this, _, event: &ItemInfoEvent, cx| {
            match event {
                ItemInfoEvent::Cancelled() => {
                    let is_new = this.item.id.is_empty() || this.item.id.starts_with("temp_");
                    if is_new {
                        cx.update_global::<TodoStore, _>(|store, _| {
                            store.remove_item(&this.item.id);
                        });
                    }
                    this.is_open = false;
                    cx.notify();
                    return;
                },
                ItemInfoEvent::Collapse() => {
                    this.flush_and_save(cx);
                    this.is_open = false;
                    cx.notify();
                    return;
                },
                ItemInfoEvent::Deleted() => {
                    this.is_open = false;
                },
                _ => {},
            }

            if let Some(ref item_info) = this.item_info {
                item_info.update(cx, |state, cx| {
                    state.handle_item_info_event(event, cx);
                });
                let latest_item = item_info.read(cx).state_manager.item.clone();

                this.item = latest_item.clone();
                this.update_version += 1;

                cx.notify();
            }
        });
        self._subscriptions.push(subscription);
        self.item_info = Some(item_info.clone());
        item_info
    }

    /// 展开详情面板（可选刷新标签选择状态）
    fn open_detail(&mut self, window: &mut Window, cx: &mut Context<Self>, refresh_labels: bool) {
        if self.is_open {
            return;
        }
        self.is_open = true;
        let item_info = self.ensure_item_info(window, cx);
        item_info.update(cx, |state, cx| {
            state.focus_name_input(window, cx);
            if refresh_labels && !state.state_manager.is_dirty() {
                state.refresh_labels_selection_from_item(cx);
            }
        });
        cx.notify();
    }

    /// 收起前把输入框内容刷进状态并保存，避免折叠后列表刷新冲掉未保存编辑
    fn flush_and_save(&mut self, cx: &mut Context<Self>) {
        if let Some(item_info) = self.item_info.as_ref() {
            item_info.update(cx, |state, cx| {
                state.save_all_changes(cx);
            });
            self.item = item_info.read(cx).state_manager.item.clone();
            self.update_version += 1;
        }
    }

    /// 若已展开则保存并收起（点击空白处时由看板调用）
    pub fn collapse_if_open(&mut self, cx: &mut Context<Self>) {
        if !self.is_open {
            return;
        }
        self.flush_and_save(cx);
        self.is_open = false;
        cx.notify();
    }

    /// 切换展开/收起状态
    fn toggle_expand(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_open {
            self.flush_and_save(cx);
            self.is_open = false;
            cx.notify();
        } else {
            self.open_detail(window, cx, true);
        }
    }

    // ==================== 快捷键处理方法 ====================

    /// 处理删除任务快捷键 (Cmd/Ctrl + D)
    fn handle_delete_shortcut(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let item_info = self.ensure_item_info(window, cx);
        item_info.update(cx, |_state, cx| {
            cx.emit(ItemInfoEvent::Deleted());
        });
        true
    }

    /// 处理切换置顶快捷键 (Cmd/Ctrl + P)
    fn handle_toggle_pin_shortcut(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let new_pinned = !self.item.pinned;
        let item_info = self.ensure_item_info(window, cx);
        item_info.update(cx, |state, _cx| {
            state.state_manager.set_pinned(new_pinned);
        });
        set_item_pinned_optimistic(self.item.clone(), new_pinned, cx);
        Arc::make_mut(&mut self.item).pinned = new_pinned;
        self.update_version += 1;
        cx.notify();
        true
    }

    /// 处理切换完成状态快捷键 (Space)
    fn handle_toggle_complete_shortcut(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let new_checked = !self.item.checked;
        let item_info = self.ensure_item_info(window, cx);
        item_info.update(cx, |state, cx| {
            state.state_manager.set_completed(new_checked);
            if new_checked {
                cx.emit(ItemInfoEvent::Finished());
            } else {
                cx.emit(ItemInfoEvent::UnFinished());
            }
        });
        true
    }

    /// 处理展开编辑快捷键 (Cmd/Ctrl + E)
    fn handle_edit_shortcut(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.is_open = true;
        self.ensure_item_info(window, cx);
        cx.notify();
        true
    }

    /// 处理收起并取消快捷键 (Escape)
    fn handle_escape_shortcut(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.is_open {
            if let Some(item_info) = self.item_info.as_ref() {
                item_info.update(cx, |_state, cx| {
                    cx.emit(ItemInfoEvent::Cancelled());
                });
            }
            self.is_open = false;
            cx.notify();
        }
        true
    }

    /// 处理键盘事件（优化后的版本）
    fn handle_key_event(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let is_cmd = event.keystroke.modifiers == gpui::Modifiers::command();
        let is_plain = event.keystroke.modifiers == gpui::Modifiers::default();
        let key = event.keystroke.key.as_str();

        match (key, is_cmd) {
            ("d", true) => return self.handle_delete_shortcut(window, cx),
            ("p", true) => return self.handle_toggle_pin_shortcut(window, cx),
            _ => {},
        }

        if self.is_open {
            match (key, is_plain) {
                ("enter", true) => {
                    self.toggle_expand(window, cx);
                    return true;
                },
                ("escape", _) => return self.handle_escape_shortcut(window, cx),
                _ => {},
            }
        } else {
            match (key, is_plain, is_cmd) {
                ("enter", true, _) => {
                    self.toggle_expand(window, cx);
                    return true;
                },
                ("space", true, _) => return self.handle_toggle_complete_shortcut(window, cx),
                ("e", _, true) => return self.handle_edit_shortcut(window, cx),
                _ => {},
            }
        }

        false
    }
}

impl Render for ItemRowState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        self.is_focused = self.focus_handle.is_focused(window);

        let text_color =
            if self.is_open { cx.theme().accent_foreground } else { cx.theme().foreground };

        let item = self.item.clone();
        let is_open = self.is_open;
        let is_focused = self.is_focused;
        let item_id = format!("item-{}", item.id);
        let version = self.update_version;

        let colors = SemanticColors::from_theme(cx);
        let hover_bg = colors.hover_overlay;
        let priority = item.priority.unwrap_or(4);
        let priority_color = gpui::rgb(ItemPriority::from_i32(priority).get_color());
        let completed_opacity = if item.checked { 0.65 } else { 1.0 };
        let left_border_width = match priority {
            1 => px(4.0),
            2 => px(3.0),
            3 => px(2.0),
            _ => px(0.0),
        };
        let row_bg = colors.priority_background_tint(priority, cx.theme().background);
        let item_info_entity = self.item_info.clone();
        let card_bg = cx.theme().background;
        let card_border = cx.theme().border;

        div()
            .id(item_id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .border_l(left_border_width)
            .border_color(priority_color)
            .when(is_open, |this| {
                this.rounded(px(8.0))
                    .p(px(8.0))
                    .my(px(4.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
            })
            .when(!is_open, |this| {
                this.rounded(px(6.0))
                    .px(px(4.0))
                    .py(px(2.0))
                    .bg(row_bg)
                    .opacity(completed_opacity)
                    .hover(move |style: StyleRefinement| style.bg(hover_bg).cursor_pointer())
            })
            .when(is_focused && !is_open, |this| this.shadow_sm())
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(|this, event, window, cx| {
                if this.handle_key_event(event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .child(
                Collapsible::new()
                    .gap_1()
                    .open(is_open)
                    .child(if is_open {
                        div().id("item-title-bar").h(px(0.)).into_any_element()
                    } else {
                        h_flex()
                            .id("item-title-bar")
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap(px(4.0))
                            .text_color(text_color)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.toggle_expand(window, cx);
                                this.focus_handle.focus(window, cx);
                            }))
                            .child(ItemListItem::new(
                                format!("{}-{}", item_id, version),
                                item.clone(),
                                is_focused,
                            ))
                            .into_any_element()
                    })
                    .when_some(item_info_entity.filter(|_| is_open), |collapsible, item_info| {
                        collapsible.content(ItemInfo::new(&item_info))
                    }),
            )
    }
}

#[derive(IntoElement)]
pub struct ItemRow {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<ItemRowState>,
}

impl Sizable for ItemRow {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for ItemRow {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ItemRow {
    pub fn new(state: &Entity<ItemRowState>) -> Self {
        Self {
            id: ("item-info", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for ItemRow {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .w_full()
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
