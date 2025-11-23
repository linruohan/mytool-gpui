use std::{collections::HashSet, rc::Rc};

use gpui::{
    Action, App, AppContext, Context, ElementId, Empty, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement as _,
    Render, RenderOnce, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Subscription, Window, actions, anchored, deferred, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, IndexPath, Sizable, Size, StyleSized, StyledExt,
    calendar::Matcher, checkbox::Checkbox, h_flex, list::ListState, v_flex,
};
use serde::Deserialize;

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
use todos::entity::LabelModel;

use crate::{DBState, LabelListDelegate, load_labels};

const CONTEXT: &'static str = "LabelsPicker";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", LabelsPickerConfirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("escape", LabelsPickerCancel, Some(CONTEXT)),
        KeyBinding::new("delete", LabelsPickerDelete, Some(CONTEXT)),
        KeyBinding::new("backspace", LabelsPickerDelete, Some(CONTEXT)),
    ])
}
/// The date of the calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    Single(Option<String>),
    Range(Option<Vec<String>>),
}

/// Events emitted by the LabelPicker.
#[derive(Clone)]
pub enum LabelsPickerEvent {
    Selected(Rc<LabelModel>),
    DeSelected(Rc<LabelModel>),
}

/// Preset value for DateRangePreset.
#[derive(Clone)]
pub enum DateRangePresetValue {
    Single(String),
    Range(Vec<String>),
}

/// Preset for date range selection.
#[derive(Clone)]
pub struct DateRangePreset {
    label: SharedString,
    value: DateRangePresetValue,
}

impl DateRangePreset {
    /// Creates a new DateRangePreset with a date.
    pub fn single(label: impl Into<SharedString>, date: String) -> Self {
        DateRangePreset { label: label.into(), value: DateRangePresetValue::Single(date) }
    }

    /// Creates a new DateRangePreset with a range of dates.
    pub fn range(label: impl Into<SharedString>, labels: Vec<String>) -> Self {
        DateRangePreset { label: label.into(), value: DateRangePresetValue::Range(labels) }
    }
}

/// Use to store the state of the date picker.
pub struct LabelsPickerState {
    focus_handle: FocusHandle,
    open: bool,
    active_index: usize,
    label_list: Entity<ListState<LabelListDelegate>>,
    selected_labels: Vec<Rc<LabelModel>>,
    disabled_matcher: Option<Rc<Matcher>>,
    _subscriptions: Vec<Subscription>,
}

impl Focusable for LabelsPickerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<LabelsPickerEvent> for LabelsPickerState {}

impl LabelsPickerState {
    /// Create a date state.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let label_list = cx.new(|cx| {
            ListState::new(LabelListDelegate::new(), window, cx).searchable(true).selectable(true)
        });

        let _subscriptions = vec![
            // cx.subscribe_in(&label_list, window, |this, _, ev: &ListEvent, _window, cx| {
            //     if let ListEvent::Confirm(ix) = ev
            //         && let Some(conn) = this.get_selected_label(*ix, cx)
            //     {
            //         // this.update_active_index(Some(ix.row));
            //         println!("ix.row: {}; label:{:?}", ix.row, conn);
            //     }
            // })
        ];
        let label_list_clone = label_list.clone();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |_view, cx| {
            let db = db.lock().await;
            let labels = load_labels(db.clone()).await;
            let rc_labels: Vec<Rc<LabelModel>> =
                labels.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("label_picker: len labels: {}", labels.len());
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
            label_list,
            open: false,
            disabled_matcher: None,
            selected_labels: Vec::new(),
            active_index: 0,
            _subscriptions,
        }
    }

    pub fn add_selected_label(&mut self, ix: IndexPath, cx: &mut Context<Self>) {
        let _ = self
            .label_list
            .read(cx)
            .delegate()
            .matched_labels
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
            .map(|label| {
                // 避免重复添加
                if !self.selected_labels.contains(&label) {
                    self.selected_labels.push(label.clone());
                    cx.emit(LabelsPickerEvent::Selected(label));
                }
            });
    }

    pub fn del_selected_label(&mut self, ix: IndexPath, cx: &mut Context<Self>) {
        let _ = self
            .label_list
            .read(cx)
            .delegate()
            .matched_labels
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
            .map(|label| {
                // 从 selected_labels 中移除
                self.selected_labels.retain(|l| l != &label);
                cx.emit(LabelsPickerEvent::DeSelected(label));
            });
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

    fn on_check_toggle(
        &mut self,
        event: &(LabelsPickerCheck, bool),
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (_, selectable) = event;
        let ix = IndexPath::new(self.active_index);
        if *selectable {
            self.add_selected_label(ix, cx);
        } else {
            self.del_selected_label(ix, cx);
        }
        cx.notify();
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
                self.focus_handle.focus(window);
            }
        }
    }

    // 打开labels列表
    fn toggle_labels(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }
}

/// A LabelPicker element.
#[derive(IntoElement)]
pub struct LabelsPicker {
    id: ElementId,
    style: StyleRefinement,
    state: Entity<LabelsPickerState>,
    cleanable: bool,
    placeholder: Option<SharedString>,
    checked_list: HashSet<IndexPath>,
    size: Size,
    appearance: bool,
    disabled: bool,
}

impl Sizable for LabelsPicker {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Focusable for LabelsPicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for LabelsPicker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for LabelsPicker {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Render for LabelsPickerState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui::IntoElement {
        Empty
    }
}

impl LabelsPicker {
    /// Create a new LabelPicker with the given [`LabelsPickerState`].
    pub fn new(state: &Entity<LabelsPickerState>) -> Self {
        Self {
            id: ("date-picker", state.entity_id()).into(),
            state: state.clone(),
            cleanable: false,
            placeholder: None,
            size: Size::default(),
            style: StyleRefinement::default(),
            appearance: true,
            disabled: false,
            checked_list: HashSet::new(),
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

    fn toggle_checked(&mut self, _checked: bool, _window: &mut Window, _cx: &mut App) {
        // if checked {
        //     self.add_selected_label(ix);
        // } else {
        //     self.del_selected_label(ix);
        // }
    }
}

impl RenderOnce for LabelsPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let label_list = self.state.read(cx).label_list.read(cx).delegate()._labels.clone();
        // This for keep focus border style, when click on the popup.
        let is_focused = self.focus_handle(cx).contains_focused(window, cx);
        let state = self.state.read(cx);
        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .on_action(window.listener_for(&self.state, LabelsPickerState::on_enter))
            .when(state.open, |this| {
                this.on_action(window.listener_for(&self.state, LabelsPickerState::on_escape))
            })
            .flex_none()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(
                div()
                    .id("label-picker-btn")
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .when(self.appearance, |this| {
                        this.bg(cx.theme().background)
                            .border_1()
                            .border_color(cx.theme().input)
                            .rounded(cx.theme().radius)
                            .when(cx.theme().shadow, |this| this.shadow_xs())
                            .when(is_focused, |this| this.focused_border(cx))
                            .when(self.disabled, |this| {
                                this.bg(cx.theme().muted).text_color(cx.theme().muted_foreground)
                            })
                    })
                    .overflow_hidden()
                    .when(!state.open && !self.disabled, |this| {
                        this.on_click(
                            window.listener_for(&self.state, LabelsPickerState::toggle_labels),
                        )
                    })
                    .child(
                        Icon::new(IconName::TagOutlineSymbolic)
                            .text_color(cx.theme().muted_foreground),
                    ),
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
                                .bg(cx.theme().background)
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    window.listener_for(&self.state, |view, _, window, cx| {
                                        view.on_escape(&LabelsPickerCancel, window, cx);
                                    }),
                                )
                                .child(v_flex().gap_3().h_full().items_start().children(
                                    label_list.iter().enumerate().map(
                                        |(_ix, label): (usize, &Rc<LabelModel>)| {
                                            h_flex()
                                                .gap_3()
                                                .child(
                                                    Checkbox::new("label-check-1").checked(false),
                                                )
                                                .child(label.name.clone())
                                        },
                                    ),
                                )),
                        ),
                    )
                    .with_priority(2),
                )
            })
    }
}
