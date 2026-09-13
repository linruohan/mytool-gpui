use gpui::{Context, Entity, Window};
use gpui_component::input::{InputEvent, InputState, TextareaState};
use rust_i18n::t;

use super::{
    super::{
        AttachmentButtonEvent, AttachmentButtonState, PriorityEvent, PriorityState,
        ProjectButtonEvent, ProjectButtonState, RecurrencyButtonEvent, RecurrencyButtonState,
        ReminderButtonEvent, ReminderButtonState, ScheduleButtonEvent, ScheduleButtonState,
        SectionEvent, SectionState,
    },
    ItemInfoEvent, ItemInfoState,
};
use crate::core::{
    notification::{NotificationExt as _, NotificationSystem},
    state::TodoStore,
};

impl ItemInfoState {
    /// 名称输入框事件处理（单行 InputState）
    pub(super) fn on_name_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let text = state.read(cx).value().to_string();
                self.state_manager.set_content(text);
                self.state_manager.mark_dirty();
                if self.state_manager.save_status != super::SaveItemStatus::Idle {
                    self.state_manager.save_status = super::SaveItemStatus::Idle;
                    cx.notify();
                }
            },
            InputEvent::PressEnter { secondary, .. } if !*secondary => {
                self.sync_inputs(cx);
            },
            InputEvent::Blur => {
                self.sync_inputs(cx);
            },
            _ => {},
        };
    }

    /// 描述输入框事件处理（多行 TextareaState，支持 auto_grow）
    pub(super) fn on_desc_input_event(
        &mut self,
        state: &Entity<TextareaState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let text = state.read(cx).value().to_string();
                self.state_manager.set_description(Some(text));
                self.state_manager.mark_dirty();
                if self.state_manager.save_status != super::SaveItemStatus::Idle {
                    self.state_manager.save_status = super::SaveItemStatus::Idle;
                    cx.notify();
                }
            },
            InputEvent::PressEnter { secondary, .. } if !*secondary => {
                // 多行 Textarea 的 Enter 通常是换行，所以这里不再特殊处理
                self.sync_inputs(cx);
            },
            InputEvent::Blur => {
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
                self.set_priority(priority.clone() as i32);
                self.persist_existing_item(cx);
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

                    let filtered = if project_id.is_empty() {
                        None
                    } else {
                        let store = cx.global::<TodoStore>();
                        store
                            .get_project(project_id)
                            .map(|_| store.sections_for_project(project_id))
                    };

                    self.section_state.update(cx, |section_state, cx| {
                        if project_id.is_empty() {
                            section_state.set_sections(None, window, cx);
                        } else if let Some(filtered_sections) = filtered {
                            section_state.set_sections(Some(filtered_sections), window, cx);
                        }
                    });

                    // 当project变更时，重置section_id
                    self.state_manager.set_section_id(None);
                    self.section_state.update(cx, |section_state, cx| {
                        section_state.set_section(None, window, cx);
                    });

                    self.persist_existing_item(cx);
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

                    self.persist_existing_item(cx);
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
            ScheduleButtonEvent::DateSelected(_) | ScheduleButtonEvent::TimeSelected(_) => {
                let schedule_state = _state.read(cx);
                self.state_manager.set_due_date(Some(schedule_state.due_date.clone()));
                self.persist_existing_item(cx);
                cx.emit(ItemInfoEvent::Updated());
            },
            ScheduleButtonEvent::Cleared => {
                self.state_manager.set_due_date(None);
                self.schedule_button_state.update(cx, |state, cx| {
                    state.set_due_date(todos::DueDate::default(), window, cx);
                });
                self.persist_existing_item(cx);
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
                self.state_manager.set_due_date(Some(due_date.clone()));
                self.persist_existing_item(cx);
                cx.emit(ItemInfoEvent::Updated());
            },
            RecurrencyButtonEvent::Cleared => {
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
                self.persist_existing_item(cx);
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
                window.notify_success(t!("todo.reminder.added").to_string(), cx);
            },
            ReminderButtonEvent::Removed(reminder_id) => {
                NotificationSystem::debug(format!("Reminder removed: {:?}", reminder_id));
                window.notify_success(t!("todo.reminder.removed").to_string(), cx);
            },
            ReminderButtonEvent::Error(error) => {
                window.notify_error(
                    t!("todo.reminder.update_failed", error => error.to_string()).to_string(),
                    cx,
                );
            },
        }

        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
    }

    pub(super) fn on_attachment_event(
        &mut self,
        _state: &Entity<AttachmentButtonState>,
        event: &AttachmentButtonEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            AttachmentButtonEvent::Added(_) | AttachmentButtonEvent::Removed(_) => {
                NotificationSystem::debug("attachment list changed");
            },
            AttachmentButtonEvent::Error(error) => {
                window.notify_error(
                    t!("todo.attach.failed", error => error.to_string()).to_string(),
                    cx,
                );
            },
        }
        cx.notify();
    }

    pub(super) fn add_subtask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let parent = self.state_manager.item.clone();
        if parent.id.is_empty() || parent.is_subtask() {
            window.notify_error(t!("todo.item.save_first_subtask").to_string(), cx);
            return;
        }
        let mut item = todos::entity::ItemModel::default();
        item.parent_id = Some(parent.id.clone());
        item.project_id = parent.project_id.clone();
        item.section_id = parent.section_id.clone();
        crate::ui::components::show_new_item_dialog(window, cx, item);
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
