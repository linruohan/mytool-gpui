use std::sync::Arc;

use gpui::{Context, Entity, Window};
use gpui_component::input::{InputEvent, InputState};
use tracing::info;

use crate::{
    core::{
        notification::{NotificationExt as _, NotificationSystem},
        state::TodoStore,
    },
    todo_actions::update_item_optimistic,
};

use super::{ItemInfoEvent, ItemInfoState};
use super::super::{
    PriorityEvent, PriorityState, ProjectButtonEvent, ProjectButtonState, RecurrencyButtonEvent,
    RecurrencyButtonState, ReminderButtonEvent, ReminderButtonState, ScheduleButtonEvent,
    ScheduleButtonState, SectionEvent, SectionState,
};

impl ItemInfoState {
    pub(super) fn on_input_event(
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
                // 🚀 关键修复：标记有未保存的修改
                self.state_manager.mark_dirty();
                // 只更新 UI，不触发数据库保存
                cx.notify();
            },
            InputEvent::PressEnter { secondary, .. } if !*secondary => {
                // Enter 键不再自动保存，只同步输入
                self.sync_inputs(cx);
            },
            InputEvent::Blur => {
                // 失焦时不再自动保存，只同步输入
                self.sync_inputs(cx);
            },
            _ => {},
        };
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
                let new_priority = priority.clone() as i32;
                info!("Priority changed to: {}", new_priority);

                self.set_priority(new_priority);

                // 如果是新建任务，只更新 state_manager，不保存到数据库
                if self.state_manager.is_new_item() {
                    info!("New item, skipping update_item_optimistic");
                } else {
                    // 🚀 立即进行乐观更新（更新 UI 和数据库）
                    update_item_optimistic(self.state_manager.item.clone(), cx);
                }
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

                    // 如果是新建任务，只更新 state_manager，不保存到数据库
                    if !self.state_manager.is_new_item() {
                        // 🚀 使用乐观更新（立即更新 UI）
                        update_item_optimistic(self.state_manager.item.clone(), cx);
                        // 设置标志以避免在 handle_item_info_event 中重复更新
                        self.state_manager.skip_next_update = true;
                    }
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

                    // 如果是新建任务，只更新 state_manager，不保存到数据库
                    if !self.state_manager.is_new_item() {
                        // 🚀 使用乐观更新（立即更新 UI）
                        update_item_optimistic(self.state_manager.item.clone(), cx);
                        // 设置标志以避免在 handle_item_info_event 中重复更新
                        self.state_manager.skip_next_update = true;
                    }
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

                // 如果是新建任务，只更新 state_manager，不保存到数据库
                if !self.state_manager.is_new_item() {
                    // 🚀 使用乐观更新（立即更新 UI 和数据库）
                    update_item_optimistic(self.state_manager.item.clone(), cx);
                }
                // 只发射事件通知父组件，不再在 handle_item_info_event 中重复保存
                cx.emit(ItemInfoEvent::Updated());
            },
            ScheduleButtonEvent::TimeSelected(_time_str) => {
                let schedule_state = _state.read(cx);
                // 使用 state_manager 更新 due date
                self.state_manager.set_due_date(Some(schedule_state.due_date.clone()));

                // 如果是新建任务，只更新 state_manager，不保存到数据库
                if !self.state_manager.is_new_item() {
                    // 🚀 使用乐观更新（立即更新 UI 和数据库）
                    update_item_optimistic(self.state_manager.item.clone(), cx);
                }
                // 只发射事件通知父组件
                cx.emit(ItemInfoEvent::Updated());
            },
            ScheduleButtonEvent::Cleared => {
                // 使用 state_manager 清除 due date
                self.state_manager.set_due_date(None);
                // 同步更新 schedule button 状态
                self.schedule_button_state.update(cx, |state, cx| {
                    state.set_due_date(todos::DueDate::default(), window, cx);
                });

                // 如果是新建任务，只更新 state_manager，不保存到数据库
                if !self.state_manager.is_new_item() {
                    // 🚀 使用乐观更新（立即更新 UI 和数据库）
                    update_item_optimistic(self.state_manager.item.clone(), cx);
                }
                // 只发射事件通知父组件
                cx.emit(ItemInfoEvent::Updated());
            },
        }

        // 强制通知 UI 更新，确保按钮显示最新状态
        cx.notify();
    }

    pub fn on_recurrency_event(
        &mut self,
        _state: &Entity<RecurrencyButtonState>,
        event: &RecurrencyButtonEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            RecurrencyButtonEvent::RecurrencyChanged(due_date) => {
                // 使用 state_manager 更新 due date
                self.state_manager.set_due_date(Some(due_date.clone()));

                // 如果是新建任务，只更新 state_manager，不保存到数据库
                if !self.state_manager.is_new_item() {
                    // 🚀 使用乐观更新（立即更新 UI 和数据库）
                    update_item_optimistic(self.state_manager.item.clone(), cx);
                }
                // 只发射事件通知父组件
                cx.emit(ItemInfoEvent::Updated());
            },
            RecurrencyButtonEvent::Cleared => {
                // 清除重复设置，但保留原有的 due_date
                let current_due_date = self.state_manager.item.due_date();
                if let Some(mut due_date) = current_due_date {
                    due_date.recurrency_type = todos::enums::RecurrencyType::NONE;
                    due_date.recurrency_interval = 0;
                    due_date.is_recurring = false;
                    due_date.recurrency_supported = false;
                    due_date.recurrency_end = "".to_string();
                    due_date.recurrency_count = 0;
                    due_date.recurrency_weeks = "".to_string();
                    self.state_manager.set_due_date(Some(due_date));
                }

                // 如果是新建任务，只更新 state_manager，不保存到数据库
                if !self.state_manager.is_new_item() {
                    // 🚀 使用乐观更新（立即更新 UI 和数据库）
                    update_item_optimistic(self.state_manager.item.clone(), cx);
                }
                // 只发射事件通知父组件
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

    pub fn set_priority(&mut self, priority: i32) {
        self.state_manager.set_priority(priority);
    }

    pub(super) fn toggle_finished(&mut self, _: &bool, _: &mut Window, cx: &mut Context<Self>) {
        let new_checked = !self.state_manager.item.checked;
        self.state_manager.set_completed(new_checked);
        if new_checked {
            cx.emit(ItemInfoEvent::Finished());
        } else {
            cx.emit(ItemInfoEvent::UnFinished());
        }
        cx.notify();
    }
}
