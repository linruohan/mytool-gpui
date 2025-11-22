use std::rc::Rc;

use gpui::{
    Action, App, AppContext, Context, Corner, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce,
    SharedString, StyleRefinement, Styled, Subscription, Window, div, px,
};
use gpui_component::{
    IndexPath, Sizable, Size, StyleSized, StyledExt as _,
    button::Button,
    list::{ListEvent, ListState},
    menu::DropdownMenu,
    v_flex,
};
use serde::Deserialize;
use todos::{entity::LabelModel, enums::item_priority::ItemPriority};

use crate::{DBState, LabelCheckListDelegate, load_labels};

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = priority, no_json)]
struct LabelsInfo(i32);

pub enum LabelsEvent {
    Selected(i32),
}
pub struct LabelsState {
    focus_handle: FocusHandle,
    label_list: Entity<ListState<LabelCheckListDelegate>>,
    pub selected_labels: Vec<Rc<LabelModel>>,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<LabelsEvent> for LabelsState {}
impl Focusable for LabelsState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl LabelsState {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let label_list = cx.new(|cx| ListState::new(LabelCheckListDelegate::new(), window, cx));
        let _subscriptions =
            vec![cx.subscribe_in(&label_list, window, |this, _, ev: &ListEvent, _window, cx| {
                if let ListEvent::Confirm(ix) = ev
                    && let Some(label) = this.get_selected_label(*ix, cx)
                {
                    this.selected_labels.push(label);
                }
            })];

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
            _subscriptions,
            focus_handle: cx.focus_handle(),
            label_list,
            selected_labels: Vec::new(),
        }
    }

    fn get_selected_label(&self, ix: IndexPath, cx: &App) -> Option<Rc<LabelModel>> {
        self.label_list
            .read(cx)
            .delegate()
            .matched_labels
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }

    fn on_action_info(&mut self, info: &LabelsInfo, _window: &mut Window, cx: &mut Context<Self>) {
        // self.priority = ItemPriority::from_i32(info.0);
        cx.emit(LabelsEvent::Selected(info.0));
        cx.notify();
    }
}

/// A DatePicker element.
#[derive(IntoElement)]
pub struct LabelsButton {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<LabelsState>,
}

impl Sizable for LabelsButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Focusable for LabelsButton {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for LabelsButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for LabelsState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        v_flex().on_action(cx.listener(Self::on_action_info)).child(
            Button::new("labels").outline().dropdown_menu_with_anchor(
                Corner::TopLeft,
                move |this, _, _| {
                    let mut this = this.scrollable(true).max_h(px(400.));
                    for p in ItemPriority::all() {
                        let p1 = p.clone() as i32;
                        this = this
                            .menu(SharedString::from(p.display_name()), Box::new(LabelsInfo(p1)))
                    }
                    this.min_w(px(100.))
                },
            ),
        )
    }
}

impl LabelsButton {
    /// Create a new DatePicker with the given [`LabelsState`].
    pub fn new(state: &Entity<LabelsState>) -> Self {
        Self {
            id: ("item-labels", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for LabelsButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .flex_none()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
