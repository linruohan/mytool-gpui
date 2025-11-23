use std::rc::Rc;

use gpui::{
    Action, App, AppContext, Context, Corner, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce,
    SharedString, StyleRefinement, Styled, Subscription, Window, div, px,
};
use gpui_component::{
    IconName, Sizable, Size, StyledExt as _, WindowExt,
    button::{Button, ButtonVariants},
    date_picker::DatePickerState,
    divider::Divider,
    h_flex,
    input::{Input, InputState},
    list::ListState,
    menu::DropdownMenu,
    v_flex,
};
use serde::Deserialize;
use todos::{
    entity::{ItemModel, LabelModel},
    enums::item_priority::ItemPriority,
};

use super::{PriorityButton, PriorityEvent, PriorityState};
use crate::{DBState, LabelListDelegate, LabelsPopoverEvent, LabelsPopoverList, load_labels};

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
    date: Entity<DatePickerState>,
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

        let date = cx.new(|cx| DatePickerState::new(window, cx));
        let priority_state = cx.new(|cx| PriorityState::new(window, cx));
        let _subscriptions = vec![
            cx.subscribe(&label_popover_list, |_this, _, ev: &LabelsPopoverEvent, _| match ev {
                LabelsPopoverEvent::Selected(label) => {
                    println!("label_popover_list select: {:?}", label);
                },
            }),
            cx.subscribe_in(
                &priority_state,
                window,
                move |this, _, ev: &PriorityEvent, _window, _cx| match ev {
                    PriorityEvent::Selected(priority) => {
                        this.set_priority(*priority);
                    },
                },
            ),
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
            date,
            priority_state,
            label_popover_list,
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

    /// Set the date of the date picker.
    pub fn set_item(
        &mut self,
        item: Rc<ItemModel>,
        _emit: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.item = item;
    }

    fn toggle_finished(&mut self, selectable: &bool, _: &mut Window, _cx: &mut Context<Self>) {
        self.checked = *selectable;
    }

    // set item of item_info
    fn item(&mut self, item: Rc<ItemModel>, _cx: &mut Context<Self>) {
        self.item = item.clone();
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
                            v_flex()
                                .gap_1()
                                .max_w(px(500.))
                                .overflow_x_hidden()
                                .flex_nowrap()
                                .child(
                                    Button::new("finish-label")
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
                                Button::new("dropdown-menu-scrollable-2")
                                    .outline()
                                    .label("Scrollable Menu (5 items)")
                                    .dropdown_menu_with_anchor(Corner::TopLeft, move |this, _, _| {
                                        let mut this = this
                                            .scrollable(true)
                                            .max_h(px(300.))
                                            .label(format!("Total {} items", 100));
                                        for i in 0..5 {
                                            this = this.menu(
                                                SharedString::from(format!("Item {}", i)),
                                                Box::new(Info(i)),
                                            )
                                        }
                                        this.min_w(px(100.))
                                    }),
                            )

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
                            .child(self.label_popover_list.clone())
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
                    )
                    .child(Button::new("12").items_end().justify_end().label("Save").on_click({
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
                                let item = ItemModel {
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
                                    priority: Some(view.priority_state.read(cx).priority() as i32),
                                    ..Default::default()
                                };
                                cx.emit(ItemInfoEvent::Update(item.into()));
                                cx.notify();
                            });
                        }
                    })),
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
