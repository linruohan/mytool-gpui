use std::{collections::HashMap, rc::Rc};

use gpui::{
    actions, anchored, deferred, div, prelude::FluentBuilder, px, Action, App, AppContext,
    Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, MouseButton, ParentElement as _, Render, RenderOnce, StyleRefinement, Styled,
    Subscription, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants}, checkbox::Checkbox, h_flex, label::Label, red_400,
    tag::Tag,
    v_flex,
    ActiveTheme,
    IconName,
    Sizable,
    Size,
    StyledExt as _,
};
use serde::Deserialize;
use todos::entity::{ItemModel, LabelModel};

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

const CONTEXT: &'static str = "ItemRow";
#[derive(Clone)]
pub enum ItemRowEvent {
    Updated(Rc<ItemModel>),    // 更新任务
    Added(Rc<ItemModel>),      // 新增任务
    Finished(Rc<ItemModel>),   // 状态改为完成
    UnFinished(Rc<ItemModel>), // 状态改为未完成
    Deleted(Rc<ItemModel>),    // 删除任务
}
pub struct ItemRowState {
    focus_handle: FocusHandle,
    pub item: Rc<ItemModel>,
    item_info: Entity<ItemInfoState>,
    open: bool,
    _subscriptions: Vec<Subscription>,
    checked: bool,
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
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
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
            open: false,
            _subscriptions,
            checked: item.clone().checked,
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

        if let Some(focused) = window.focused(cx)
            && focused.contains(&self.focus_handle, window)
        {
            self.focus_handle.focus(window, cx);
        }
    }

    // pub fn on_labels_event(
    //     &mut self,
    //     _state: &Entity<LabelsPopoverList>,
    //     event: &LabelsPopoverEvent,
    //     _window: &mut Window,
    //     _cx: &mut Context<Self>,
    // ) {
    //     match event {
    //         LabelsPopoverEvent::Selected(label) => {
    //             self.add_checked_labels(label.clone());
    //         },
    //         LabelsPopoverEvent::DeSelected(label) => {
    //             self.rm_checked_labels(label.clone());
    //         },
    //     }
    // }

    fn toggle_finished(&mut self, selectable: &bool, _: &mut Window, _cx: &mut Context<Self>) {
        self.checked = *selectable;
    }

    // 显示label list
    fn toggle_labels(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }
}

impl Render for ItemRowState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let labels = cx.global::<LabelState>().labels.clone();
        let label_map: HashMap<&str, &Rc<LabelModel>> =
            labels.iter().map(|l| (l.id.as_str(), l)).collect();
        let item_labels = &self.item.labels;
        let item = self.item.clone();
        div()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            // .on_action(window.listener_for(&self.state, ItemRowState::on_enter))
            // .on_action(window.listener_for(&self.state, ItemRowState::on_delete))
            .when(self.open, |this| {
                this.on_action(cx.listener(ItemRowState::on_escape))
            })
            .flex_1()
            .w_full()
            .relative()
            .child(
                h_flex()
                    .items_center()
                    .justify_start()
                    .gap_2()
                    .child(Checkbox::new("item-finished").checked(self.checked).on_click(cx.listener(move |view, checked, _window, _cx| {
                        view.checked = *checked;
                    }
                    )))
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
                                    .when(!self.open, |this| {
                                        this.on_click(
                                            cx.listener(ItemRowState::toggle_labels),
                                        )
                                    })
                            )
                    )
            )
            .when(self.open, |this| {
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
                                    cx.listener(|view, _, window, cx| {
                                        view.on_escape(&ItemRowCancel, window, cx);
                                    }),
                                )
                                .child(section("item_info").child(ItemInfo::new(&self.item_info)))
                        ),
                    )
                        .with_priority(2),
                )
            })
    }
}

/// A DatePicker element.
#[derive(IntoElement)]
pub struct ItemRow {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<ItemRowState>,
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

impl ItemRow {
    /// Create a new DatePicker with the given [`ItemRowState`].
    pub fn new(state: &Entity<ItemRowState>) -> Self {
        Self {
            id: ("item-info", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for ItemRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .w_full()
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
