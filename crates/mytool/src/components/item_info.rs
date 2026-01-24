use std::{collections::HashSet, rc::Rc};

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
    PriorityButton, PriorityEvent, PriorityState, SectionButton, SectionEvent, SectionState,
};
use crate::{
    LabelsPopoverEvent, LabelsPopoverList,
    todo_actions::{add_item, completed_item, delete_item, uncompleted_item, update_item},
    todo_state::LabelState,
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
    pub item: Rc<ItemModel>,
    _subscriptions: Vec<Subscription>,
    // item view
    checked: bool,
    name_input: Entity<InputState>,
    desc_input: Entity<InputState>,
    priority_state: Entity<PriorityState>,
    section_state: Entity<SectionState>,
    label_popover_list: Entity<LabelsPopoverList>,
}

impl Focusable for ItemInfoState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<ItemInfoEvent> for ItemInfoState {}
impl ItemInfoState {
    pub fn new(item: Rc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = item.clone();

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("To-do Name"));

        let desc_input = cx.new(|cx| {
            InputState::new(window, cx).auto_grow(5, 20).placeholder("Add a description ...")
        });
        let label_popover_list = cx.new(|cx| LabelsPopoverList::new(window, cx));

        let priority_state = cx.new(|cx| PriorityState::new(window, cx));
        let section_state = cx.new(|cx| SectionState::new(window, cx));
        let _subscriptions = vec![
            cx.subscribe_in(&name_input, window, Self::on_input_event),
            cx.subscribe_in(&desc_input, window, Self::on_input_event),
            cx.subscribe_in(&label_popover_list, window, Self::on_labels_event),
            cx.subscribe_in(&priority_state, window, Self::on_priority_event),
            cx.subscribe_in(&section_state, window, Self::on_section_event),
        ];
        let mut this = Self {
            focus_handle: cx.focus_handle(),
            item: item.clone(),
            _subscriptions,
            name_input,
            desc_input,
            checked: false,
            priority_state,
            section_state,
            label_popover_list,
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
                let item = Rc::make_mut(&mut self.item);
                if state == &self.name_input {
                    item.content = text;
                } else {
                    item.description = Some(text);
                }
                // 只更新 UI，不触发数据库保存
                cx.notify();
            },
            InputEvent::PressEnter { secondary } => {
                let _text = state.read(cx).value().to_string();
                if *secondary {
                } else {
                    // Enter 键时保存
                    cx.emit(ItemInfoEvent::Updated());
                }
            },
            _ => {},
        };
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
                self.add_checked_labels(label.clone());
            },
            LabelsPopoverEvent::DeSelected(label) => {
                self.rm_checked_labels(label.clone());
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

    pub fn on_section_event(
        &mut self,
        _state: &Entity<SectionState>,
        event: &SectionEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            SectionEvent::Selected(section_id) => {
                let item = Rc::make_mut(&mut self.item);
                item.section_id =
                    if section_id.is_empty() { None } else { Some(section_id.clone()) };
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

    pub fn add_checked_labels(&mut self, label: Rc<LabelModel>) {
        let item = Rc::make_mut(&mut self.item);
        let mut labels_set = match &item.labels {
            Some(current) => {
                current.split(';').filter(|s: &&str| !s.is_empty()).collect::<HashSet<&str>>()
            },
            _ => HashSet::new(),
        };

        // 添加新标签（HashSet 自动去重）
        labels_set.insert(&label.id);

        // 重新拼接成字符串
        let new_labels = labels_set.into_iter().collect::<Vec<&str>>().join(";");

        item.labels = Some(new_labels);
    }

    pub fn rm_checked_labels(&mut self, label: Rc<LabelModel>) {
        let item = Rc::make_mut(&mut self.item);
        if let Some(current_labels) = &item.labels {
            if current_labels.is_empty() {
                item.labels = None;
                return;
            }

            // 使用正则表达式或更精确的字符串处理
            let new_labels = current_labels
                .split(';')
                .filter(|id: &&str| *id != label.id && !id.is_empty())
                .collect::<Vec<_>>()
                .join(";");

            item.labels = if new_labels.is_empty() { None } else { Some(new_labels) };
        }
    }

    pub fn selected_labels(&self, cx: &mut Context<Self>) -> Vec<Rc<LabelModel>> {
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
                all_labels.iter().find(|label| label.id == trimmed_id).map(Rc::clone)
            })
            .collect()
    }

    pub fn priority(&self) -> Option<ItemPriority> {
        Some(ItemPriority::from_i32(self.item.priority.unwrap_or_default()))
    }

    pub fn set_priority(&mut self, priority: i32) {
        let item = Rc::make_mut(&mut self.item);
        item.priority = Some(priority);
    }

    fn toggle_finished(&mut self, _: &bool, _: &mut Window, cx: &mut Context<Self>) {
        let item = Rc::make_mut(&mut self.item);
        item.checked = !item.checked;
        if item.checked {
            cx.emit(ItemInfoEvent::Finished());
        } else {
            cx.emit(ItemInfoEvent::UnFinished());
        }
        cx.notify();
    }

    // set item of item_info
    pub fn set_item(&mut self, item: Rc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) {
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
        self.section_state.update(cx, |this, cx| {
            if let Some(section_id) = &item.section_id {
                let sections = cx.global::<crate::todo_state::SectionState>().sections.clone();
                if let Some(section) = sections.iter().find(|s| &s.id == section_id) {
                    this.set_section(Some(section.clone()), window, cx);
                }
            }
        });
        self.label_popover_list.update(cx, |this, cx| {
            if let Some(labels) = item.labels.clone() {
                this.set_item_checked_label_id(labels, window, cx);
            }
        });
    }

    // label_toggle_checked：label选中或取消选中
    fn label_toggle_checked(
        &mut self,
        label: Rc<LabelModel>,
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
                                        let item = Rc::make_mut(&mut this.item);
                                        item.pinned = !item.pinned;
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
                        view.label_toggle_checked(label_clone.clone(), checked, window, cx);
                    }))
            })))
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        h_flex().gap_2().child(
                            v_flex().gap_1().overflow_x_hidden().flex_nowrap().child(
                                Button::new("item-schedule")
                                    .label("Schedule")
                                    .tooltip("schedule item")
                                    .small()
                                    .icon(IconName::MonthSymbolic)
                                    .ghost()
                                    .compact()
                                    .on_click({
                                        // let items_panel = self.items_panel.clone();
                                        move |_event, _window, _cx| {}
                                    }),
                            ),
                        ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .justify_end()
                            .child(
                                Button::new("item-attachment")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .tooltip("Add attachment")
                                    .icon(IconName::MailAttachmentSymbolic)
                                    .on_click({
                                        // let items_panel = self.items_panel.clone();
                                        move |_event, _window, _cx| {}
                                    }),
                            )
                            .child(self.label_popover_list.clone()) // tags
                            .child(PriorityButton::new(&self.priority_state)) // priority
                            .child(
                                Button::new("item-reminder")
                                    .small()
                                    .tooltip("Set reminder")
                                    .ghost()
                                    .compact()
                                    .icon(IconName::AlarmSymbolic)
                                    .on_click({
                                        // let items_panel = self.items_panel.clone();
                                        move |_event, _window, _cx| {}
                                    }),
                            )
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
                            .child(
                                Button::new("item-project")
                                    .label("Inbox")
                                    .small()
                                    .icon(IconName::Inbox)
                                    .ghost()
                                    .compact()
                                    .on_click({
                                        // let items_panel = self.items_panel.clone();
                                        move |_event, _window, _cx| {}
                                    }),
                            )
                            .child("——>")
                            .child(SectionButton::new(
                                &self.section_state,
                                cx.global::<crate::todo_state::SectionState>().sections.clone(),
                            )),
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
