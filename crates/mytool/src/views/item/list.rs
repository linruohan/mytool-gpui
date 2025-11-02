use gpui::prelude::FluentBuilder;
use gpui::{
    App, Context, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Task,
    Window, actions, px,
};
use gpui_component::{
    ActiveTheme, IndexPath, Selectable, h_flex,
    label::Label,
    list::{List, ListDelegate, ListItem},
    v_flex,
};
use std::rc::Rc;
use todos::entity::ItemModel;
actions!(label, [SelectedItem]);

pub enum ItemEvent {
    Added(Rc<ItemModel>),
    Modified(Rc<ItemModel>),
    Deleted(Rc<ItemModel>),
}

#[derive(IntoElement)]
pub struct ItemListItem {
    base: ListItem,
    ix: IndexPath,
    item: Rc<ItemModel>,
    selected: bool,
}

impl ItemListItem {
    pub fn new(
        id: impl Into<ElementId>,
        item: Rc<ItemModel>,
        ix: IndexPath,
        selected: bool,
    ) -> Self {
        ItemListItem {
            item,
            ix,
            base: ListItem::new(id),
            selected,
        }
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
        self.base
            .px_2()
            .py_1()
            .overflow_x_hidden()
            .border_1()
            .when(self.selected, |this| {
                this.border_color(cx.theme().list_active_border)
            })
            .rounded(cx.theme().radius)
            .child(
                h_flex().items_center().justify_between().gap_2().child(
                    h_flex().gap_2().child(
                        v_flex()
                            .gap_1()
                            .max_w(px(500.))
                            .overflow_x_hidden()
                            .flex_nowrap()
                            .child(Label::new(self.item.content.clone()).whitespace_nowrap()),
                    ),
                ),
            )
    }
}

pub struct ItemListDelegate {
    pub(crate) _items: Vec<Rc<ItemModel>>,
    pub(crate) matched_items: Vec<Vec<Rc<ItemModel>>>,
    selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    query: SharedString,
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
        let items: Vec<Rc<ItemModel>> = self
            ._items
            .iter()
            .filter(|item| {
                item.content
                    .to_lowercase()
                    .contains(&self.query.to_lowercase())
                    | item
                        .description
                        .clone()
                        .unwrap()
                        .to_lowercase()
                        .contains(&self.query.to_lowercase())
            })
            .cloned()
            .collect();
        for item in items.into_iter() {
            self.matched_items.push(vec![item]);
        }
    }

    pub fn update_items(&mut self, items: Vec<Rc<ItemModel>>) {
        self._items = items;
        self.matched_items = vec![self._items.clone()];
        if !self.matched_items.is_empty() && self.selected_index.is_none() {
            self.selected_index = Some(IndexPath::default());
        }
    }
    pub fn add(&mut self, item: Rc<ItemModel>) {
        let mut items = self._items.clone();
        items.push(item.clone());
        self.update_items(items);
    }
    #[allow(unused)]
    fn selected_item(&self) -> Option<Rc<ItemModel>> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.matched_items
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }
}
impl ListDelegate for ItemListDelegate {
    type Item = ItemListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Task<()> {
        self.prepare(query.to_owned());
        Task::ready(())
    }
    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.matched_items.len()
    }

    fn render_item(
        &self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(company) = self.matched_items[ix.section].get(ix.row) {
            return Some(ItemListItem::new(ix, company.clone(), ix, selected));
        }

        None
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
        println!("Confirmed with secondary item: {}", secondary);
        window.dispatch_action(Box::new(SelectedItem), cx);
    }
}
