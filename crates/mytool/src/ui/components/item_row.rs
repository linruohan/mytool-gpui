use std::sync::Arc;

use gpui::{
    App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, StyleRefinement,
    Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable, Size, StyledExt as _, button::Button, collapsible::Collapsible,
    h_flex, v_flex,
};
use todos::entity::ItemModel;

use crate::{
    ItemInfo, ItemInfoEvent, ItemInfoState, ItemListItem, SemanticColors, todo_state::TodoStore,
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
    pub item_info: Entity<ItemInfoState>,
    is_open: bool,
    is_hovered: bool,          // 悬停状态
    is_focused: bool,          // 焦点状态
    focus_handle: FocusHandle, // 焦点句柄
    _subscriptions: Vec<Subscription>,
    update_version: usize,       // 用于强制重新渲染 ItemListItem
    cached_store_version: usize, // 缓存的 TodoStore 版本号，用于优化性能
}

impl EventEmitter<ItemRowEvent> for ItemRowState {}

impl Focusable for ItemRowState {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ItemRowState {
    pub fn new(item: Arc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
        let item_id = item.id.clone();
        let focus_handle = cx.focus_handle();

        let _subscriptions = vec![
            cx.observe_global_in::<TodoStore>(window, move |this, window, cx| {
                let store = cx.global::<TodoStore>();

                // 性能优化：检查版本号，只在数据变化时更新
                if this.cached_store_version == store.version() {
                    return;
                }
                this.cached_store_version = store.version();

                let state_items = store.all_items.clone();
                if let Some(updated_item) = state_items.iter().find(|i| i.id == item_id) {
                    this.item = updated_item.clone();
                    this.update_version += 1; // 增加版本号，强制重新渲染
                    this.item_info.update(cx, |this_info, cx| {
                        this_info.set_item(updated_item.clone(), window, cx);
                    });
                    cx.notify();
                }
            }),
            cx.subscribe(&item_info, |this, _, event: &ItemInfoEvent, cx| {
                this.item_info.update(cx, |state, cx| {
                    state.handle_item_info_event(event, cx);
                });
                // 直接从 item_info 中获取最新的 item，确保及时更新
                let latest_item = this.item_info.read(cx).state_manager.item.clone();
                this.item = latest_item;
                this.update_version += 1; // 增加版本号，强制重新渲染
                cx.notify();
            }),
        ];

        Self {
            item,
            item_info,
            is_open: false,
            is_hovered: false,
            is_focused: false,
            focus_handle,
            _subscriptions,
            update_version: 0,
            cached_store_version: 0,
        }
    }

    /// 保存所有修改
    fn save_all_changes(&mut self, cx: &mut Context<Self>) {
        self.item_info.update(cx, |state, cx| {
            state.save_all_changes(cx);
        });
    }

    /// 切换展开/收起状态
    fn toggle_expand(&mut self, cx: &mut Context<Self>) {
        // 如果当前是展开状态，收缩时保存所有修改
        if self.is_open {
            self.save_all_changes(cx);
        }
        self.is_open = !self.is_open;
        cx.notify();
    }

    /// 处理键盘事件
    fn handle_key_event(&mut self, event: &gpui::KeyDownEvent, cx: &mut Context<Self>) -> bool {
        match event.keystroke.key.as_str() {
            "enter" => {
                // Enter 键：切换展开/收起
                self.toggle_expand(cx);
                true
            },
            "space" => {
                // 空格键：切换完成状态
                if !self.is_open {
                    let new_checked = !self.item.checked;
                    self.item_info.update(cx, |state, cx| {
                        state.state_manager.set_completed(new_checked);
                        if new_checked {
                            cx.emit(ItemInfoEvent::Finished());
                        } else {
                            cx.emit(ItemInfoEvent::UnFinished());
                        }
                    });
                    true
                } else {
                    false
                }
            },
            "e" if event.keystroke.modifiers == gpui::Modifiers::command() => {
                // Cmd/Ctrl + E：编辑任务（展开）
                if !self.is_open {
                    self.is_open = true;
                    cx.notify();
                }
                true
            },
            "d" if event.keystroke.modifiers == gpui::Modifiers::command() => {
                // Cmd/Ctrl + D：删除任务
                self.item_info.update(cx, |_state, cx| {
                    cx.emit(ItemInfoEvent::Deleted());
                });
                true
            },
            "p" if event.keystroke.modifiers == gpui::Modifiers::command() => {
                // Cmd/Ctrl + P：切换置顶
                let new_pinned = !self.item.pinned;
                self.item_info.update(cx, |state, cx| {
                    state.state_manager.set_pinned(new_pinned);
                    cx.emit(ItemInfoEvent::Updated());
                });
                true
            },
            _ => false,
        }
    }
}

impl Render for ItemRowState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        // 更新焦点状态
        self.is_focused = self.focus_handle.is_focused(window);

        let text_color =
            if self.is_open { cx.theme().accent_foreground } else { cx.theme().foreground };

        // 从 item_info 中获取最新的 item，确保显示最新的数据
        let item = self.item_info.read(cx).state_manager.item.clone();
        let item_info = self.item_info.clone();
        let is_open = self.is_open;
        let is_focused = self.is_focused;
        let item_id = format!("item-{}", item.id);
        let view = cx.entity();
        let version = self.update_version; // 获取当前版本号

        // 获取语义化颜色
        let colors = SemanticColors::from_theme(cx);
        // 将 Option<i32> 转换为 u8，默认为 0（无优先级）
        let priority = item.priority.unwrap_or(0).clamp(0, 3) as u8;
        let priority_color = colors.priority_color(priority);

        // 根据任务状态选择状态颜色
        let status_indicator = if item.checked {
            Some(colors.status_completed)
        } else if item.pinned {
            Some(colors.status_pinned)
        } else {
            None
        };

        // 完成状态的视觉效果
        let completed_opacity = if item.checked { 0.6 } else { 1.0 };

        // 焦点状态的边框颜色
        let border_color = if is_focused { cx.theme().accent } else { cx.theme().border };

        div()
            .id(item_id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            // 应用视觉层次：圆角和间距
            .rounded(px(8.0))
            .p(px(12.0))
            .my(px(4.0))  // 添加垂直间距
            // 边框样式
            .border_1()
            .border_color(border_color)
            // 优先级指示器：左侧彩色边框
            .border_l_4()
            .border_color(priority_color)
            // 背景色
            .bg(cx.theme().background)
            // 完成状态的透明度
            .opacity(completed_opacity)
            // 焦点环效果
            .when(is_focused, |this| {
                this.shadow_md()
                    .border_color(cx.theme().accent)
            })
            // 悬停效果：提升视觉层次
            .on_mouse_move(cx.listener(|this, _event, _window, cx| {
                this.is_hovered = true;
                cx.notify();
            }))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _event, window, cx| {
                this.focus_handle.focus(window, cx);
                cx.notify();
            }))
            .hover(|style| {
                style
                    .bg(colors.hover_overlay)
                    .border_color(priority_color.opacity(0.8))
                    .shadow_sm()  // 添加轻微阴影
                    .cursor_pointer()
            })
            // 状态指示器：顶部边框（如果有状态）
            .when_some(status_indicator, |this, color| {
                this.border_t_2().border_color(color)
            })
            // 键盘事件处理
            .on_key_down(cx.listener(|this, event, _window, cx| {
                if this.handle_key_event(event, cx) {
                    cx.stop_propagation();
                }
            }))
            .child(
                Collapsible::new()
                    .gap_1()
                    .open(is_open)
                    .child(
                        h_flex()
                            .items_center()
                            .justify_start()
                            .gap(px(8.0))
                            .text_color(text_color)
                            .child(ItemListItem::new(
                                format!("{}-{}", item_id, version),
                                item.clone(),
                                false,
                            ))
                            .child(
                                Button::new("toggle-edit")
                                    .small()
                                    .outline()
                                    .icon(IconName::ChevronDown)
                                    .when(is_open, |this| this.icon(IconName::ChevronUp))
                                    .tooltip(if is_open {
                                        "Close editor (Enter)"
                                    } else {
                                        "Open editor (Enter)"
                                    })
                                    .on_click(move |_event, _window, cx| {
                                        cx.update_entity(&view, |this, cx| {
                                            this.toggle_expand(cx);
                                        })
                                    }),
                            ),
                    )
                    .content(
                        v_flex()
                            .gap(px(8.0))
                            .p(px(8.0))
                            .mt(px(8.0))
                            .bg(cx.theme().background.opacity(0.5))  // 半透明背景
                            .rounded(px(6.0))  // 稍小的圆角
                            .border_1()
                            .border_color(cx.theme().border.opacity(0.5))
                            .child(ItemInfo::new(&item_info))
                    ),
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
