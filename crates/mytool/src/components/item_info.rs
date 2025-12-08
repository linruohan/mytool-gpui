use std::{collections::HashSet, rc::Rc};

use gpui::{
    Action, App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, StyleRefinement,
    Styled, Subscription, Window, div,
};
use gpui_component::{
    IconName, Sizable, Size, StyledExt as _, WindowExt,
    button::{Button, ButtonVariants},
    divider::Divider,
    h_flex,
    input::{Input, InputEvent, InputState},
    list::ListState,
    v_flex,
};
use serde::Deserialize;
use serde_json::Value;
use todos::{
    entity::{ItemModel, LabelModel},
    enums::item_priority::ItemPriority,
};

use super::{PriorityButton, PriorityEvent, PriorityState};
use crate::{
    LabelListDelegate, LabelsPopoverEvent, LabelsPopoverList, service::load_labels,
    todo_state::DBState,
};

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = item_info, no_json)]
struct Info(i32);
const CONTEXT: &'static str = "ItemInfo";
#[derive(Clone)]
pub enum ItemInfoEvent {
    Update(Rc<ItemModel>),
    Add(Rc<ItemModel>),
}
pub struct ItemInfoState {
    focus_handle: FocusHandle,
    pub item: Rc<ItemModel>,
    pub label_list: Entity<ListState<LabelListDelegate>>,
    _subscriptions: Vec<Subscription>,
    // item view
    checked: bool,
    name_input: Entity<InputState>,
    desc_input: Entity<InputState>,
    priority_state: Entity<PriorityState>,
    label_popover_list: Entity<LabelsPopoverList>,
}

impl Focusable for ItemInfoState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<ItemInfoEvent> for ItemInfoState {}
impl ItemInfoState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = Rc::new(ItemModel::default());

        let label_list =
            cx.new(|cx| ListState::new(LabelListDelegate::new(), window, cx).selectable(true));

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("To-do Name"));

        let desc_input = cx.new(|cx| {
            InputState::new(window, cx).auto_grow(5, 20).placeholder("Add a description ...")
        });
        let label_popover_list = cx.new(|cx| LabelsPopoverList::new(window, cx));

        let priority_state = cx.new(|cx| PriorityState::new(window, cx));
        let _subscriptions = vec![
            cx.subscribe_in(&name_input, window, Self::on_input_event),
            cx.subscribe_in(&desc_input, window, Self::on_input_event),
            cx.subscribe_in(&label_popover_list, window, Self::on_labels_event),
            cx.subscribe_in(&priority_state, window, Self::on_priority_event),
        ];

        let label_list_clone = label_list.clone();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |_view, cx| {
            let db = db.lock().await;
            let labels = load_labels(db.clone()).await;
            let rc_labels: Vec<Rc<LabelModel>> =
                labels.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("label_panel: len labels: {}", labels.len());
            let _ = cx
                .update_entity(&label_list_clone, |list, cx| {
                    list.delegate_mut().update_labels(rc_labels);
                    cx.notify();
                })
                .ok();
        })
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            item,
            label_list,
            _subscriptions,
            name_input,
            desc_input,
            checked: false,
            priority_state,
            label_popover_list,
        }
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
            },
            InputEvent::PressEnter { secondary } => {
                let _text = state.read(cx).value().to_string();
                if *secondary {
                    println!("Shift+Enter pressed - insert line break");
                } else {
                    println!("Enter pressed - could submit form");
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
        _cx: &mut Context<Self>,
    ) {
        match event {
            LabelsPopoverEvent::Selected(label) => {
                self.add_checked_labels(label.clone());
            },
            LabelsPopoverEvent::DeSelected(label) => {
                self.rm_checked_labels(label.clone());
            },
        }
    }

    pub fn on_priority_event(
        &mut self,
        _state: &Entity<PriorityState>,
        event: &PriorityEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            PriorityEvent::Selected(priority) => {
                self.set_priority(*priority);
            },
        }
    }

    pub fn add_checked_labels(&mut self, label: Rc<LabelModel>) {
        let item = Rc::make_mut(&mut self.item);
        let mut labels_set = match &item.labels {
            Some(Value::String(current)) => {
                current.split(';').filter(|s: &&str| !s.is_empty()).collect::<HashSet<&str>>()
            },
            _ => HashSet::new(),
        };

        // 添加新标签（HashSet 自动去重）
        labels_set.insert(&label.id);

        // 重新拼接成字符串
        let new_labels = labels_set.into_iter().collect::<Vec<&str>>().join(";");

        item.labels = Some(Value::String(new_labels));
    }

    pub fn rm_checked_labels(&mut self, label: Rc<LabelModel>) {
        let item = Rc::make_mut(&mut self.item);
        if let Some(Value::String(current_labels)) = &item.labels {
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

            item.labels =
                if new_labels.is_empty() { None } else { Some(Value::String(new_labels)) };
        }
    }

    pub fn selected_labels(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<Rc<LabelModel>> {
        let Some(label_ids) = &self.item.labels else {
            return Vec::new();
        };
        let all_labels = self.label_list.read(cx).delegate()._labels.clone();
        label_ids
            .to_string()
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

    fn toggle_finished(&mut self, selectable: &bool, _: &mut Window, _cx: &mut Context<Self>) {
        self.checked = *selectable;
    }

    // set item of item_info
    pub fn set_item(&mut self, item: Rc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) {
        self.item = item.clone();
        self.name_input.update(cx, |this, cx| {
            this.set_value(&item.content.clone(), window, cx);
        });
        self.desc_input.update(cx, |this, cx| {
            this.set_value(&item.description.clone().unwrap_or_default(), window, cx);
        });
        self.priority_state.update(cx, |this, cx| {
            if let Some(priority) = item.priority {
                this.set_priority(ItemPriority::from_i32(priority), window, cx);
            }
        });
        self.label_popover_list.update(cx, |this, cx| {
            if let Some(labels) = item.labels.clone() {
                this.set_item_checked_label_id(labels, window, cx);
            }
        });
    }
}

impl Render for ItemInfoState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        v_flex()
            .border_3()
            .child(Input::new(&self.name_input).focus_bordered(false))
            .child(Input::new(&self.desc_input).bordered(false))
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
                                    .small()
                                    .icon(IconName::MonthSymbolic)
                                    .ghost()
                                    .compact()
                                    .on_click({
                                        // let items_panel = self.items_panel.clone();
                                        move |_event, _window, _cx| {
                                            // let items_panel_clone = items_panel.clone();
                                            // items_panel_clone.update(cx, |items_panel,
                                            // cx| {
                                            //     items_panel.
                                            // show_finish_item_dialog(window,
                                            // cx);
                                            //     cx.notify();
                                            // })
                                        }
                                    }),
                            ),
                        ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .justify_end()
                            // .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                Button::new("item-add")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::PlusLargeSymbolic)
                                    .on_click({
                                        // let items_panel = self.items_panel.clone();
                                        move |_event, _window, _cx| {
                                            // let items_panel_clone = items_panel.clone();
                                            // items_panel_clone.update(cx, |items_panel, cx| {
                                            //     items_panel.show_finish_item_dialog(window, cx);
                                            //     cx.notify();
                                            // })
                                        }
                                    }),
                            )
                            .child(self.label_popover_list.clone()) // tags
                            .child(PriorityButton::new(&self.priority_state))
                            .child(
                                Button::new("item-reminder")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::AlarmSymbolic)
                                    .on_click({
                                        // let items_panel = self.items_panel.clone();
                                        move |_event, _window, _cx| {
                                            // let items_panel_clone = items_panel.clone();
                                            // items_panel_clone.update(cx, |items_panel, cx| {
                                            //     items_panel.show_item_dialog(window, cx, false);
                                            //     cx.notify();
                                            // })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("item-pin")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::PinSymbolic)
                                    .on_click({
                                        // let items_panel = self.items_panel.clone();
                                        move |_event, _window, _cx| {
                                            // let items_panel_clone = items_panel.clone();
                                            // items_panel_clone.update(cx, |items_panel, cx| {
                                            //     items_panel.show_item_dialog(window, cx, true);
                                            //     cx.notify();
                                            // })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("item-more")
                                    .icon(IconName::ViewMoreSymbolic)
                                    .small()
                                    .ghost()
                                    .on_click({
                                        // let items_panel = self.items_panel.clone();
                                        move |_event, _window, _cx| {
                                            // let items_panel_clone = items_panel.clone();
                                            // items_panel_clone.update(cx, |items_panel, cx| {
                                            //     items_panel.show_item_delete_dialog(window, cx);
                                            //     cx.notify();
                                            // })
                                        }
                                    }),
                            ),
                    ),
            )
            .child(Divider::horizontal().p_2())
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
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
                                            move |_event, _window, _cx| {
                                                // let items_panel_clone = items_panel.clone();
                                                // items_panel_clone.update(cx, |items_panel,
                                                // cx| {
                                                //     items_panel.
                                                // show_finish_item_dialog(window,
                                                // cx);
                                                //     cx.notify();
                                                // })
                                            }
                                        }),
                                )
                                .child("——>")
                                .child(
                                    Button::new("item-section")
                                        .label("Section")
                                        .small()
                                        .ghost()
                                        .compact()
                                        .on_click({
                                            // let items_panel = self.items_panel.clone();
                                            move |_event, _window, _cx| {
                                                // let items_panel_clone = items_panel.clone();
                                                // items_panel_clone.update(cx, |items_panel,
                                                // cx| {
                                                //     items_panel.
                                                // show_finish_item_dialog(window,
                                                // cx);
                                                //     cx.notify();
                                                // })
                                            }
                                        }),
                                ),
                        ),
                    )
                    .child(h_flex().gap_2().items_center().justify_end().child(
                        Button::new("save").label("Save").on_click({
                            let view = view.clone();
                            let name_input_clone1 = self.name_input.clone();
                            let des_input_clone1 = self.desc_input.clone();
                            let label_popover_list_clone = self.label_popover_list.clone();
                            move |_, window, cx| {
                                window.close_dialog(cx);
                                view.update(cx, |view, cx| {
                                    let label_ids = label_popover_list_clone
                                        .read(cx)
                                        .selected_labels
                                        .iter()
                                        .map(|label| label.id.clone())
                                        .collect::<Vec<String>>()
                                        .join(";");
                                    println!("label_ids: {}", label_ids);
                                    let item = Rc::new(ItemModel {
                                        content: name_input_clone1.read(cx).value().to_string(),
                                        description: Some(
                                            des_input_clone1.read(cx).value().to_string(),
                                        ),
                                        checked: view.checked,
                                        labels: if label_ids.is_empty() {
                                            None
                                        } else {
                                            Some(label_ids.parse().unwrap_or_default())
                                        },
                                        priority: Some(
                                            view.priority_state.read(cx).priority() as i32
                                        ),
                                        ..Default::default()
                                    });
                                    println!("item_info: before:{:?}", item.clone());
                                    cx.emit(ItemInfoEvent::Update(item.clone()));
                                    cx.notify();
                                });
                            }
                        }),
                    )),
            )
    }
}

/// A DatePicker element.
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
    /// Create a new DatePicker with the given [`ItemInfoState`].
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
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            // .flex_none()
            .w_full()
            // .relative()
            // .input_text_size(self.size)
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
