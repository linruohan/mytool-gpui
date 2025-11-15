use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, Styled, Window, px,
};
use gpui_component::{
    button::Button,
    h_flex,
    list::{List, ListDelegate, ListItem, ListState},
    popover::Popover,
    v_flex,
};
use todos::enums::item_priority::ItemPriority;

struct DropdownListDelegate {
    parent: Entity<PopoverList>,
    confirmed_index: Option<usize>,
    selected_index: Option<usize>,
    items: Vec<Arc<i32>>,
    matches: Vec<Arc<i32>>,
}
impl ListDelegate for DropdownListDelegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        self.matches.len()
    }

    fn render_item(
        &self,
        ix: gpui_component::IndexPath,
        _: &mut Window,
        _: &mut App,
    ) -> Option<Self::Item> {
        let confirmed = Some(ix.row) == self.confirmed_index;
        if let Some(item) = self.matches.get(ix.row) {
            let list_item = ListItem::new(("item", ix.row)).confirmed(confirmed).child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(ItemPriority::from_i32(**item).display_name()),
            );
            Some(list_item)
        } else {
            None
        }
    }

    fn set_selected_index(
        &mut self,
        ix: Option<gpui_component::IndexPath>,
        _: &mut Window,
        cx: &mut Context<gpui_component::list::ListState<Self>>,
    ) {
        self.selected_index = ix.map(|ix| ix.row);

        if let Some(_) = ix {
            cx.notify();
        }
    }

    fn confirm(&mut self, _: bool, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;
            self.confirmed_index = self.selected_index;
            if let Some(ix) = self.confirmed_index {
                if let Some(item) = self.matches.get(ix) {
                    this.priority = ItemPriority::from_i32(**item);
                }
            }
            cx.notify();
        })
    }

    fn cancel(&mut self, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;

            cx.notify();
        })
    }
}
pub struct PopoverList {
    focus_handle: FocusHandle,
    list: Entity<ListState<DropdownListDelegate>>,
    list_popover_open: bool,
    priority: ItemPriority,
}

impl PopoverList {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items: Vec<Arc<i32>> =
            ItemPriority::all().iter().map(|item| Arc::new(item.clone() as i32)).collect();
        let parent = cx.entity();
        let delegate = DropdownListDelegate {
            parent,
            selected_index: None,
            confirmed_index: None,
            items: items.clone(),
            matches: items.clone(),
        };

        let list = cx.new(|cx| ListState::new(delegate, window, cx).searchable(true));

        cx.focus_self(window);

        Self {
            list,
            list_popover_open: false,
            priority: ItemPriority::NONE,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for PopoverList {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
const CONTEXT: &str = "popover-list";
impl RenderOnce for PopoverList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex().key_context(CONTEXT).track_focus(&self.focus_handle).size_full().gap_6().child(
            Popover::new("popover-list")
                .p_0()
                .text_sm()
                .open(self.list_popover_open)
                .trigger(Button::new("pop").outline().label("Popup List"))
                .track_focus(&self.list.focus_handle(cx))
                .child(List::new(&self.list))
                .w_64()
                .h(px(200.)),
        )
    }
}
