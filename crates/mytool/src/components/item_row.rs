use std::rc::Rc;

use gpui::{
    App, AppContext, Context, ElementId, Entity, EventEmitter, InteractiveElement, IntoElement,
    ParentElement as _, Render, RenderOnce, StyleRefinement, Styled, Subscription, Window, div,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable, Size, StyledExt as _, button::Button, collapsible::Collapsible,
    h_flex, v_flex,
};
use todos::entity::ItemModel;

use crate::{ItemInfo, ItemInfoEvent, ItemInfoState, ItemListItem, todo_state::ItemState};

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
    pub item: Rc<ItemModel>,
    item_info: Entity<ItemInfoState>,
    is_open: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ItemRowEvent> for ItemRowState {}
impl ItemRowState {
    pub fn new(item: Rc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
        let item_id = item.id.clone();

        let _subscriptions = vec![
            cx.observe_global_in::<ItemState>(window, move |this, window, cx| {
                let state_items = cx.global::<ItemState>().items.clone();
                if let Some(updated_item) = state_items.iter().find(|i| i.id == item_id) {
                    if updated_item != &this.item {
                        this.item = updated_item.clone();
                        this.item_info.update(cx, |this_info, cx| {
                            this_info.set_item(updated_item.clone(), window, cx);
                        });
                        cx.notify();
                    }
                }
            }),
            cx.subscribe(&item_info, |this, _, event: &ItemInfoEvent, cx| {
                this.item_info.update(cx, |state, cx| {
                    state.handle_item_info_event(event, cx);
                });
            }),
        ];

        Self { item, item_info, is_open: false, _subscriptions }
    }
}

impl Render for ItemRowState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let text_color =
            if self.is_open { cx.theme().accent_foreground } else { cx.theme().foreground };
        let item = self.item.clone();
        let item_info = self.item_info.clone();
        let is_open = self.is_open;
        let item_id = format!("item-{}", self.item.id);
        let view = cx.entity();
        div().border_3().id(item_id.clone()).rounded(px(5.0)).child(
            Collapsible::new()
                .gap_1()
                .open(is_open)
                .child(
                    h_flex()
                        .items_center()
                        .justify_start()
                        .gap_2()
                        .text_color(text_color)
                        .child(ItemListItem::new(item_id.clone(), item.clone(), false))
                        .child(
                            Button::new("toggle2")
                                .small()
                                .outline()
                                .icon(IconName::ChevronDown)
                                .when(is_open, |this| this.icon(IconName::ChevronUp))
                                .on_click(move |_event, _window, cx| {
                                    cx.update_entity(&view, |this, cx| {
                                        this.is_open = !this.is_open;
                                        cx.notify();
                                    })
                                }),
                        ),
                )
                .content(v_flex().gap_2().child(ItemInfo::new(&item_info))),
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
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .w_full()
            .refine_style(&self.style)
            .child(self.state)
    }
}
