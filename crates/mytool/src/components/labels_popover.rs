use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Subscription, Window, px,
};
use gpui_component::{
    IconName, Sizable,
    button::{Button, ButtonVariants},
    list::{List, ListState},
    popover::Popover,
    v_flex,
};
use todos::entity::LabelModel;

use crate::{LabelCheckListDelegate, SelectedCheckLabel, todo_state::LabelState};

pub enum LabelsPopoverEvent {
    Selected(Rc<LabelModel>),
    DeSelected(Rc<LabelModel>),
}

pub struct LabelsPopoverList {
    focus_handle: FocusHandle,
    pub label_list: Entity<ListState<LabelCheckListDelegate>>,
    pub selected_labels: Vec<Rc<LabelModel>>,
    pub(crate) list_popover_open: bool,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<LabelsPopoverEvent> for LabelsPopoverList {}
impl LabelsPopoverList {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let parent = cx.entity();
        let label_list = cx.new(|cx| {
            ListState::new(LabelCheckListDelegate::new(parent), window, cx)
                .searchable(true)
                .selectable(true)
        });

        cx.focus_self(window);
        let label_list_clone = label_list.clone();
        let _subscriptions = vec![cx.observe_global::<LabelState>(move |_this, cx| {
            let labels = cx.global::<LabelState>().labels.clone();
            cx.update_entity(&label_list_clone, |list, cx| {
                list.delegate_mut().update_labels(labels);
                cx.notify();
            });
            cx.notify();
        })];
        Self {
            list_popover_open: false,
            label_list,
            focus_handle: cx.focus_handle(),
            selected_labels: Vec::new(),
            _subscriptions,
        }
    }

    pub fn set_item_checked_label_id(
        &mut self,
        label_ids: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let all_labels = self.label_list.read(cx).delegate()._labels.clone();
        self.selected_labels = label_ids
            .split(';')
            .filter_map(|label_id| {
                let trimmed_id = label_id.trim();
                if trimmed_id.is_empty() {
                    return None;
                }
                all_labels.iter().find(|label| label.id == trimmed_id).map(Rc::clone)
            })
            .collect();
        self.label_list.update(cx, |list, cx| {
            list.delegate_mut().set_item_checked_labels(self.selected_labels.clone(), window, cx);
        });
    }

    fn selected_label(&mut self, _: &SelectedCheckLabel, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.label_list.read(cx);
        if let Some(label) = picker.delegate().selected_label()
            && !self.selected_labels.contains(&label)
        {
            self.selected_labels.push(label.clone());
            cx.emit(LabelsPopoverEvent::Selected(label.clone()));
            cx.notify();
        }
        println!("Selected label: {:?}", self.selected_labels.len());
    }

    // fn unselected_label(
    //     &mut self,
    //     _: &SelectedCheckLabel,
    //     _: &mut Window,
    //     cx: &mut Context<Self>,
    // ) {
    //     let picker = self.label_list.read(cx);
    //     if let Some(label) = picker.delegate().selected_label() {
    //         if self.selected_labels.contains(&label) {
    //             self.selected_labels.retain(|l| l != &label);
    //             cx.emit(LabelsPopoverEvent::Selected(label.clone()));
    //             cx.notify();
    //         }
    //     }
    //     println!("un Selected label: {:?}", self.selected_labels.len());
    // }
}

impl Focusable for LabelsPopoverList {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
const CONTEXT: &str = "label-popover-list";
impl Render for LabelsPopoverList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .items_center()
            .justify_end()
            .on_action(cx.listener(Self::selected_label))
            .child(
                Popover::new("popover-list")
                    .p_0()
                    .text_sm()
                    .open(self.list_popover_open)
                    .on_open_change(cx.listener(move |this, open, _, cx| {
                        this.list_popover_open = *open;
                        cx.notify();
                    }))
                    .trigger(
                        Button::new("item-labels-button")
                            .small()
                            .ghost()
                            .compact()
                            .icon(IconName::TagOutlineSymbolic),
                    )
                    .track_focus(&self.label_list.focus_handle(cx))
                    .child(List::new(&self.label_list))
                    .w_64()
                    .h(px(200.)),
            )
    }
}
