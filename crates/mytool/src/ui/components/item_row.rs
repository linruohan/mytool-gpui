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
use todos::{entity::ItemModel, enums::item_priority::ItemPriority};
use tracing::info;

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
                    // 检查 item 是否真的发生了变化
                    if this.item == *updated_item {
                        // item 没有变化，跳过更新
                        return;
                    }

                    // 检查是否是标签更新（通过比较 labels 字段）
                    let is_label_update = this.item.labels != updated_item.labels;

                    this.item = updated_item.clone();
                    this.update_version += 1; // 增加版本号，强制重新渲染

                    // 添加调试日志
                    use tracing::info;
                    info!(
                        "ItemRowState: item updated - id: {}, labels: {:?}, version: {}, \
                         is_label_update: {}",
                        updated_item.id, updated_item.labels, this.update_version, is_label_update
                    );

                    // 更新 item_info 中的状态
                    this.item_info.update(cx, |this_info, cx| {
                        // 无论是否是标签更新，都使用 update_item_without_reloading_labels
                        // 这样可以避免覆盖用户正在进行的编辑
                        this_info.update_item_without_reloading_labels(
                            updated_item.clone(),
                            window,
                            cx,
                        );

                        // 如果是标签更新，强制刷新 LabelsPopoverList 的选中状态
                        if is_label_update {
                            this_info.refresh_labels_selection_from_item(cx);
                        }
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
        info!("🚀 ItemRow::save_all_changes START for item: {}", self.item.id);

        // 调用 ItemInfoState 的 save_all_changes 方法
        // 让 ItemInfoState 处理保存操作，避免重复调用 update_item_optimistic
        self.item_info.update(cx, |state, cx| {
            state.save_all_changes(cx);
        });

        // 获取最新的 item 数据（已包含用户的修改）
        let latest_item = self.item_info.read(cx).state_manager.item.clone();
        info!(
            "📊 Item data after save - id: {}, content: '{}', priority: {:?}, labels: {:?}",
            latest_item.id, latest_item.content, latest_item.priority, latest_item.labels
        );

        // 更新本地 item 引用
        self.item = latest_item;
        self.update_version += 1;
        cx.notify();
        info!("✅ ItemRow::save_all_changes END");
    }

    /// 切换展开/收起状态
    fn toggle_expand(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 如果当前是展开状态，收缩时保存所有修改
        if self.is_open {
            self.save_all_changes(cx);
        }
        self.is_open = !self.is_open;

        // 如果展开，尝试让第一个输入框获得焦点
        if self.is_open {
            self.item_info.update(cx, |state, cx| {
                // 尝试让 name_input 获得焦点
                state.focus_name_input(window, cx);

                // 关键修复：展开时强制刷新标签选中状态
                // 从当前 item 的 labels 字段同步 LabelsPopoverList 的选中状态
                state.refresh_labels_selection_from_item(cx);
            });
        }

        cx.notify();
    }

    /// 展开详情面板
    fn expand(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_open {
            self.is_open = true;
            self.item_info.update(cx, |state, cx| {
                state.focus_name_input(window, cx);
            });
            cx.notify();
        }
    }

    /// 收起详情面板并保存修改
    fn collapse(&mut self, cx: &mut Context<Self>) {
        if self.is_open {
            self.save_all_changes(cx);
            self.is_open = false;
            cx.notify();
        }
    }

    /// 检查点击是否在展开按钮区域
    fn is_toggle_button_click(&self, event: &gpui::MouseDownEvent) -> bool {
        // 这里可以根据实际的按钮位置来判断
        // 暂时简化处理，假设右侧区域是按钮区域
        event.position.x > px(300.0) // 简化的判断逻辑
    }

    /// 处理键盘事件
    fn handle_key_event(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        // 如果是展开状态，让输入框优先处理键盘事件
        if self.is_open {
            // 只处理特定的快捷键，其他键盘事件让输入框处理
            match event.keystroke.key.as_str() {
                "enter" if event.keystroke.modifiers == gpui::Modifiers::default() => {
                    // Enter 键：切换展开/收起（仅在没有修饰键时）
                    self.toggle_expand(window, cx);
                    true
                },
                "escape" => {
                    // Escape 键：收起
                    if self.is_open {
                        self.save_all_changes(cx);
                        self.is_open = false;
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
                _ => false, // 其他键盘事件让输入框处理
            }
        } else {
            // 收起状态下的键盘事件处理
            match event.keystroke.key.as_str() {
                "enter" => {
                    // Enter 键：切换展开/收起
                    self.toggle_expand(window, cx);
                    true
                },
                "space" => {
                    // 空格键：切换完成状态
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
                },
                "e" if event.keystroke.modifiers == gpui::Modifiers::command() => {
                    // Cmd/Ctrl + E：编辑任务（展开）
                    self.is_open = true;
                    cx.notify();
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
}

impl Render for ItemRowState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        // 更新焦点状态
        self.is_focused = self.focus_handle.is_focused(window);

        let text_color =
            if self.is_open { cx.theme().accent_foreground } else { cx.theme().foreground };

        // 从 item_info 中获取最新的 item，确保显示最新的数据
        let item = self.item_info.read(cx).state_manager.item.clone();

        // 添加调试日志，跟踪标签变化

        // info!(
        //     "ItemRow render - item id: {}, labels: {:?}, version: {}",
        //     item.id, item.labels, self.update_version
        // );
        let _version = self.update_version; // 获取当前版本号
        // info!(
        //     "ItemRow render - item id: {}, labels: {:?}, version: {}",
        //     item.id, item.labels, version
        // );
        let item_info = self.item_info.clone();
        let is_open = self.is_open;
        let is_focused = self.is_focused;
        let item_id = format!("item-{}", item.id);
        let view = cx.entity();
        let version = self.update_version; // 获取当前版本号

        // 获取语义化颜色
        let colors = SemanticColors::from_theme(cx);
        // 获取优先级值 (1=High, 2=Medium, 3=Low, 4=None)
        let priority = item.priority.unwrap_or(4);
        // 使用 ItemPriority::get_color() 获取优先级颜色
        let priority_color = gpui::rgb(ItemPriority::from_i32(priority).get_color());

        // 根据任务状态选择状态颜色（只显示完成状态）
        let status_indicator = if item.checked { Some(colors.status_completed) } else { None };

        // 完成状态的视觉效果
        let completed_opacity = if item.checked { 0.6 } else { 1.0 };

        // 优先级边框宽度
        let left_border_width = match priority {
            1 => px(4.0), // High: 更粗的边框
            2 => px(3.0), // Medium: 中等边框
            3 => px(2.0), // Low: 细边框
            _ => px(1.0), // None: 最细边框
        };

        div()
            .id(item_id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            // 应用视觉层次：圆角和间距
            .rounded(px(8.0))
            .p(px(12.0))
            .my(px(4.0))  // 添加垂直间距
            // 优先级边框 - 左侧边框
            .border_l(left_border_width)
            .border_color(priority_color)
            // 背景色 - 根据优先级添加轻微色调
            .bg(colors.priority_background_tint(priority, cx.theme().background))
            // 完成状态的透明度
            .opacity(completed_opacity)
            // 焦点环效果 - 使用优先级颜色
            .when(is_focused, |this| {
                this.shadow_md()
                    .border_color(priority_color)
                    .border(px(2.0)) // 焦点时加粗边框
            })
            // 悬停效果：提升视觉层次
            .on_mouse_move(cx.listener(|this, _event, _window, cx| {
                this.is_hovered = true;
                cx.notify();
            }))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, event, window, cx| {
                // 检查是否点击的是展开按钮区域
                if this.is_toggle_button_click(event) {
                    // 点击展开按钮，切换状态
                    this.toggle_expand(window, cx);
                } else if !this.is_open {
                    // 点击其他区域且当前未展开，则展开详情
                    this.expand(window, cx);
                }
                // 无论如何都获得焦点
                this.focus_handle.focus(window, cx);
                cx.notify();
            }))
            .hover(|style: gpui::StyleRefinement| {
                style
                    .bg(colors.hover_overlay)
                    .shadow_md()
                    .cursor_pointer()
            })
            // 状态指示器：顶部边框（如果有状态）
            .when_some(status_indicator, |this: gpui::Stateful<gpui::Div>, color| {
                this.border_t_2().border_color(color)
            })
            // 键盘事件处理
            .on_key_down(cx.listener(|this, event, window, cx| {
                if this.handle_key_event(event, window, cx) {
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
                                    .on_click(move |_event, window, cx| {
                                        cx.update_entity(&view, |this, cx| {
                                            this.toggle_expand(window, cx);
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
