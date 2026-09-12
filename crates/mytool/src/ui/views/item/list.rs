use std::sync::Arc;

use gpui::{
    App, Context, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Task, Window, actions, div,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IndexPath, Placement, Selectable, WindowExt,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    label::Label,
    list::{ListDelegate, ListItem, ListState},
    v_flex,
};
use todos::{entity::ItemModel, utils::datetime::DateTime};

use crate::{
    SemanticColors, label_chip, todo_actions::complete_item_optimistic, todo_state::TodoStore,
};

actions!(item, [SelectedItem]);
pub enum ItemEvent {
    Finished(Arc<ItemModel>),
    Added(Arc<ItemModel>),
    Modified(Arc<ItemModel>),
    Deleted(Arc<ItemModel>),
}

#[derive(IntoElement)]
pub struct ItemListItem {
    base: ListItem,
    item: Arc<ItemModel>,
    selected: bool,
}

impl ItemListItem {
    pub fn new(id: impl Into<ElementId>, item: Arc<ItemModel>, selected: bool) -> Self {
        ItemListItem { item, base: ListItem::new(id), selected }
    }
}

impl Selectable for ItemListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for ItemListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = SemanticColors::from_theme(cx);
        let text_color =
            if self.selected { cx.theme().accent_foreground } else { cx.theme().foreground };

        let due_label = self
            .item
            .due_date()
            .and_then(|due_date| due_date.datetime())
            .map(|datetime| DateTime::default().get_relative_date_from_date(&datetime));
        let due_color = if self.item.checked {
            cx.theme().muted_foreground
        } else if self.item.is_past_due() {
            colors.status_overdue
        } else if self.item.is_due_today() {
            colors.status_today
        } else {
            colors.status_scheduled
        };

        let item_label_chips: Vec<_> = self
            .item
            .labels
            .as_deref()
            .unwrap_or("")
            .split(';')
            .filter(|id| !id.is_empty())
            .filter_map(|id| {
                cx.global::<TodoStore>()
                    .get_label(id)
                    .map(|label| label_chip(label.name.clone(), &label.color))
            })
            .collect();

        let item_for_check = self.item.clone();

        self.base.px_1().py_0p5().flex_1().overflow_x_hidden().child(
            h_flex()
                .items_center()
                .justify_start()
                .gap_2()
                .w_full()
                .min_w_0()
                .text_color(text_color)
                .child(
                    div()
                        .id("item-check-wrap")
                        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                            cx.stop_propagation();
                        })
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .child(Checkbox::new("item-finished").checked(self.item.checked).on_click(
                            move |checked, _, cx| {
                                complete_item_optimistic(item_for_check.clone(), *checked, cx);
                            },
                        )),
                )
                .child(
                    v_flex().flex_1().min_w_0().overflow_x_hidden().flex_nowrap().child(
                        Label::new(self.item.content.clone())
                            .whitespace_nowrap()
                            .when(self.item.checked, |this| {
                                this.line_through().text_color(cx.theme().muted_foreground)
                            }),
                    ),
                )
                .when(!item_label_chips.is_empty(), |this| {
                    this.child(h_flex().gap_1().flex_shrink_0().children(item_label_chips))
                })
                .when_some(due_label, |this, due_label| {
                    this.child(
                        Label::new(due_label)
                            .text_sm()
                            .text_color(due_color)
                            .when(self.item.checked, |this| this.line_through()),
                    )
                }),
        )
    }
}

pub struct ItemListDelegate {
    pub _items: Vec<Arc<ItemModel>>,
    pub matched_items: Vec<Vec<Arc<ItemModel>>>,
    selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    query: SharedString,
}
impl Default for ItemListDelegate {
    fn default() -> Self {
        Self::new()
    }
}
impl ItemListDelegate {
    pub fn new() -> Self {
        Self {
            _items: vec![],
            matched_items: vec![],
            selected_index: None,
            confirmed_index: None,
            query: "".into(),
        }
    }

    fn prepare(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        let query_lower = self.query.to_lowercase();
        let items: Vec<Arc<ItemModel>> = if query_lower.is_empty() {
            self._items.clone()
        } else {
            self._items
                .iter()
                .filter(|item| item.content.to_lowercase().contains(&query_lower))
                .cloned()
                .collect()
        };
        self.matched_items = vec![items];
    }

    pub fn update_items(&mut self, items: Vec<Arc<ItemModel>>) {
        self._items = items;
        self.matched_items = vec![self._items.clone()];
        if !self.matched_items.is_empty() && self.selected_index.is_none() {
            self.selected_index = Some(IndexPath::default());
        }
    }

    pub fn selected_item(&self) -> Option<Arc<ItemModel>> {
        let ix = self.selected_index?;
        self.matched_items
            .get(ix.section)
            .and_then(|c: &Vec<Arc<ItemModel>>| c.get(ix.row))
            .cloned()
    }

    // open_sheet_at_item: 点击任务，靠右显示任务详情
    fn open_sheet_at_item(
        &mut self,
        item: Arc<ItemModel>,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        window.open_sheet_at(Placement::Right, cx, move |this, _, _cx| {
            this.overlay(true)
                .overlay_closable(true)
                .size(px(400.))
                .title(item.content.clone())
                .gap_4()
                .child(Button::new("send-notification").child("Test Notification").on_click(
                    |_, window, cx| {
                        window.push_notification("Hello this is message from Drawer.", cx)
                    },
                ))
                .child(Label::new(item.description.clone().unwrap_or_default().to_string()))
                .footer(
                    h_flex()
                        .gap_6()
                        .items_center()
                        .child(Button::new("confirm").primary().label("确认").on_click(
                            |_, window, cx| {
                                window.close_sheet(cx);
                            },
                        ))
                        .child(Button::new("cancel").label("取消").on_click(|_, window, cx| {
                            window.close_sheet(cx);
                        })),
                )
        });
    }
}
impl ListDelegate for ItemListDelegate {
    type Item = ItemListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.prepare(query.to_owned());
        Task::ready(())
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        self.matched_items[section].len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(item) = self.matched_items[ix.section].get(ix.row) {
            let item: &Arc<ItemModel> = item;
            return Some(ItemListItem::new(ix, item.clone(), selected));
        }
        None
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        tracing::debug!("Confirmed with items: {}", secondary);
        window.dispatch_action(Box::new(SelectedItem), cx);
        let item_some = self.selected_item();
        if let Some(item) = item_some {
            self.open_sheet_at_item(item, window, cx);
        }
    }
}
