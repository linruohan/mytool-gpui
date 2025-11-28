use std::rc::Rc;

use gpui::{
    App, Context, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, Styled, Task, Window, actions, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IconName, IndexPath, Placement, Selectable, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    label::Label,
    list::{ListDelegate, ListItem, ListState},
    red_400,
    tag::Tag,
    v_flex,
};
use todos::entity::ItemModel;

actions!(item, [SelectedItem]);
pub enum ItemEvent {
    Finished(Rc<ItemModel>),
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
        ItemListItem { item, ix, base: ListItem::new(id), selected }
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
        let text_color =
            if self.selected { cx.theme().accent_foreground } else { cx.theme().foreground };

        let _bg_color = if self.selected {
            cx.theme().list_active
        } else if self.ix.row.is_multiple_of(2) {
            cx.theme().list
        } else {
            cx.theme().list_even
        };

        self.base
            .px_2()
            .py_1()
            .overflow_x_hidden()
            .border_1()
            .rounded(cx.theme().radius)
            .when(self.selected, |this| this.border_color(cx.theme().list_active_border))
            .rounded(cx.theme().radius)
            .child(
                h_flex()
                    .items_center()
                    .justify_start()
                    .gap_2()
                    .text_color(text_color)
                    .child(Checkbox::new("item-finished").checked(self.item.checked))
                    .child(
                        Label::new("Tomorrow").when(self.item.checked, |this| {
                            this.line_through().text_color(red_400())
                        }),
                    )
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
                                    h_flex().gap_2().flex().px_2().children(
                                        self.item
                                            .labels
                                            .iter()
                                            .flat_map(|labels| labels.as_str().unwrap().split(';'))
                                            .filter(|label| !label.is_empty())
                                            .map(|label| Tag::primary().child(label.to_string()))
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
                                        let item = self.item.clone();
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
                    ),
            )
    }
}

pub struct ItemListDelegate {
    pub _items: Vec<Rc<ItemModel>>,
    pub matched_items: Vec<Vec<Rc<ItemModel>>>,
    // label_list: Entity<ListState<LabelListDelegate>>,
    selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    query: SharedString,
}

impl ItemListDelegate {
    pub fn new() -> Self {
        // let label_list =
        //     cx.new(|cx| ListState::new(LabelListDelegate::new(), window, cx).selectable(true));
        // let label_list_clone = label_list.clone();
        // let db = cx.global::<DBState>().conn.clone();
        // cx.spawn(async move |_view, cx| {
        //     let db = db.lock().await;
        //     let labels = load_labels(db.clone()).await;
        //     let rc_labels: Vec<Rc<LabelModel>> =
        //         labels.iter().map(|pro| Rc::new(pro.clone())).collect();
        //     println!("item list: len labels: {}", labels.len());
        //     let _ = cx
        //         .update_entity(&label_list_clone, |list, cx| {
        //             list.delegate_mut().update_labels(rc_labels);
        //             cx.notify();
        //         })
        //         .ok();
        // })
        // .detach();
        Self {
            _items: vec![],
            matched_items: vec![],
            selected_index: None,
            confirmed_index: None,
            query: "".into(),
            // label_list,
        }
    }

    fn get_label_by_id(
        &mut self,
        _id: &str,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<String> {
        // let labels = self.label_list.read(cx).delegate()._labels.clone();
        // if let Some(label) = labels.iter().find(|label| label.id == id).cloned() {
        //     Some(label.name.clone())
        // } else {
        //     None
        // }
        None
    }

    fn prepare(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        let items: Vec<Rc<ItemModel>> = self
            ._items
            .iter()
            .filter(|item| item.content.to_lowercase().contains(&self.query.to_lowercase()))
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

    pub fn selected_item(&self) -> Option<Rc<ItemModel>> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.matched_items.get(ix.section).and_then(|c| c.get(ix.row)).cloned()
    }

    // open_sheet_at_item: 点击任务，靠右显示任务详情
    fn open_sheet_at_item(
        &mut self,
        item: Rc<ItemModel>,
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
                .child(Label::new(item.content.clone()))
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

    fn render_item(&self, ix: IndexPath, _: &mut Window, _: &mut App) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(item) = self.matched_items[ix.section].get(ix.row) {
            return Some(ItemListItem::new(ix, item.clone(), ix, selected));
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
        println!("Confirmed with items: {}", secondary);
        window.dispatch_action(Box::new(SelectedItem), cx);
        let item_some = self.selected_item();
        if let Some(item) = item_some {
            self.open_sheet_at_item(item, window, cx);
        }
    }
}
