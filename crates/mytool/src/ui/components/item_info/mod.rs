use std::sync::Arc;

use gpui::{
    App, AppContext, BorrowAppContext, Context, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, RenderOnce, StyleRefinement,
    Styled, Subscription, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    IconName, Sizable, Size, StyledExt as _,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    separator::Separator,
    theme::ActiveTheme,
    v_flex,
};
use todos::{
    entity::ItemModel,
    enums::item_priority::ItemPriority,
};
use tracing::{info, warn};

use super::{
    AttachmentButton, AttachmentButtonState, PriorityButton, PriorityState, ProjectButton,
    ProjectButtonState, RecurrencyButton, RecurrencyButtonState, ReminderButton,
    ReminderButtonState, ScheduleButton, ScheduleButtonState, SectionButton, SectionState,
};
use crate::{
    LabelsPopoverList,
    core::{
        notification::NotificationSystem,
        state::{DBState, TodoStore},
    },
    todo_actions::set_item_pinned_optimistic,
    ui::theme::visual_enhancements::SemanticColors,
};

mod handlers;
mod item_state_manager;
mod labels;
mod save;
mod types;

pub use item_state_manager::{ItemStateManager, SaveItemStatus};
pub use types::ItemInfoEvent;

const CONTEXT: &str = "ItemInfo";

pub struct ItemInfoState {
    focus_handle: FocusHandle,
    /// 集中的状态管理器
    pub state_manager: ItemStateManager,
    _subscriptions: Vec<Subscription>,
    // item view
    name_input: Entity<InputState>,
    desc_input: Entity<InputState>,
    priority_state: Entity<PriorityState>,
    project_state: Entity<ProjectButtonState>,
    section_state: Entity<SectionState>,
    schedule_button_state: Entity<ScheduleButtonState>,
    recurrency_button_state: Entity<RecurrencyButtonState>,
    label_popover_list: Entity<LabelsPopoverList>,
    attachment_state: Entity<AttachmentButtonState>,
    reminder_state: Entity<ReminderButtonState>,
}

impl Focusable for ItemInfoState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<ItemInfoEvent> for ItemInfoState {}
impl ItemInfoState {
    pub fn new(item: Arc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = item.clone();

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Task name..."));

        let desc_input = cx.new(|cx| {
            InputState::new(window, cx).auto_grow(5, 20).placeholder("Add description...")
        });
        let label_popover_list = cx.new(|cx| LabelsPopoverList::new(window, cx));

        let priority_state = cx.new(|cx| PriorityState::new(window, cx));
        let project_state = cx.new(|cx| ProjectButtonState::new(window, cx));
        let section_state = cx.new(|cx| SectionState::new(window, cx));
        let schedule_button_state = cx.new(|cx| {
            let mut state = ScheduleButtonState::new(window, cx);
            // 使用类型安全的 due_date() 方法
            if let Some(due_date) = item.due_date() {
                state.set_due_date(due_date, window, cx);
            }
            state
        });
        let recurrency_button_state = cx.new(|cx| {
            let mut state = RecurrencyButtonState::new(window, cx);
            // 如果有 due_date 且有重复设置，初始化 recurrency_button_state
            if let Some(due_date) = item.due_date() {
                state.set_due_date(due_date, window, cx);
            }
            state
        });
        let attachment_state = cx.new(|cx| AttachmentButtonState::new(item.id.clone(), window, cx));
        let reminder_state = cx.new(|cx| ReminderButtonState::new(item.id.clone(), window, cx));

        let _subscriptions = vec![
            cx.subscribe_in(&name_input, window, Self::on_input_event),
            cx.subscribe_in(&desc_input, window, Self::on_input_event),
            cx.subscribe_in(&label_popover_list, window, Self::on_labels_event),
            cx.subscribe_in(&priority_state, window, Self::on_priority_event),
            cx.subscribe_in(&project_state, window, Self::on_project_event),
            cx.subscribe_in(&section_state, window, Self::on_section_event),
            cx.subscribe_in(&schedule_button_state, window, Self::on_schedule_event),
            cx.subscribe_in(&recurrency_button_state, window, Self::on_recurrency_event),
            cx.subscribe_in(&reminder_state, window, Self::on_reminder_event),
            // 订阅 TodoStore 的变化，确保 pinned 状态和其他状态变化时能够更新界面
            cx.observe_global_in::<TodoStore>(window, move |this, _window, cx| {
                // 🚀 关键修复：检查是否需要跳过更新（避免保存时的死锁）
                if this.state_manager.skip_next_update {
                    info!("ItemInfoState: skipping update due to skip_next_update flag");
                    this.state_manager.skip_next_update = false;
                    return;
                }

                let store = cx.global::<TodoStore>();
                let current_id = &this.state_manager.item.id;

                // 先尝试用当前 ID 查找
                if let Some(updated_item) = store.get_item(current_id) {
                    // 只有当 item 确实发生变化时才更新，避免不必要的渲染
                    if this.state_manager.item != updated_item {
                        // 如果找到且发生变化，更新状态
                        this.state_manager.item = updated_item;
                        // 触发重新渲染
                        cx.notify();
                    }
                } else if current_id.starts_with("temp_") {
                    // 如果当前是临时 ID 且找不到，检查 ID 映射
                    if let Some(real_id) = store.get_real_id(current_id)
                        && let Some(real_item) = store.get_item(real_id)
                    {
                        tracing::info!(
                            "ItemInfoState: detected ID change from {} to {} via mapping",
                            current_id,
                            real_id
                        );

                        // 更新 state_manager 中的 item
                        this.state_manager.item = real_item.clone();

                        // 更新 AttachmentButtonState 的 item_id
                        let new_item_id = real_item.id.clone();
                        this.attachment_state.update(cx, |state, cx| {
                            state.update_item_id(new_item_id.clone(), cx);
                        });

                        // 更新 ReminderButtonState 的 item_id
                        this.reminder_state.update(cx, |state, cx| {
                            state.update_item_id(new_item_id, cx);
                        });

                        // 触发重新渲染
                        cx.notify();
                    }
                }
            }),
        ];
        let mut this = Self {
            focus_handle: cx.focus_handle(),
            state_manager: ItemStateManager::new(item.clone()),
            _subscriptions,
            name_input,
            desc_input,
            priority_state,
            project_state,
            section_state,
            schedule_button_state,
            recurrency_button_state,
            label_popover_list,
            attachment_state,
            reminder_state,
        };
        this.set_item(item, window, cx);
        this
    }

    // set item of item_info
    pub fn set_item(&mut self, item: Arc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) {
        self.set_item_internal(item, window, cx, true);
    }

    /// 更新 item 但不重新加载标签（用于避免覆盖用户的标签更改）
    pub fn update_item_without_reloading_labels(
        &mut self,
        item: Arc<ItemModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_item_internal(item, window, cx, false);
    }

    /// 内部方法：设置 item，可选择是否重新加载标签
    fn set_item_internal(
        &mut self,
        item: Arc<ItemModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
        reload_labels: bool,
    ) {
        // 更新 state_manager
        self.state_manager = ItemStateManager::new(item.clone());

        self.name_input.update(cx, |this, cx| {
            this.set_value(item.content.clone(), window, cx);
        });
        self.desc_input.update(cx, |this, cx| {
            this.set_value(item.description.clone().unwrap_or_default(), window, cx);
        });
        self.priority_state.update(cx, |this, cx| {
            if let Some(priority) = item.priority {
                this.set_priority(ItemPriority::from_i32(priority), window, cx);
            }
        });

        // 🚀 性能优化：一次性获取所有需要的数据，克隆后立即释放借用
        let (projects, all_sections) = {
            let todo_store = cx.global::<TodoStore>();
            (todo_store.projects.clone(), todo_store.sections.clone())
        };

        self.project_state.update(cx, |this, cx| {
            if let Some(project_id) = &item.project_id
                && let Some(project) = projects.iter().find(|p| &p.id == project_id)
            {
                this.set_project(Some(project.id.clone()), window, cx);
            }
        });

        // 根据project_id更新section_state的sections
        let item_section_id = item.section_id.clone();
        self.section_state.update(cx, |section_state, cx| {
            if let Some(project_id) = &item.project_id {
                // 根据project_id获取对应的sections
                if let Some(project) = projects.iter().find(|p| &p.id == project_id) {
                    // 获取该project的sections
                    let filtered_sections: Vec<Arc<todos::entity::SectionModel>> = all_sections
                        .iter()
                        .filter(|s| s.project_id.as_ref() == Some(&project.id))
                        .cloned()
                        .collect();

                    // 确保section_id属于当前project，在移动之前检查
                    if let Some(section_id) = &item_section_id
                        && !filtered_sections.iter().any(|s| &s.id == section_id)
                    {
                        // 使用 state_manager 更新 section_id
                        self.state_manager.set_section_id(None);
                    }

                    section_state.set_sections(Some(filtered_sections), window, cx);
                }
            } else {
                // 如果是Inbox，使用全局的SectionState
                section_state.set_sections(None, window, cx);
            }

            // 设置section
            if let Some(section_id) = &item_section_id {
                // 🚀 性能优化：使用已有的 sections 引用，避免再次访问全局状态
                let sections = if let Some(sections) = &section_state.sections {
                    sections
                } else {
                    &all_sections
                };
                if let Some(section) = sections.iter().find(|s| &s.id == section_id) {
                    section_state.set_section(Some(section.id.clone()), window, cx);
                }
            } else {
                section_state.set_section(None, window, cx);
            }
        });

        // Labels 现在存储在 item_labels 关联表中，需要异步加载
        // 只有在 reload_labels 为 true 时才重新加载标签
        if reload_labels {
            // 异步加载当前项目的标签
            let item_id_for_labels = item.id.clone();
            let label_popover_list = self.label_popover_list.clone();
            let db_state = cx.global::<crate::todo_state::DBState>().clone();
            let this_entity = cx.entity();

            cx.spawn(async move |_this, cx| {
                // 🚀 6.9修复：防御性检查 Store 是否已初始化
                //
                // 【问题】BoardBase::new() 同步创建 ItemInfoState 时会调用 set_item_internal，
                //   而此时 state_init 中的 Store 还是 None（Store 是异步初始化的）。
                //   直接调用 get_store() 会触发 expect panic → 应用闪退。
                //
                // 【修复】先检查 is_store_ready()，未就绪时安全跳过标签加载。
                //   后续 TodoStore 数据加载完成后，observe_global 回调会再次触发更新。
                if !db_state.is_store_ready() {
                    tracing::debug!(
                        "Store not ready, skipping label load for item: {}",
                        item_id_for_labels
                    );
                    return;
                }

                let store = db_state.get_store_async().await;
                match store.get_labels_by_item(&item_id_for_labels).await {
                    Ok(item_labels) => {
                        let label_ids: Vec<String> =
                            item_labels.iter().map(|l| l.id.clone()).collect();
                        let label_ids_str = label_ids.join(";");

                        cx.update_entity(&label_popover_list, |popover_list, cx| {
                            // 注意：这里不能使用 window 参数，因为它不能跨越异步边界
                            // 我们需要在 set_item_checked_label_id 方法中处理这个问题
                            popover_list.set_item_checked_label_id_async(label_ids_str, cx);
                        });

                        // 触发UI更新，确保标签复选框状态正确显示
                        cx.update_entity(&this_entity, |_item_info_state, cx| {
                            cx.notify();
                        });
                    },
                    Err(e) => {
                        NotificationSystem::log_error("Failed to load item labels", e);
                        // 如果加载失败，清空标签选择
                        cx.update_entity(&label_popover_list, |popover_list, cx| {
                            popover_list.set_item_checked_label_id_async(String::new(), cx);
                        });

                        // 即使失败也要触发UI更新
                        cx.update_entity(&this_entity, |_item_info_state, cx| {
                            cx.notify();
                        });
                    },
                }
            })
            .detach();
        }

        // 使用类型安全的 due_date() 方法
        self.schedule_button_state.update(cx, |this, cx| {
            if let Some(due_date) = item.due_date() {
                this.set_due_date(due_date, window, cx);
                return;
            }
            this.set_due_date(todos::DueDate::default(), window, cx);
        });

        // 更新 recurrency_button_state
        self.recurrency_button_state.update(cx, |this, cx| {
            if let Some(due_date) = item.due_date() {
                this.set_due_date(due_date, window, cx);
            } else {
                this.set_due_date(todos::DueDate::default(), window, cx);
            }
        });

        // 异步加载附件和提醒
        let item_id = item.id.clone();
        let attachment_state = self.attachment_state.clone();
        let reminder_state = self.reminder_state.clone();

        cx.spawn(async move |_this, cx| {
            // 异步获取 Store
            let db_state = cx.update_global::<DBState, _>(|db_state, _| db_state.clone());
            let store = db_state.get_store_async().await;

            // 加载附件
            let attachments =
                crate::state_service::load_attachments_by_item_with_store(&item_id, store.clone())
                    .await;
            let rc_attachments =
                attachments.iter().map(|a| Arc::new(a.clone())).collect::<Vec<_>>();
            cx.update_entity(&attachment_state, |state: &mut AttachmentButtonState, cx| {
                state.set_attachments(rc_attachments, cx);
            });

            // 加载提醒
            let reminders =
                crate::state_service::load_reminders_by_item_with_store(&item_id, store).await;
            let rc_reminders = reminders.iter().map(|r| Arc::new(r.clone())).collect::<Vec<_>>();
            cx.update_entity(&reminder_state, |state: &mut ReminderButtonState, cx| {
                state.set_reminders(rc_reminders, cx);
            });
        })
        .detach();
    }
}

impl Render for ItemInfoState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        // 🚀 7.0修复：检查异步保存结果，实现延迟 mark_clean
        let item_id = self.state_manager.item.id.clone();
        if !item_id.is_empty() {
            // 检查保存结果（包括临时 ID 和真实 ID）
            let save_result =
                cx.update_global::<crate::core::state::SaveResults, _>(|results, _| {
                    // 先检查当前 ID，再检查临时 ID
                    let result = results.take_result(&item_id);
                    if result.is_some() {
                        result
                    } else if item_id.starts_with("temp_") {
                        // 如果是临时 ID，也检查有没有相关的保存结果
                        results.take_result(&item_id)
                    } else {
                        None
                    }
                });

            if let Some(save_success) = save_result {
                if save_success {
                    info!("render: received save success for {}, marking clean", item_id);
                    self.state_manager.mark_clean();
                    self.state_manager.update_original();
                    self.state_manager.save_status = SaveItemStatus::Succeeded;
                } else {
                    warn!("render: received save failure for {}, keeping dirty", item_id);
                    self.state_manager.mark_dirty();
                    self.state_manager.save_status = SaveItemStatus::Failed;
                }
                cx.notify();
            }
        }

        let view = cx.entity();
        // 🚀 性能优化：克隆 labels 后立即释放借用，避免在闭包中持有不可变借用
        let labels = cx.global::<TodoStore>().labels.clone();
        // 🚀 性能优化：在渲染开始时缓存选中的标签，避免在闭包中重复调用
        let selected_labels = self.selected_labels(cx);

        let colors = SemanticColors::from_theme(cx);
        let pinned_color = if self.state_manager.item.pinned {
            colors.status_pinned
        } else {
            cx.theme().muted_foreground
        };

        v_flex()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(6.0))
            .overflow_hidden()  // 确保圆角生效
            .shadow_sm()  // 添加轻微阴影
            // 阻止点击事件冒泡，防止意外收起
            .on_mouse_down(gpui::MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            })
            .child(
                h_flex()
                    .gap_1()
                    .p(px(6.0))
                    .bg(cx.theme().background)
                    .border_b_1()
                    .border_color(cx.theme().border.opacity(0.5))
                    .child(
                        Checkbox::new("item-checked")
                            .checked(self.state_manager.item.checked)
                            .on_click(cx.listener(Self::toggle_finished)),
                    )
                    .child(
                        Input::new(&self.name_input)
                            .focus_bordered(false)
                    )
                    .child(
                        Button::new("item-pin")
                            .small()
                            .ghost()
                            .compact()
                            .icon(IconName::PinSymbolic)
                            .text_color(pinned_color)
                            .tooltip("Pin item")
                            .on_click({
                                let item = self.state_manager.item.clone();
                                move |_event, _window, cx| {
                                    set_item_pinned_optimistic(item.clone(), !item.pinned, cx);
                                }
                            }),
                    )
                    // 🚀 7.0新增：保存状态指示器
                    .when(
                        self.state_manager.save_status != SaveItemStatus::Idle,
                        |this| {
                            let status = self.state_manager.save_status;
                            let (icon, color, tooltip) = match status {
                                SaveItemStatus::Saving => (
                                    IconName::ClockSymbolic,
                                    cx.theme().warning,
                                    "Saving...",
                                ),
                                SaveItemStatus::Succeeded => (
                                    IconName::CheckmarkSmallSymbolic,
                                    gpui::green().opacity(0.8),
                                    "Saved successfully",
                                ),
                                SaveItemStatus::Failed => (
                                    IconName::Info,
                                    gpui::red().opacity(0.8),
                                    "Save failed - please retry",
                                ),
                                _ => (IconName::Info, cx.theme().muted_foreground, ""),
                            };
                            this.child(
                                Button::new("save-status")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(icon)
                                    .text_color(color)
                                    .tooltip(tooltip),
                            )
                        },
                    ),
            )
            .child(
                Input::new(&self.desc_input)
                    .bordered(false)
                    .px(px(6.0))
                    .py(px(4.0))
                    .bg(cx.theme().background.opacity(0.5))
            )
            .child(
                h_flex()
                    .gap_2()
                    .p(px(6.0))
                    .flex_wrap()
                    .children(labels.iter().map(|label| {
                        let label_clone = label.clone();
                        let view_clone = view.clone();
                        let is_checked = selected_labels.iter().any(|l| l.id == label.id);
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .p_1()
                            .rounded(px(4.0))
                            .hover(|style| style.bg(cx.theme().accent.opacity(0.1)))
                            .child(
                                Checkbox::new(format!("label-checkbox-{}", label.id))
                                    .checked(is_checked)
                                    .on_click(cx.listener(move |_this, _event, window, cx| {
                                        info!("Label checkbox clicked! Label: {}", label_clone.name);
                                        let label_model = label_clone.as_ref().clone();
                                        cx.update_entity(&view_clone, |view, cx| {
                                            let new_checked = !view.selected_labels(cx).iter().any(|l| l.id == label_clone.id);
                                            view.label_toggle_checked(Arc::new(label_model), &new_checked, window, cx);
                                        });
                                    }))
                            )
                            .child(label.name.clone())
                    }))
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_1()
                    .p(px(6.0))
                    .bg(cx.theme().background.opacity(0.3))
                    .border_t_1()
                    .border_color(cx.theme().border.opacity(0.5))
                    .child(
                        h_flex().gap_1().child(
                            h_flex()
                                .gap_1()
                                .overflow_x_hidden()
                                .flex_nowrap()
                                .child(ScheduleButton::new(&self.schedule_button_state))
                                .child(RecurrencyButton::new(&self.recurrency_button_state)),
                        ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .justify_end()
                            .child(AttachmentButton::new(&self.attachment_state))
                            .child(self.label_popover_list.clone()) // tags
                            .child(PriorityButton::new(&self.priority_state)) // priority
                            .child(ReminderButton::new(&self.reminder_state))
                            .child(
                                Button::new("item-due")
                                    .small()
                                    .ghost()
                                    .tooltip("Set due date")
                                    .compact()
                                    .icon(IconName::DelayLongSmallSymbolic)
                                    .on_click(move |_event, _window, _cx| {}),
                            )
                            .child(
                                Button::new("item-more")
                                    .icon(IconName::ViewMoreSymbolic)
                                    .small()
                                    .ghost()
                                    .tooltip("more actions")
                                    .on_click(move |_event, _window, _cx| {}),
                            ),
                ),
            )
            .child(Separator::horizontal().p_1())
            .child(
                h_flex().items_center().justify_between().gap_1().child(
                    h_flex().gap_1().child(
                        h_flex()
                            .gap_1()
                            .overflow_x_hidden()
                            .flex_nowrap()
                            .child(ProjectButton::new(&self.project_state))
                            .child("——>")
                            .child(SectionButton::new(&self.section_state)),
                    ),
                ),
            )
    }
}

#[derive(IntoElement)]
pub struct ItemInfo {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<ItemInfoState>,
}

impl Sizable for ItemInfo {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Focusable for ItemInfo {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for ItemInfo {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ItemInfo {
    pub fn new(state: &Entity<ItemInfoState>) -> Self {
        Self {
            id: ("item-info", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for ItemInfo {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            // 移除 track_focus，让子组件（输入框）自己管理焦点
            .w_full()
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
