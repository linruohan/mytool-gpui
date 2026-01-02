use std::{collections::HashMap, rc::Rc};

use gpui::{
    App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, MouseButton, ParentElement as _, Render, RenderOnce,
    StyleRefinement, Styled, Subscription, Window, div, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable, Size, StyledExt as _,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    collapsible::Collapsible,
    h_flex,
    label::Label,
    red_400,
    tag::Tag,
    v_flex,
};
use todos::entity::{ItemModel, LabelModel};

use crate::{ItemInfo, ItemInfoState, todo_state::LabelState};

const CONTEXT: &str = "ItemRow";
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
    is_open: bool,
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
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));

        let _subscriptions = vec![];
        Self {
            focus_handle: cx.focus_handle(),
            item: item.clone(),
            item_info,
            is_open: false,
            _subscriptions,
        }
    }
}

impl Render for ItemRowState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let text_color =
            if self.is_open { cx.theme().accent_foreground } else { cx.theme().foreground };
        let labels = cx.global::<LabelState>().labels.clone();
        let label_map: HashMap<&str, &Rc<LabelModel>> =
            labels.iter().map(|l| (l.id.as_str(), l)).collect();
        let item_labels = &self.item.labels;
        let item = self.item.clone();
        let item_info = self.item_info.clone();
        let is_open = self.is_open;
        let item_id = format!("item-{}", self.item.id.clone());
        let view = cx.entity();
        div().id(item_id).child(
            Collapsible::new()
                .gap_1()
                .open(is_open)
                .child(
                    h_flex()
                        .items_center()
                        .justify_start()
                        .gap_2()
                        .text_color(text_color)
                        .child(Checkbox::new("item-finished").checked(self.item.checked))
                        .child(Label::new("Tomorrow").when(self.item.checked, |this| {
                            this.line_through().text_color(red_400())
                        }))
                        .child(
                            v_flex()
                                .gap_1()
                                .overflow_x_hidden()
                                .flex_nowrap()
                                .child(
                                    Label::new(self.item.content.clone())
                                        .whitespace_nowrap()
                                        .when(self.item.checked, |this| this.line_through()),
                                )
                                .when(self.item.labels.is_some(), |this| {
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
                        .child(self.item.priority.unwrap_or_default().to_string())
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
                                ),
                        )
                        .child(
                            Button::new("toggle2")
                                .label("Details")
                                .small()
                                .outline()
                                .icon(IconName::ChevronDown)
                                .when(is_open, |this| this.icon(IconName::ChevronUp))
                                .on_click(move |_event, _window, cx| {
                                    let view = view.clone();
                                    cx.update_entity(&view, |this, cx| {
                                        this.is_open = !this.is_open;
                                        cx.notify();
                                    })
                                }),
                        ),
                )
                .content(v_flex().gap_2().child(ItemInfo::new(&item_info.clone()))),
        )
    }
}

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
