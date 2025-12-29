use std::{collections::HashMap, rc::Rc};

use gpui::{
    actions, anchored, deferred, div, prelude::FluentBuilder as _, px, Action, App, AppContext,
    Context, ElementId, Empty, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement as _,
    Render, RenderOnce, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants}, checkbox::Checkbox, h_flex, label::Label, red_400, tag::Tag, v_flex,
    ActiveTheme,
    Disableable,
    IconName,
    Sizable,
    Size,
    StyleSized as _,
    StyledExt as _,
};
use serde::Deserialize;
use todos::entity::{ItemModel, LabelModel};
use tokio::io::AsyncReadExt;

use crate::{section, todo_state::LabelState, ItemInfo, ItemInfoEvent, ItemInfoState};

actions!(item_row, [ItemRowCancel, ItemRowDelete,]);
#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = item_row, no_json)]
pub struct ItemRowConfirm {
    /// Is confirm with secondary.
    pub secondary: bool,
}
#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = item_row, no_json)]
pub struct ItemRowCheck {
    /// Is confirm with secondary.
    pub select: bool,
}

const CONTEXT: &'static str = "item_row";
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", ItemRowConfirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("escape", ItemRowCancel, Some(CONTEXT)),
        KeyBinding::new("delete", ItemRowDelete, Some(CONTEXT)),
        KeyBinding::new("backspace", ItemRowDelete, Some(CONTEXT)),
    ])
}

#[derive(Clone)]
pub enum ItemRowEvent {
    Added(Rc<LabelModel>),
    Removed(Rc<LabelModel>),
}

/// Use to store the state of the date picker.
pub struct ItemRowState {
    focus_handle: FocusHandle,
    item: Rc<ItemModel>,
    item_info: Entity<ItemInfoState>,
    checked: bool,
    open: bool,
    _subscriptions: Vec<Subscription>,
}

impl Focusable for ItemRowState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<ItemRowEvent> for ItemRowState {}

impl ItemRowState {
    pub fn new(item: Rc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = item.clone();
        let item_info = cx.new(|cx| ItemInfoState::new(window, cx));
        let _subscriptions =
            vec![cx.subscribe(&item_info, |this, _, _event: &ItemInfoEvent, cx| {
                this.item_info.update(cx, |_item_info, _cx| {
                    // item_info.handel_item_info_event(event, cx);
                });
            })];

        Self {
            focus_handle: cx.focus_handle(),
            item: item.clone(),
            item_info,
            checked: item.clone().checked,
            open: false,
            _subscriptions,
        }
    }

    fn on_escape(&mut self, _: &ItemRowCancel, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            cx.propagate();
        }

        self.focus_back_if_need(window, cx);
        self.open = false;

        cx.notify();
    }

    fn on_enter(&mut self, _: &ItemRowConfirm, _: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.open = true;
            cx.notify();
        }
    }

    fn on_delete(&mut self, _: &ItemRowDelete, _window: &mut Window, _cx: &mut Context<Self>) {
        // self.clean(&ClickEvent::default(), window, cx);
    }

    // To focus the Picker Input, if current focus in is on the container.
    //
    // This is because mouse down out the Calendar, GPUI will move focus to the container.
    // So we need to move focus back to the Picker Input.
    //
    // But if mouse down target is some other focusable element (e.g.: [`crate::Input`]), we should
    // not move focus.
    fn focus_back_if_need(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }

        if let Some(focused) = window.focused(cx) {
            if focused.contains(&self.focus_handle, window) {
                self.focus_handle.focus(window, cx);
            }
        }
    }

    // 显示label list
    fn toggle_labels(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }

    fn toggle_checked_labels(&mut self, checked: &bool, _: &mut Window, _cx: &mut Context<Self>) {
        println!("toggle_checked_labels: {}", checked);
        let _changed_label: Option<Rc<LabelModel>> = None;
        // self.label_list.update(cx, |list, _cx| {
        //     if let Some(ix) = &list.delegate().selected_index {
        //         if let Some(label) = list
        //             .delegate()
        //             .matched_labels
        //             .get(ix.section)
        //             .and_then(|c| c.get(ix.row))
        //             .cloned()
        //         {
        //             changed_label = Some(label.clone());
        //
        //             let exists = self.checked_labels.iter().any(|l| Rc::ptr_eq(l, &label));
        //
        //             if *checked && !exists {
        //                 self.checked_labels.push(label.clone());
        //             } else if !*checked && exists {
        //                 self.checked_labels.retain(|l| !Rc::ptr_eq(l, &label));
        //             }
        //         }
        //     }
        // });
        //
        // if let Some(label) = changed_label {
        //     let event = if *checked {
        //         LabelPickerEvent::Added(label)
        //     } else {
        //         LabelPickerEvent::Removed(label)
        //     };
        //     cx.emit(event);
        //     cx.notify();
        // }
    }

    fn checked_preset(
        &mut self,
        _checked_labels: Vec<Rc<LabelModel>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // self.checked_labels = checked_labels;
        // self.label_list.update(cx, |state, cx| {
        //     state.delegate_mut().checked_labels = self.checked_labels.clone();
        //     cx.notify();
        // })
    }
}

/// A DatePicker element.
#[derive(IntoElement)]
pub struct ItemRow {
    id: ElementId,
    style: StyleRefinement,
    state: Entity<ItemRowState>,
    cleanable: bool,
    selected: bool,
    placeholder: Option<SharedString>,
    size: Size,
    appearance: bool,
    disabled: bool,
}

impl Sizable for ItemRow {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Focusable for ItemRow {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for ItemRow {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for ItemRow {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Render for ItemRowState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui::IntoElement {
        Empty
    }
}

impl ItemRow {
    /// Create a new DatePicker with the given [`ItemRowState`].
    pub fn new(state: &Entity<ItemRowState>) -> Self {
        Self {
            id: ("date-picker", state.entity_id()).into(),
            state: state.clone(),
            cleanable: false,
            placeholder: None,
            size: Size::default(),
            style: StyleRefinement::default(),
            appearance: true,
            disabled: false,
            selected: false,
        }
    }

    /// Set the placeholder of the date picker, default: "".
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set whether to show the clear button when the input field is not empty, default is false.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }

    /// Set appearance of the date picker, if false, the date picker will be in a minimal style.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }
}

impl RenderOnce for ItemRow {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // This for keep focus border style, when click on the popup.
        let _is_focused = self.focus_handle(cx).contains_focused(window, cx);
        let state = self.state.read(cx);
        let item = state.item.clone();
        let item_info = state.item_info.clone();
        let text_color =
            if self.selected { cx.theme().accent_foreground } else { cx.theme().foreground };

        let labels = cx.global::<LabelState>().labels.clone();
        let label_map: HashMap<&str, &Rc<LabelModel>> =
            labels.iter().map(|l| (l.id.as_str(), l)).collect();
        let item_labels = &item.labels;
        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            // .on_action(window.listener_for(&self.state, ItemRowState::on_enter))
            // .on_action(window.listener_for(&self.state, ItemRowState::on_delete))
            // .when(state.open, |this| {
            //     this.on_action(window.listener_for(&self.state, ItemRowState::on_escape))
            // })
            .flex_1()
            .w_full()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            // .child(
            //     div()
            //         .id("item-row")
            //         .relative()
            //         .flex()
            //         .items_center()
            //         .justify_between()
            //         .overflow_hidden()
            //         .input_text_size(self.size)
            // )
            .child(
                h_flex()
                    .items_center()
                    .justify_start()
                    .gap_2()
                    .text_color(text_color)
                    .child(Checkbox::new("item-finished").checked(item.checked))
                    .child(
                        Label::new("Tomorrow").when(item.checked, |this| {
                            this.line_through().text_color(red_400())
                        }),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .overflow_x_hidden()
                            .flex_nowrap()
                            .child(
                                Label::new(item.content.clone())
                                    .whitespace_nowrap()
                                    .when(item.checked, |this| this.line_through()),
                            )
                            .when(item.labels.is_some(), |this| {
                                this.child(
                                    h_flex().gap_2().flex().children(
                                        item_labels
                                            .iter()
                                            .flat_map(|group| {
                                                group.split(';').filter(|id| !id.is_empty())
                                            })
                                            .filter_map(|id| {
                                                label_map.get(id).map(|label| {
                                                    Tag::primary().child(label.name.clone())
                                                })
                                            })
                                            .collect::<Vec<_>>(),
                                    ),
                                )
                            }),
                    )
                    .child(item.priority.unwrap_or_default().to_string())
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .justify_end()
                            .flex()
                            .px_2()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                Button::new("edit")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::EditSymbolic)
                                    .on_click(move |_event, _window, _cx| {
                                        let item = item.clone();
                                        println!("edit item:{:?}", item);
                                    }),
                            )
                            .child(
                                Button::new("delete")
                                    .icon(IconName::UserTrashSymbolic)
                                    .small()
                                    .ghost()
                                    .on_click(|_, _, _cx| {
                                        println!("delete item:");
                                    }),
                            )
                            .child(
                                Button::new("详情")
                                    .tooltip("show item info")
                                    .small().icon(IconName::ViewMoreSymbolic)
                                    .ghost()
                                    .compact()
                                    .when(!state.open && !self.disabled, |this| {
                                        this.on_click(
                                            window.listener_for(&self.state, ItemRowState::toggle_labels),
                                        )
                                    })
                            )
                    )
            )
            .when(state.open, |this| {
                this.child(
                    deferred(
                        anchored().snap_to_window_with_margin(px(8.)).child(
                            div()
                                .mt_1p5()
                                .p_3()
                                .w_full()
                                .border_1()
                                .border_color(cx.theme().border)
                                .shadow_lg()
                                .rounded((cx.theme().radius * 2.).min(px(8.)))
                                .bg(cx.theme().popover)
                                .text_color(cx.theme().popover_foreground)
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    window.listener_for(&self.state, |view, _, window, cx| {
                                        view.on_escape(&ItemRowCancel, window, cx);
                                    }),
                                )
                                .child(section("item_info").child(ItemInfo::new(&item_info)))
                        ),
                    )
                        .with_priority(2),
                )
            })
    }
}
