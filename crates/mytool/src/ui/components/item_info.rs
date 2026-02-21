use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    Action, App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, StyleRefinement,
    Styled, Subscription, Window, div, px,
};
use gpui_component::{
    IconName, Sizable, Size, StyledExt as _,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    divider::Divider,
    h_flex,
    input::{Input, InputEvent, InputState},
    theme::ActiveTheme,
    v_flex,
};
use serde::Deserialize;
use todos::{
    entity::{ItemModel, LabelModel},
    enums::item_priority::ItemPriority,
};

use super::{
    AttachmentButton, AttachmentButtonState, PriorityButton, PriorityEvent, PriorityState,
    ProjectButton, ProjectButtonEvent, ProjectButtonState, ReminderButton, ReminderButtonEvent,
    ReminderButtonState, ScheduleButton, ScheduleButtonEvent, ScheduleButtonState, SectionButton,
    SectionEvent, SectionState,
};
use crate::{
    LabelsPopoverEvent, LabelsPopoverList,
    core::{
        notification::{NotificationExt, NotificationSystem},
        state::{TodoStore, get_db_connection},
    },
    todo_actions::{
        // 🚀 使用乐观更新（性能优化）
        add_item_optimistic,
        complete_item_optimistic,
        delete_item_optimistic,
        set_item_pinned_optimistic,
        update_item_optimistic,
    },
    ui::theme::visual_enhancements::SemanticColors,
};

/// 集中的状态管理结构
/// 用于统一管理 item 的状态更新，减少手动同步
pub struct ItemStateManager {
    /// 任务模型
    pub item: Arc<ItemModel>,
    /// 避免重复更新的标志
    pub skip_next_update: bool,
    /// 上次更新时间
    last_update_time: Option<Instant>,
    /// 更新间隔（毫秒）
    update_interval: Duration,
}

// 注意：此 debounce 函数已定义但未使用
// 考虑移除或在需要时使用它来优化频繁的用户输入事件

impl ItemStateManager {
    /// 创建新的 ItemStateManager
    pub fn new(item: Arc<ItemModel>) -> Self {
        Self {
            item,
            skip_next_update: false,
            last_update_time: None,
            update_interval: Duration::from_millis(500), // 500ms 更新间隔
        }
    }

    /// 统一的状态更新方法
    /// 使用闭包来修改 item 数据
    ///
    /// 性能注意：每次调用都会克隆整个 ItemModel
    /// 考虑批量更新以减少克隆次数
    pub fn update_item<F>(&mut self, f: F)
    where
        F: Fn(&mut ItemModel),
    {
        let mut item_data = (*self.item).clone();
        f(&mut item_data);
        self.item = Arc::new(item_data);
    }

    /// 批量更新多个字段，减少克隆次数
    pub fn batch_update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut ItemModel),
    {
        let mut item_data = (*self.item).clone();
        f(&mut item_data);
        self.item = Arc::new(item_data);
    }

    /// 设置项目 ID
    pub fn set_project_id(&mut self, project_id: Option<String>) {
        self.update_item(|item| {
            item.project_id = project_id.clone();
        });
    }

    /// 设置分区 ID
    pub fn set_section_id(&mut self, section_id: Option<String>) {
        self.update_item(|item| {
            item.section_id = section_id.clone();
        });
    }

    /// 设置优先级
    pub fn set_priority(&mut self, priority: i32) {
        self.update_item(|item| {
            item.priority = Some(priority);
        });
    }

    /// 设置截止日期
    pub fn set_due_date(&mut self, due_date: Option<todos::DueDate>) {
        self.update_item(|item| {
            item.due = due_date.clone().map(|d| serde_json::to_value(d).unwrap_or_default());
        });
    }

    /// 设置内容
    pub fn set_content(&mut self, content: String) {
        self.update_item(|item| {
            item.content = content.clone();
        });
    }

    /// 设置描述
    pub fn set_description(&mut self, description: Option<String>) {
        self.update_item(|item| {
            item.description = description.clone();
        });
    }

    /// 设置完成状态
    pub fn set_completed(&mut self, completed: bool) {
        self.update_item(|item| {
            item.checked = completed;
            item.completed_at = if completed { Some(chrono::Utc::now().naive_utc()) } else { None };
        });
    }

    /// 设置置顶状态
    pub fn set_pinned(&mut self, pinned: bool) {
        self.update_item(|item| {
            item.pinned = pinned;
        });
    }

    /// 检查是否可以进行更新
    /// 基于上次更新时间和更新间隔
    pub fn can_update(&mut self) -> bool {
        let now = Instant::now();
        if let Some(last_time) = self.last_update_time
            && now.duration_since(last_time) < self.update_interval
        {
            return false;
        }
        self.last_update_time = Some(now);
        true
    }
}

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = item_info, no_json)]
struct Info(i32);
const CONTEXT: &str = "ItemInfo";
#[derive(Clone)]
pub enum ItemInfoEvent {
    Updated(),    // 更新任务
    Added(),      // 新增任务
    Finished(),   // 状态改为完成
    UnFinished(), // 状态改为未完成
    Deleted(),    // 删除任务
}
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
            cx.subscribe_in(&reminder_state, window, Self::on_reminder_event),
            // 订阅 TodoStore 的变化，确保 pinned 状态和其他状态变化时能够更新界面
            cx.observe_global_in::<TodoStore>(window, move |this, _window, cx| {
                let store = cx.global::<TodoStore>();
                // 查找当前 item 是否在 store 中
                if let Some(updated_item) = store.get_item(&this.state_manager.item.id) {
                    // 如果找到，更新状态
                    this.state_manager.item = updated_item;
                    // 触发重新渲染
                    cx.notify();
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
            label_popover_list,
            attachment_state,
            reminder_state,
        };
        this.set_item(item, window, cx);
        this
    }

    /// 检查是否有任何子组件具有焦点
    pub fn has_focus_within(&self, window: &Window, cx: &App) -> bool {
        // 检查主焦点句柄
        if self.focus_handle.is_focused(window) {
            return true;
        }

        // 检查输入框焦点
        if self.name_input.focus_handle(cx).is_focused(window)
            || self.desc_input.focus_handle(cx).is_focused(window)
        {
            return true;
        }

        // 检查其他子组件焦点
        if self.priority_state.focus_handle(cx).is_focused(window)
            || self.project_state.focus_handle(cx).is_focused(window)
            || self.section_state.focus_handle(cx).is_focused(window)
            || self.schedule_button_state.focus_handle(cx).is_focused(window)
            || self.label_popover_list.focus_handle(cx).is_focused(window)
            || self.attachment_state.focus_handle(cx).is_focused(window)
            || self.reminder_state.focus_handle(cx).is_focused(window)
        {
            return true;
        }

        false
    }

    /// 当失去焦点时调用，用于通知父组件
    pub fn on_focus_lost(&mut self, cx: &mut Context<Self>) {
        // 保存所有修改
        self.save_all_changes(cx);
        // 可以发送一个自定义事件通知父组件
        cx.emit(ItemInfoEvent::Updated());
    }

    fn on_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let text = state.read(cx).value().to_string();
                if state == &self.name_input {
                    self.state_manager.set_content(text);
                } else {
                    self.state_manager.set_description(Some(text));
                }
                // 只更新 UI，不触发数据库保存
                cx.notify();
            },
            InputEvent::PressEnter { secondary } => {
                if !*secondary {
                    // Enter 键时保存（仅在变更时）
                    if self.sync_inputs(cx) {
                        cx.emit(ItemInfoEvent::Updated());
                    }
                }
            },
            InputEvent::Blur => {
                // 失焦时自动保存
                self.save_all_changes(cx);
            },
            _ => {},
        };
    }

    pub fn sync_inputs(&mut self, cx: &mut Context<Self>) -> bool {
        let name = self.name_input.read(cx).value().to_string();
        let desc = self.desc_input.read(cx).value().to_string();
        let new_desc = if desc.is_empty() { None } else { Some(desc) };

        let current_item = &self.state_manager.item;
        let changed = current_item.content != name || current_item.description != new_desc;
        if changed {
            self.state_manager.set_content(name);
            self.state_manager.set_description(new_desc);
        }
        changed
    }

    /// 保存所有修改到数据库
    pub fn save_all_changes(&mut self, cx: &mut Context<Self>) {
        // 同步输入框内容
        let has_input_changes = self.sync_inputs(cx);

        // 触发更新事件
        if has_input_changes {
            cx.emit(ItemInfoEvent::Updated());
        }
    }

    pub fn on_labels_event(
        &mut self,
        _state: &Entity<LabelsPopoverList>,
        event: &LabelsPopoverEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            LabelsPopoverEvent::Selected(label) => {
                let label_model = (**label).clone();
                self.add_checked_labels(Arc::new(label_model), window, cx);
                // 不立即同步，避免关闭 popover
            },
            LabelsPopoverEvent::DeSelected(label) => {
                let label_model = (**label).clone();
                self.rm_checked_labels(Arc::new(label_model), window, cx);
                // 不立即同步，避免关闭 popover
            },
            LabelsPopoverEvent::LabelsChanged(label_ids) => {
                let item_id = self.state_manager.item.id.clone();
                let db = get_db_connection(cx);
                let label_ids_clone = label_ids.clone();

                cx.spawn(async move |_this, _cx| {
                    let label_ids_vec: Vec<String> = label_ids_clone
                        .split(';')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect();

                    let store = todos::Store::new((*db).clone());
                    if let Err(e) = store.set_item_labels(&item_id, &label_ids_vec).await {
                        NotificationSystem::log_error("Failed to set item labels", e);
                    }
                })
                .detach();

                // 只在 LabelsChanged 时发出更新事件，这通常在 popover 关闭时发生
                cx.emit(ItemInfoEvent::Updated());
            },
        }
        // 只在必要时通知 UI 更新，避免过度刷新导致 popover 关闭
        if matches!(event, LabelsPopoverEvent::LabelsChanged(_)) {
            cx.notify();
        }
    }

    /// 同步标签选择状态 - 仅在需要时调用，避免过度刷新
    fn sync_labels_selection(&mut self, cx: &mut Context<Self>) {
        // 从当前选中的标签生成 label_ids 字符串
        let selected_label_ids = self.label_popover_list.read(cx).get_selected_label_ids();

        // 只在有实际变化时触发事件
        if !selected_label_ids.is_empty() {
            // 简单地发送更新事件，但不立即通知以避免关闭 popover
            cx.emit(ItemInfoEvent::Updated());
        }
    }

    /// 让名称输入框获得焦点
    pub fn focus_name_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.name_input.update(cx, |input_state, cx| {
            input_state.focus(window, cx);
        });
    }

    pub fn on_priority_event(
        &mut self,
        _state: &Entity<PriorityState>,
        event: &PriorityEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            PriorityEvent::Selected(priority) => {
                self.set_priority(priority.clone() as i32);
                // 🚀 使用乐观更新（立即更新 UI）
                update_item_optimistic(self.state_manager.item.clone(), cx);
                // 设置标志以避免在 handle_item_info_event 中重复更新
                self.state_manager.skip_next_update = true;
            },
        }
        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
    }

    pub fn on_project_event(
        &mut self,
        _state: &Entity<ProjectButtonState>,
        event: &ProjectButtonEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            ProjectButtonEvent::Selected(project_id) => {
                let item = self.state_manager.item.clone();
                let old_project_id = item.project_id.clone();
                let new_project_id =
                    if project_id.is_empty() { None } else { Some(project_id.clone()) };

                // 只有当project_id实际变化时才更新sections
                if old_project_id != new_project_id {
                    // 使用 state_manager 更新 project_id
                    self.state_manager.set_project_id(new_project_id.clone());

                    // 🚀 性能优化：一次性获取所有需要的数据，克隆后立即释放借用
                    let (projects, all_sections) = {
                        let todo_store = cx.global::<TodoStore>();
                        (todo_store.projects.clone(), todo_store.sections.clone())
                    };

                    // 根据project_id更新section_state的sections
                    self.section_state.update(cx, |section_state, cx| {
                        if project_id.is_empty() {
                            // 如果是Inbox，使用全局的SectionState
                            section_state.set_sections(None, window, cx);
                        } else {
                            // 根据project_id获取对应的sections
                            if let Some(project) = projects.iter().find(|p| &p.id == project_id) {
                                // 获取该project的sections
                                let filtered_sections: Vec<Arc<todos::entity::SectionModel>> =
                                    all_sections
                                        .iter()
                                        .filter(|s| s.project_id.as_ref() == Some(&project.id))
                                        .cloned()
                                        .collect();
                                section_state.set_sections(Some(filtered_sections), window, cx);
                            }
                        }
                    });

                    // 当project变更时，重置section_id
                    self.state_manager.set_section_id(None);
                    self.section_state.update(cx, |section_state, cx| {
                        section_state.set_section(None, window, cx);
                    });

                    // 🚀 使用乐观更新（立即更新 UI）
                    update_item_optimistic(self.state_manager.item.clone(), cx);
                    // 设置标志以避免在 handle_item_info_event 中重复更新
                    self.state_manager.skip_next_update = true;
                }
            },
        }
        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
    }

    pub fn on_section_event(
        &mut self,
        _state: &Entity<SectionState>,
        event: &SectionEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            SectionEvent::Selected(section_id) => {
                let current_item = &self.state_manager.item;
                let new_section_id =
                    if section_id.is_empty() { None } else { Some(section_id.clone()) };

                // 只有当section_id实际变化时才更新
                if current_item.section_id != new_section_id {
                    self.state_manager.set_section_id(new_section_id);
                    // 🚀 使用乐观更新（立即更新 UI）
                    update_item_optimistic(self.state_manager.item.clone(), cx);
                    // 设置标志以避免在 handle_item_info_event 中重复更新
                    self.state_manager.skip_next_update = true;
                    // 立即通知UI更新
                    cx.notify();
                }
                cx.emit(ItemInfoEvent::Updated());
            },
        }
        cx.notify();
    }

    pub fn on_schedule_event(
        &mut self,
        _state: &Entity<ScheduleButtonState>,
        event: &ScheduleButtonEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            ScheduleButtonEvent::DateSelected(_date_str) => {
                let schedule_state = _state.read(cx);
                // 使用 state_manager 更新 due date
                self.state_manager.set_due_date(Some(schedule_state.due_date.clone()));
                // 🚀 使用乐观更新（立即更新 UI）
                update_item_optimistic(self.state_manager.item.clone(), cx);
                // 设置标志以避免在 handle_item_info_event 中重复更新
                self.state_manager.skip_next_update = true;
                cx.emit(ItemInfoEvent::Updated());
            },
            ScheduleButtonEvent::TimeSelected(_time_str) => {
                let schedule_state = _state.read(cx);
                // 使用 state_manager 更新 due date
                self.state_manager.set_due_date(Some(schedule_state.due_date.clone()));
                // 🚀 使用乐观更新
                update_item_optimistic(self.state_manager.item.clone(), cx);
                // 设置标志以避免在 handle_item_info_event 中重复更新
                self.state_manager.skip_next_update = true;
                cx.emit(ItemInfoEvent::Updated());
            },
            ScheduleButtonEvent::RecurrencySelected(_recurrency_type) => {
                let schedule_state = _state.read(cx);
                // 使用 state_manager 更新 due date
                self.state_manager.set_due_date(Some(schedule_state.due_date.clone()));
                // 🚀 使用乐观更新
                update_item_optimistic(self.state_manager.item.clone(), cx);
                // 设置标志以避免在 handle_item_info_event 中重复更新
                self.state_manager.skip_next_update = true;
                cx.emit(ItemInfoEvent::Updated());
            },
            ScheduleButtonEvent::Cleared => {
                // 使用 state_manager 清除 due date
                self.state_manager.set_due_date(None);
                // 同步更新 schedule button 状态
                self.schedule_button_state.update(cx, |state, cx| {
                    state.set_due_date(todos::DueDate::default(), window, cx);
                });
                // 🚀 使用乐观更新
                update_item_optimistic(self.state_manager.item.clone(), cx);
                // 设置标志以避免在 handle_item_info_event 中重复更新
                self.state_manager.skip_next_update = true;
                cx.emit(ItemInfoEvent::Updated());
            },
        }

        // 强制通知 UI 更新，确保按钮显示最新状态
        cx.notify();
    }

    pub fn on_reminder_event(
        &mut self,
        _state: &Entity<ReminderButtonState>,
        event: &ReminderButtonEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            ReminderButtonEvent::Added(reminder) => {
                NotificationSystem::debug(format!("Reminder added: {:?}", reminder.id));
                window.notify_success("Reminder added successfully", cx);
            },
            ReminderButtonEvent::Removed(reminder_id) => {
                NotificationSystem::debug(format!("Reminder removed: {:?}", reminder_id));
                window.notify_success("Reminder removed", cx);
            },
            ReminderButtonEvent::Error(error) => {
                window.notify_error(format!("Failed to manage reminder: {}", error), cx);
            },
        }

        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
    }

    pub fn handle_item_info_event(&mut self, event: &ItemInfoEvent, cx: &mut Context<Self>) {
        match event {
            ItemInfoEvent::Finished() => {
                // 🚀 使用乐观更新（立即完成任务）
                complete_item_optimistic(self.state_manager.item.clone(), true, cx);
            },
            ItemInfoEvent::Added() => {
                // 🚀 使用乐观更新（立即添加任务）
                add_item_optimistic(self.state_manager.item.clone(), cx);
            },
            ItemInfoEvent::Updated() => {
                // 检查是否需要跳过此次更新（避免重复调用）
                if !self.state_manager.skip_next_update && self.state_manager.can_update() {
                    // 🚀 使用乐观更新（立即更新任务）
                    update_item_optimistic(self.state_manager.item.clone(), cx);
                }
                // 重置标志
                self.state_manager.skip_next_update = false;
            },
            ItemInfoEvent::Deleted() => {
                // 🚀 使用乐观更新（立即删除任务）
                delete_item_optimistic(self.state_manager.item.clone(), cx);
            },
            ItemInfoEvent::UnFinished() => {
                // 🚀 使用乐观更新（立即取消完成）
                complete_item_optimistic(self.state_manager.item.clone(), false, cx);
            },
        }
        cx.notify();
    }

    pub fn add_checked_labels(
        &mut self,
        label: Arc<LabelModel>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let item_id = self.state_manager.item.id.clone();
        let label_name = label.name.clone();
        let db = get_db_connection(cx);

        cx.spawn(async move |_this, _cx| {
            let store = todos::Store::new((*db).clone());
            match store.add_label_to_item(&item_id, &label_name).await {
                Ok(_) => {
                    NotificationSystem::debug(format!("Label '{}' added to item", label_name));
                },
                Err(e) => {
                    NotificationSystem::log_error("Failed to add label to item", e);
                },
            }
        })
        .detach();
    }

    pub fn rm_checked_labels(
        &mut self,
        label: Arc<LabelModel>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let item_id = self.state_manager.item.id.clone();
        let label_id = label.id.clone();
        let db = get_db_connection(cx);

        cx.spawn(async move |_this, _cx| {
            let store = todos::Store::new((*db).clone());
            match store.remove_label_from_item(&item_id, &label_id).await {
                Ok(_) => {
                    NotificationSystem::debug("Label removed from item");
                },
                Err(e) => {
                    NotificationSystem::log_error("Failed to remove label from item", e);
                },
            }
        })
        .detach();
    }

    /// 获取选中的 Labels
    ///
    /// 注意：由于 Labels 现在存储在关联表中，此方法返回的是本地缓存的 labels
    /// 如果需要最新的 labels，请使用异步方法从数据库加载
    pub fn selected_labels(&self, cx: &mut Context<Self>) -> Vec<Arc<LabelModel>> {
        // 从 LabelPopoverList 获取当前选中的 labels
        self.label_popover_list.read(cx).selected_labels.clone()
    }

    pub fn priority(&self) -> Option<ItemPriority> {
        Some(ItemPriority::from_i32(self.state_manager.item.priority.unwrap_or_default()))
    }

    pub fn set_priority(&mut self, priority: i32) {
        self.state_manager.set_priority(priority);
    }

    fn toggle_finished(&mut self, _: &bool, _: &mut Window, cx: &mut Context<Self>) {
        let new_checked = !self.state_manager.item.checked;
        self.state_manager.set_completed(new_checked);
        if new_checked {
            cx.emit(ItemInfoEvent::Finished());
        } else {
            cx.emit(ItemInfoEvent::UnFinished());
        }
        cx.notify();
    }

    // set item of item_info
    pub fn set_item(&mut self, item: Arc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) {
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
        // 注意：这里简化处理，实际项目中可能需要更好的状态管理
        // 暂时清空 labels，等待异步加载完成
        self.label_popover_list.update(cx, |this, cx| {
            this.set_item_checked_label_id(String::new(), window, cx);
        });

        // 使用类型安全的 due_date() 方法
        self.schedule_button_state.update(cx, |this, cx| {
            if let Some(due_date) = item.due_date() {
                this.set_due_date(due_date, window, cx);
                return;
            }
            this.set_due_date(todos::DueDate::default(), window, cx);
        });

        // 异步加载附件和提醒
        let item_id = item.id.clone();
        let attachment_state = self.attachment_state.clone();
        let reminder_state = self.reminder_state.clone();
        let db = get_db_connection(cx);

        cx.spawn(async move |_this, cx| {
            // 加载附件
            let attachments =
                crate::state_service::load_attachments_by_item(&item_id, (*db).clone()).await;
            let rc_attachments =
                attachments.iter().map(|a| Arc::new(a.clone())).collect::<Vec<_>>();
            cx.update_entity(&attachment_state, |state: &mut AttachmentButtonState, cx| {
                state.set_attachments(rc_attachments, cx);
            });

            // 加载提醒
            let reminders =
                crate::state_service::load_reminders_by_item(&item_id, (*db).clone()).await;
            let rc_reminders = reminders.iter().map(|r| Arc::new(r.clone())).collect::<Vec<_>>();
            cx.update_entity(&reminder_state, |state: &mut ReminderButtonState, cx| {
                state.set_reminders(rc_reminders, cx);
            });
        })
        .detach();
    }

    // label_toggle_checked：label选中或取消选中
    fn label_toggle_checked(
        &mut self,
        label: Arc<LabelModel>,
        selected: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if *selected {
            self.add_checked_labels(label.clone(), window, cx);
        } else {
            self.rm_checked_labels(label.clone(), window, cx);
        }
        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
    }
}

impl Render for ItemInfoState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        // 🚀 性能优化：克隆 labels 后立即释放借用，避免在闭包中持有不可变借用
        let labels = cx.global::<TodoStore>().labels.clone();
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
            .rounded(px(8.0))
            .overflow_hidden()  // 确保圆角生效
            .shadow_sm()  // 添加轻微阴影
            // 阻止点击事件冒泡，防止意外收起
            .on_mouse_down(gpui::MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            })
            .child(
                h_flex()
                    .gap_2()
                    .p(px(8.0))
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
                    ),
            )
            .child(
                Input::new(&self.desc_input)
                    .bordered(false)
                    .px(px(8.0))
                    .py(px(6.0))
                    .bg(cx.theme().background.opacity(0.5))
            )
            .child(
                h_flex()
                    .gap_3()
                    .p(px(8.0))
                    .flex_wrap()
                    .children(labels.iter().enumerate().map(|(ix, label)| {
                        let label_clone = label.clone();
                        Checkbox::new(format!("label-{}", ix))
                            .label(label.name.clone())
                            .checked(self.selected_labels(cx).iter().any(|l| l.id == label.id))
                            .on_click(cx.listener(move |view, checked: &bool, window, cx| {
                                // 将 Rc<LabelModel> 转换为 Arc<LabelModel>
                                let label_model = label_clone.as_ref().clone();
                                view.label_toggle_checked(Arc::new(label_model), checked, window, cx);
                            }))
                    }))
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .p(px(8.0))
                    .bg(cx.theme().background.opacity(0.3))
                    .border_t_1()
                    .border_color(cx.theme().border.opacity(0.5))
                    .child(
                        h_flex().gap_2().child(
                            v_flex()
                                .gap_1()
                                .overflow_x_hidden()
                                .flex_nowrap()
                                .child(ScheduleButton::new(&self.schedule_button_state)),
                        ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
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
                                    .on_click({
                                        let _view = view.clone();
                                        move |_event, _window, _cx| {}
                                    }),
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
            .child(Divider::horizontal().p_2())
            .child(
                h_flex().items_center().justify_between().gap_2().child(
                    h_flex().gap_2().child(
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
