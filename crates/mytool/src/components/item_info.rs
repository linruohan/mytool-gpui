use std::{collections::HashSet, sync::Arc};

use gpui::{
    Action, App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, StyleRefinement,
    Styled, Subscription, Window, blue, div, px,
};
use gpui_component::{
    IconName, Sizable, Size, StyledExt as _,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    divider::Divider,
    h_flex,
    input::{Input, InputEvent, InputState},
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
    todo_actions::{add_item, completed_item, delete_item, uncompleted_item, update_item},
    todo_state::{DBState, LabelState},
};

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
    pub item: Arc<ItemModel>,
    _subscriptions: Vec<Subscription>,
    // item view
    #[allow(dead_code)]
    checked: bool,
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

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("To-do Name"));

        let desc_input = cx.new(|cx| {
            InputState::new(window, cx).auto_grow(5, 20).placeholder("Add a description ...")
        });
        let label_popover_list = cx.new(|cx| LabelsPopoverList::new(window, cx));

        let priority_state = cx.new(|cx| PriorityState::new(window, cx));
        let project_state = cx.new(|cx| ProjectButtonState::new(window, cx));
        let section_state = cx.new(|cx| SectionState::new(window, cx));
        let schedule_button_state = cx.new(|cx| {
            let mut state = ScheduleButtonState::new(window, cx);
            if let Some(due_date) = item
                .due
                .as_ref()
                .and_then(|json| serde_json::from_value::<todos::DueDate>(json.clone()).ok())
            {
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
        ];
        let mut this = Self {
            focus_handle: cx.focus_handle(),
            item: item.clone(),
            _subscriptions,
            name_input,
            desc_input,
            checked: false,
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
                let mut item_data = (*self.item).clone();
                if state == &self.name_input {
                    item_data.content = text;
                } else {
                    item_data.description = Some(text);
                }
                self.item = Arc::new(item_data);
                // 只更新 UI，不触发数据库保存
                cx.notify();
            },
            InputEvent::PressEnter { secondary } => {
                let _text = state.read(cx).value().to_string();
                if *secondary {
                } else {
                    // Enter 键时保存（仅在变更时）
                    if self.sync_inputs(cx) {
                        cx.emit(ItemInfoEvent::Updated());
                    }
                }
            },
            _ => {},
        };
    }

    pub fn sync_inputs(&mut self, cx: &mut Context<Self>) -> bool {
        let name = self.name_input.read(cx).value().to_string();
        let desc = self.desc_input.read(cx).value().to_string();
        let new_desc = if desc.is_empty() { None } else { Some(desc) };

        let mut item_data = (*self.item).clone();
        let changed = item_data.content != name || item_data.description != new_desc;
        if changed {
            item_data.content = name;
            item_data.description = new_desc;
            self.item = Arc::new(item_data);
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
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            LabelsPopoverEvent::Selected(label) => {
                let label_model = (**label).clone();
                self.add_checked_labels(Arc::new(label_model));
            },
            LabelsPopoverEvent::DeSelected(label) => {
                let label_model = (**label).clone();
                self.rm_checked_labels(Arc::new(label_model));
            },
        }
        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
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
                self.set_priority(*priority);
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
                let item = self.item.clone();
                let old_project_id = item.project_id.clone();
                let new_project_id =
                    if project_id.is_empty() { None } else { Some(project_id.clone()) };

                // 只有当project_id实际变化时才更新sections
                if old_project_id != new_project_id {
                    // 创建一个新的 ItemModel 实例并修改它
                    let mut item_model = (*item).clone();
                    item_model.project_id = new_project_id.clone();
                    self.item = Arc::new(item_model);

                    // 根据project_id更新section_state的sections
                    self.section_state.update(cx, |section_state, cx| {
                        if project_id.is_empty() {
                            // 如果是Inbox，使用全局的SectionState
                            section_state.set_sections(None, window, cx);
                        } else {
                            // 根据project_id获取对应的sections
                            let projects =
                                cx.global::<crate::todo_state::ProjectState>().projects.clone();
                            let all_sections =
                                cx.global::<crate::todo_state::ProjectState>().sections.clone();

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
                    let mut item_model = (*self.item).clone();
                    item_model.section_id = None;
                    self.item = Arc::new(item_model);
                    self.section_state.update(cx, |section_state, cx| {
                        section_state.set_section(None, window, cx);
                    });
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
                let mut item_data = (*self.item).clone();
                item_data.section_id =
                    if section_id.is_empty() { None } else { Some(section_id.clone()) };
                self.item = Arc::new(item_data);
            },
        }
        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
    }

    pub fn on_schedule_event(
        &mut self,
        _state: &Entity<ScheduleButtonState>,
        event: &ScheduleButtonEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            ScheduleButtonEvent::DateSelected(_date_str) => {
                let schedule_state = _state.read(cx);
                let mut item_data = (*self.item).clone();
                if let Ok(json_value) = serde_json::to_value(&schedule_state.due_date) {
                    item_data.due = Some(json_value);
                }
                self.item = Arc::new(item_data);
            },
            ScheduleButtonEvent::TimeSelected(_time_str) => {
                let schedule_state = _state.read(cx);
                let mut item_data = (*self.item).clone();
                if let Ok(json_value) = serde_json::to_value(&schedule_state.due_date) {
                    item_data.due = Some(json_value);
                }
                self.item = Arc::new(item_data);
            },
            ScheduleButtonEvent::RecurrencySelected(_recurrency_type) => {
                let schedule_state = _state.read(cx);
                let mut item_data = (*self.item).clone();
                if let Ok(json_value) = serde_json::to_value(&schedule_state.due_date) {
                    item_data.due = Some(json_value);
                }
                self.item = Arc::new(item_data);
            },
            ScheduleButtonEvent::Cleared => {
                let mut item_data = (*self.item).clone();
                item_data.due = None;
                self.item = Arc::new(item_data);
            },
            // ScheduleButtonEvent::DueDateChanged => {
            //     let schedule_state = _state.read(cx);
            //     let item = Rc::make_mut(&mut self.item);
            //     if let Ok(json_value) = serde_json::to_value(&schedule_state.due_date) {
            //         item.due = Some(json_value);
            //     }
            // },
            // ScheduleButtonEvent::PickerOpened(_) => {
            //     // 不需要更新 item 数据，只需要处理 UI 状态
            // },
        }
        // println!("schedule changed: {:?}", self.item.due);

        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
    }

    pub fn on_reminder_event(
        &mut self,
        _state: &Entity<ReminderButtonState>,
        event: &ReminderButtonEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            ReminderButtonEvent::Added(reminder) => {
                // 这里可以更新 item 的 reminders 字段
                // 由于提醒已经通过 todo_actions::add_reminder 保存到数据库
                // 这里只需要确保 UI 状态正确即可
                println!("Reminder added: {:?}", reminder.id);
            },
            ReminderButtonEvent::Removed(reminder_id) => {
                // 这里可以更新 item 的 reminders 字段
                // 由于提醒已经通过 todo_actions::delete_reminder 从数据库删除
                // 这里只需要确保 UI 状态正确即可
                println!("Reminder removed: {:?}", reminder_id);
            },
            ReminderButtonEvent::Error(error) => {
                // 处理错误
                println!("Reminder error: {:?}", error);
            },
        }

        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
    }

    pub fn handle_item_info_event(&mut self, event: &ItemInfoEvent, cx: &mut Context<Self>) {
        match event {
            ItemInfoEvent::Finished() => {
                completed_item(self.item.clone(), cx);
            },
            ItemInfoEvent::Added() => {
                add_item(self.item.clone(), cx);
            },
            ItemInfoEvent::Updated() => {
                update_item(self.item.clone(), cx);
            },
            ItemInfoEvent::Deleted() => {
                delete_item(self.item.clone(), cx);
            },
            ItemInfoEvent::UnFinished() => {
                uncompleted_item(self.item.clone(), cx);
            },
        }
        cx.notify();
    }

    pub fn add_checked_labels(&mut self, label: Arc<LabelModel>) {
        // 创建一个新的 ItemModel 实例并修改它
        let mut item_model = (*self.item).clone();
        let mut labels_set = match &item_model.labels {
            Some(current) => {
                current.split(';').filter(|s: &&str| !s.is_empty()).collect::<HashSet<&str>>()
            },
            _ => HashSet::new(),
        };

        // 添加新标签（HashSet 自动去重）
        labels_set.insert(&label.id);

        // 重新拼接成字符串
        let new_labels = labels_set.into_iter().collect::<Vec<&str>>().join(";");

        item_model.labels = Some(new_labels);
        self.item = Arc::new(item_model);
    }

    pub fn rm_checked_labels(&mut self, label: Arc<LabelModel>) {
        // 创建一个新的 ItemModel 实例并修改它
        let mut item_model = (*self.item).clone();
        if let Some(current_labels) = &item_model.labels {
            if current_labels.is_empty() {
                item_model.labels = None;
                self.item = Arc::new(item_model);
                return;
            }

            // 使用正则表达式或更精确的字符串处理
            let new_labels = current_labels
                .split(';')
                .filter(|id: &&str| *id != label.id && !id.is_empty())
                .collect::<Vec<_>>()
                .join(";");

            item_model.labels = if new_labels.is_empty() { None } else { Some(new_labels) };
            self.item = Arc::new(item_model);
        }
    }

    pub fn selected_labels(&self, cx: &mut Context<Self>) -> Vec<Arc<LabelModel>> {
        let Some(label_ids) = &self.item.labels else {
            return Vec::new();
        };
        let all_labels = cx.global::<LabelState>().labels.clone();
        label_ids
            .split(';')
            .filter_map(|label_id| {
                let trimmed_id = label_id.trim();
                if trimmed_id.is_empty() {
                    return None;
                }
                all_labels.iter().find(|label| label.id == trimmed_id).map(|rc_label| {
                    let label = (**rc_label).clone();
                    Arc::new(label)
                })
            })
            .collect()
    }

    pub fn priority(&self) -> Option<ItemPriority> {
        Some(ItemPriority::from_i32(self.item.priority.unwrap_or_default()))
    }

    pub fn set_priority(&mut self, priority: i32) {
        let mut item_data = (*self.item).clone();
        item_data.priority = Some(priority);
        self.item = Arc::new(item_data);
    }

    fn toggle_finished(&mut self, _: &bool, _: &mut Window, cx: &mut Context<Self>) {
        let mut item_data = (*self.item).clone();
        item_data.checked = !item_data.checked;
        self.item = Arc::new(item_data);
        if self.item.checked {
            cx.emit(ItemInfoEvent::Finished());
        } else {
            cx.emit(ItemInfoEvent::UnFinished());
        }
        cx.notify();
    }

    // set item of item_info
    pub fn set_item(&mut self, item: Arc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) {
        self.item = item.clone();
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
        self.project_state.update(cx, |this, cx| {
            if let Some(project_id) = &item.project_id {
                let projects = cx.global::<crate::todo_state::ProjectState>().projects.clone();
                if let Some(project) = projects.iter().find(|p| &p.id == project_id) {
                    this.set_project(Some(project.id.clone()), window, cx);
                }
            }
        });

        // 根据project_id更新section_state的sections
        self.section_state.update(cx, |section_state, cx| {
            if let Some(project_id) = &item.project_id {
                // 根据project_id获取对应的sections
                let projects = cx.global::<crate::todo_state::ProjectState>().projects.clone();
                if let Some(project) = projects.iter().find(|p| &p.id == project_id) {
                    // 获取该project的sections
                    let project_sections =
                        cx.global::<crate::todo_state::ProjectState>().sections.clone();
                    let filtered_sections: Vec<Arc<todos::entity::SectionModel>> = project_sections
                        .iter()
                        .filter(|s| s.project_id.as_ref() == Some(&project.id))
                        .cloned()
                        .collect();

                    // 确保section_id属于当前project，在移动之前检查
                    if let Some(section_id) = &item.section_id
                        && !filtered_sections.iter().any(|s| &s.id == section_id)
                    {
                        // 创建一个新的 ItemModel 实例并修改它
                        let mut item_model = (*self.item).clone();
                        item_model.section_id = None;
                        self.item = Arc::new(item_model);
                    }

                    section_state.set_sections(Some(filtered_sections), window, cx);
                }
            } else {
                // 如果是Inbox，使用全局的SectionState
                section_state.set_sections(None, window, cx);
            }

            // 设置section
            if let Some(section_id) = &item.section_id {
                let sections = if let Some(sections) = &section_state.sections {
                    sections.clone()
                } else {
                    cx.global::<crate::todo_state::SectionState>().sections.clone()
                };
                if let Some(section) = sections.iter().find(|s| &s.id == section_id) {
                    section_state.set_section(Some(section.id.clone()), window, cx);
                }
            } else {
                section_state.set_section(None, window, cx);
            }
        });

        self.label_popover_list.update(cx, |this, cx| {
            if let Some(labels) = item.labels.clone() {
                this.set_item_checked_label_id(labels, window, cx);
            }
        });

        self.schedule_button_state.update(cx, |this, cx| {
            if let Some(due) = item.due.clone()
                && let Ok(due_date) = serde_json::from_value::<todos::DueDate>(due)
            {
                this.set_due_date(due_date, window, cx);
                return;
            }
            this.set_due_date(todos::DueDate::default(), window, cx);
        });

        // 异步加载附件和提醒
        let item_id = item.id.clone();
        let attachment_state = self.attachment_state.clone();
        let reminder_state = self.reminder_state.clone();
        let db = cx.global::<DBState>().conn.clone();

        cx.spawn(async move |_this, cx| {
            // 加载附件
            let attachments = crate::service::load_attachments_by_item(&item_id, db.clone()).await;
            let rc_attachments =
                attachments.iter().map(|a| Arc::new(a.clone())).collect::<Vec<_>>();
            cx.update_entity(&attachment_state, |state: &mut AttachmentButtonState, cx| {
                state.set_attachments(rc_attachments, cx);
            });

            // 加载提醒
            let reminders = crate::service::load_reminders_by_item(&item_id, db.clone()).await;
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
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if *selected {
            self.add_checked_labels(label.clone());
        } else {
            self.rm_checked_labels(label.clone());
        }
        cx.emit(ItemInfoEvent::Updated());
        cx.notify();
    }
}

impl Render for ItemInfoState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        let labels = cx.global::<LabelState>().labels.clone();
        v_flex()
            .border_2()
            .border_color(blue())
            .rounded(px(10.0))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Checkbox::new("item-checked")
                            .checked(self.item.checked)
                            .on_click(cx.listener(Self::toggle_finished)),
                    )
                    .child(Input::new(&self.name_input).focus_bordered(false))
                    .child(
                        Button::new("item-pin")
                            .small()
                            .ghost()
                            .compact()
                            .icon(IconName::PinSymbolic)
                            .tooltip("Pin item")
                            .on_click({
                                let view = view.clone();
                                move |_event, _window, cx| {
                                    cx.update_entity(&view, |this, cx| {
                                        let mut item_model = (*this.item).clone();
                                        item_model.pinned = !item_model.pinned;
                                        this.item = Arc::new(item_model);
                                        cx.emit(ItemInfoEvent::Updated());
                                        cx.notify();
                                    });
                                }
                            }),
                    ),
            )
            .child(Input::new(&self.desc_input).bordered(false))
            .child(h_flex().gap_3().children(labels.iter().enumerate().map(|(ix, label)| {
                let label_clone = label.clone();
                Checkbox::new(format!("label-{}", ix))
                    .label(label.name.clone())
                    .checked(self.selected_labels(cx).iter().any(|l| l.id == label.id))
                    .on_click(cx.listener(move |view, checked: &bool, window, cx| {
                        // 将 Rc<LabelModel> 转换为 Arc<LabelModel>
                        let label_model = label_clone.as_ref().clone();
                        view.label_toggle_checked(Arc::new(label_model), checked, window, cx);
                    }))
            })))
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
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
                                    .on_click({
                                        // let items_panel = self.items_panel.clone();
                                        move |_event, _window, _cx| {}
                                    }),
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
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .w_full()
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
