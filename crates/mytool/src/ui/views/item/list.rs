use std::{collections::HashMap, sync::Arc};

use gpui::{
    App, Context, ElementId, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Task, Window, actions, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Colorize, Icon, IconName, IndexPath, Placement, Selectable, WindowExt,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    label::Label,
    list::{ListDelegate, ListItem, ListState},
    red_400, v_flex,
};
use todos::{
    entity::{ItemModel, LabelModel},
    utils::datetime::DateTime,
};

use crate::todo_state::TodoStore;

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
        // info!("ItemListItem rendering - id: {}, labels: {:?}", self.item.id, self.item.labels);

        let text_color =
            if self.selected { cx.theme().accent_foreground } else { cx.theme().foreground };

        let labels = cx.global::<TodoStore>().labels.clone();
        let label_map: HashMap<&str, &Arc<LabelModel>> =
            labels.iter().map(|l| (l.id.as_str(), l)).collect();

        // 从 ItemModel 的 labels 字段获取标签 ID
        let item_labels: Vec<String> = self
            .item
            .labels
            .as_ref()
            .map(|labels_str| {
                labels_str.split(';').filter(|id| !id.is_empty()).map(|id| id.to_string()).collect()
            })
            .unwrap_or_default();

        // info!("ItemListItem parsed labels - id: {}, parsed: {:?}", self.item.id, item_labels);

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
                        Label::new(
                            // 使用类型安全的 due_date() 方法
                            self.item
                                .due_date()
                                .and_then(|due_date| due_date.datetime())
                                .map(|datetime| {
                                    DateTime::default().get_relative_date_from_date(&datetime)
                                })
                                .unwrap_or_else(|| "No date".to_string()),
                        )
                        .when(self.item.checked, |this| this.line_through().text_color(red_400())),
                    )
                    .child(
                        v_flex().overflow_x_hidden().flex_nowrap().child(
                            Label::new(self.item.content.clone())
                                .whitespace_nowrap()
                                .when(self.item.checked, |this| this.line_through()),
                        ),
                    )
                    // 显示标签
                    .when(!item_labels.is_empty(), |this| {
                        this.child(
                            h_flex().gap_1().flex().children(
                                item_labels
                                    .iter()
                                    .flat_map(|group| group.split(';').filter(|id| !id.is_empty()))
                                    .filter_map(|id| {
                                        label_map.get(id).map(|label| {
                                            h_flex()
                                                .rounded(px(10.0))
                                                .bg(Hsla::from(gpui::rgb(
                                                    u32::from_str_radix(&label.color[1..], 16)
                                                        .ok()
                                                        .unwrap_or_default(),
                                                ))
                                                .lighten(0.3))
                                                .child(
                                                    Icon::build(IconName::TagOutlineSymbolic)
                                                        .text_color(Hsla::from(gpui::rgb(
                                                            u32::from_str_radix(
                                                                &label.color[1..],
                                                                16,
                                                            )
                                                            .ok()
                                                            .unwrap_or_default(),
                                                        ))),
                                                )
                                                .child(label.name.clone())
                                        })
                                    })
                                    .collect::<Vec<_>>(),
                            ),
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

    #[allow(unused)]
    fn get_label_by_id(&mut self, id: &str, _window: &mut Window, cx: &mut App) -> Option<String> {
        let labels = cx.global::<TodoStore>().labels.clone();
        labels.iter().find(|label| label.id == id).cloned().map(|label| label.name.clone())
    }

    fn prepare(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        let items: Vec<Arc<ItemModel>> = self
            ._items
            .iter()
            .filter(|item| item.content.to_lowercase().contains(&self.query.to_lowercase()))
            .cloned()
            .collect();
        for item in items.into_iter() {
            self.matched_items.push(vec![item]);
        }
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
        println!("Confirmed with items: {}", secondary);
        window.dispatch_action(Box::new(SelectedItem), cx);
        let item_some = self.selected_item();
        if let Some(item) = item_some {
            self.open_sheet_at_item(item, window, cx);
        }
    }
}
