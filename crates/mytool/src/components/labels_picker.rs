use std::rc::Rc;

use gpui::{
    Action, App, AppContext, Context, ElementId, Empty, Entity, EventEmitter, FocusHandle,
    Focusable, Hsla, InteractiveElement as _, IntoElement, KeyBinding, MouseButton,
    ParentElement as _, Render, RenderOnce, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Subscription, Window, actions, anchored, deferred, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable, Size, StyleSized as _, StyledExt as _,
    checkbox::Checkbox,
    h_flex,
    list::{ListEvent, ListState},
    v_flex,
};
use serde::Deserialize;
use todos::entity::LabelModel;

use crate::{LabelListDelegate, todo_state::LabelState};

actions!(labels_picker, [LabelsPickerCancel, LabelsPickerDelete,]);
#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = labels_picker, no_json)]
pub struct LabelsPickerConfirm {
    /// Is confirm with secondary.
    pub secondary: bool,
}
#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = labels_picker, no_json)]
pub struct LabelsPickerCheck {
    /// Is confirm with secondary.
    pub select: bool,
}

const CONTEXT: &'static str = "LabelPicker";
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", LabelsPickerConfirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("escape", LabelsPickerCancel, Some(CONTEXT)),
        KeyBinding::new("delete", LabelsPickerDelete, Some(CONTEXT)),
        KeyBinding::new("backspace", LabelsPickerDelete, Some(CONTEXT)),
    ])
}

#[derive(Clone)]
pub enum LabelPickerEvent {
    Added(Rc<LabelModel>),
    Removed(Rc<LabelModel>),
}

/// Use to store the state of the date picker.
pub struct LabelPickerState {
    focus_handle: FocusHandle,
    label_list: Entity<ListState<LabelListDelegate>>,
    checked_labels: Vec<Rc<LabelModel>>,
    open: bool,
    _subscriptions: Vec<Subscription>,
}

impl Focusable for LabelPickerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<LabelPickerEvent> for LabelPickerState {}

impl LabelPickerState {
    pub(crate) fn new_with_checked(
        checked_labels: Vec<Rc<LabelModel>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label_list = cx.new(|cx| {
            ListState::new(LabelListDelegate::new(), window, cx).searchable(true).selectable(false)
        });
        let label_list_clone = label_list.clone();
        let _subscriptions = vec![
            cx.observe_global::<LabelState>(move |_this, cx| {
                let labels = cx.global::<LabelState>().labels.clone();
                let _ = cx.update_entity(&label_list_clone, |list, cx| {
                    list.delegate_mut().update_labels(labels);
                    cx.notify();
                });
                cx.notify();
            }),
            cx.subscribe(&label_list, |_this, _, ev, _| match ev {
                ListEvent::Select(_ix) => {},
                _ => {},
            }),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            label_list,
            checked_labels,
            open: false,
            _subscriptions,
        }
    }

    fn on_escape(&mut self, _: &LabelsPickerCancel, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            cx.propagate();
        }

        self.focus_back_if_need(window, cx);
        self.open = false;

        cx.notify();
    }

    fn on_enter(&mut self, _: &LabelsPickerConfirm, _: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.open = true;
            cx.notify();
        }
    }

    fn on_delete(&mut self, _: &LabelsPickerDelete, _window: &mut Window, _cx: &mut Context<Self>) {
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

    fn toggle_checked_labels(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        println!("toggle_checked_labels: {}", checked);
        let mut changed_label: Option<Rc<LabelModel>> = None;
        self.label_list.update(cx, |list, _cx| {
            if let Some(ix) = &list.delegate().selected_index {
                if let Some(label) = list
                    .delegate()
                    .matched_labels
                    .get(ix.section)
                    .and_then(|c| c.get(ix.row))
                    .cloned()
                {
                    changed_label = Some(label.clone());

                    let exists = self.checked_labels.iter().any(|l| Rc::ptr_eq(l, &label));

                    if *checked && !exists {
                        self.checked_labels.push(label.clone());
                    } else if !*checked && exists {
                        self.checked_labels.retain(|l| !Rc::ptr_eq(l, &label));
                    }
                }
            }
        });

        if let Some(label) = changed_label {
            let event = if *checked {
                LabelPickerEvent::Added(label)
            } else {
                LabelPickerEvent::Removed(label)
            };
            cx.emit(event);
            cx.notify();
        }
    }

    fn checked_preset(
        &mut self,
        checked_labels: Vec<Rc<LabelModel>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.checked_labels = checked_labels;
        self.label_list.update(cx, |state, cx| {
            state.delegate_mut().checked_labels = self.checked_labels.clone();
            cx.notify();
        })
    }
}

/// A DatePicker element.
#[derive(IntoElement)]
pub struct LabelPicker {
    id: ElementId,
    style: StyleRefinement,
    state: Entity<LabelPickerState>,
    cleanable: bool,
    placeholder: Option<SharedString>,
    size: Size,
    appearance: bool,
    disabled: bool,
}

impl Sizable for LabelPicker {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Focusable for LabelPicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for LabelPicker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for LabelPicker {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Render for LabelPickerState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui::IntoElement {
        Empty
    }
}

impl LabelPicker {
    /// Create a new DatePicker with the given [`LabelPickerState`].
    pub fn new(state: &Entity<LabelPickerState>) -> Self {
        Self {
            id: ("date-picker", state.entity_id()).into(),
            state: state.clone(),
            cleanable: false,
            placeholder: None,
            size: Size::default(),
            style: StyleRefinement::default(),
            appearance: true,
            disabled: false,
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

impl RenderOnce for LabelPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // This for keep focus border style, when click on the popup.
        let _is_focused = self.focus_handle(cx).contains_focused(window, cx);
        let state = self.state.read(cx);
        let checked_labels = state.checked_labels.clone();
        let label_list = state.label_list.read(cx).delegate()._labels.clone();
        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .on_action(window.listener_for(&self.state, LabelPickerState::on_enter))
            .on_action(window.listener_for(&self.state, LabelPickerState::on_delete))
            .when(state.open, |this| {
                this.on_action(window.listener_for(&self.state, LabelPickerState::on_escape))
            })
            .flex_1()
            .w_full()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(
                div()
                    .id("label-picker-input")
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .overflow_hidden()
                    .input_text_size(self.size)
                    .input_size(self.size)
                    .when(!state.open && !self.disabled, |this| {
                        this.on_click(
                            window.listener_for(&self.state, LabelPickerState::toggle_labels),
                        )
                    }),
            )
            .when(state.open, |this| {
                this.child(
                    deferred(
                        anchored().snap_to_window_with_margin(px(8.)).child(
                            div()
                                .occlude()
                                .mt_1p5()
                                .p_3()
                                .border_1()
                                .border_color(cx.theme().border)
                                .shadow_lg()
                                .rounded((cx.theme().radius * 2.).min(px(8.)))
                                .bg(cx.theme().popover)
                                .text_color(cx.theme().popover_foreground)
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    window.listener_for(&self.state, |view, _, window, cx| {
                                        view.on_escape(&LabelsPickerCancel, window, cx);
                                    }),
                                )
                                .child(
                                    v_flex()
                                        .gap_3()
                                        .h_full()
                                        .items_start()
                                        .children(label_list.iter().enumerate().map(|(_ix, label)| {
                                            h_flex()
                                                .px_2()
                                                .py_1()
                                                // .overflow_x_hidden()
                                                .border_1()
                                                .rounded(cx.theme().radius)
                                                .child(
                                                    h_flex().items_center().justify_between().gap_2().child(
                                                        h_flex()
                                                            .gap_2()
                                                            .items_center()
                                                            .justify_end()
                                                            .child(
                                                                Checkbox::new("is-checked")
                                                                    .checked(checked_labels.contains(&label.clone()))
                                                                    .on_click(window.listener_for(&self.state, LabelPickerState::toggle_checked_labels))
                                                            )
                                                            .child(
                                                                Icon::build(IconName::TagOutlineSymbolic).text_color(Hsla::from(
                                                                    gpui::rgb(
                                                                        u32::from_str_radix(&label.color[1..], 16)
                                                                            .ok()
                                                                            .unwrap_or_default(),
                                                                    ),
                                                                )),
                                                            )
                                                            .child(div().w(px(120.)).child(label.name.clone())),
                                                    ),
                                                )
                                        }))
                                ),
                        ),
                    )
                        .with_priority(2),
                )
            })
    }
}
