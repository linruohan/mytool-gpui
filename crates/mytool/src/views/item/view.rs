use crate::{DBState, ItemEvent, ItemListDelegate, load_items};
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, Render, Styled,
    Subscription, WeakEntity, Window, px,
};
use gpui_component::{
    ActiveTheme, IndexPath, WindowExt,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerState},
    input::{Input, InputState},
    list::{List, ListEvent, ListState},
    v_flex,
};
use std::rc::Rc;
use todos::entity::ItemModel;

impl EventEmitter<ItemEvent> for ItemsPanel {}
pub struct ItemsPanel {
    input_esc: Entity<InputState>,
    pub item_list: Entity<ListState<ItemListDelegate>>,
    item_due: Option<String>,
    is_loading: bool,
    pub active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl ItemsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_esc = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter DB URL")
                .clean_on_escape()
        });

        let item_list =
            cx.new(|cx| ListState::new(ItemListDelegate::new(), window, cx).searchable(true));

        let _subscriptions =
            vec![
                cx.subscribe_in(&item_list, window, |this, _, ev: &ListEvent, window, cx| {
                    if let ListEvent::Confirm(ix) = ev
                        && let Some(conn) = this.get_selected_item(*ix, cx)
                    {
                        this.input_esc.update(cx, |is, cx| {
                            is.set_value(conn.clone().content.clone(), window, cx);
                            cx.notify();
                        })
                    }
                }),
            ];

        let item_list_clone = item_list.clone();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |_view, cx| {
            let db = db.lock().await;
            let items = load_items(db.clone()).await;
            let rc_items: Vec<Rc<ItemModel>> =
                items.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("len items: {}", items.len());
            let _ = cx
                .update_entity(&item_list_clone, |list, cx| {
                    list.delegate_mut().update_items(rc_items);
                    cx.notify();
                })
                .ok();
        })
        .detach();
        Self {
            input_esc,
            item_due: None,
            is_loading: false,
            item_list,
            active_index: Some(0),
            _subscriptions,
        }
    }
    fn get_selected_item(&self, ix: IndexPath, cx: &App) -> Option<Rc<ItemModel>> {
        self.item_list
            .read(cx)
            .delegate()
            .matched_items
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }
    pub fn update_active_index(&mut self, value: Option<usize>) {
        self.active_index = value;
    }
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
    pub fn handle_item_event(&mut self, event: &ItemEvent, cx: &mut Context<Self>) {
        match event {
            ItemEvent::Added(item) => {
                println!("handle_item_event:");
                self.add_item(cx, item.clone())
            }
            ItemEvent::Modified(item) => self.mod_item(cx, item.clone()),
            ItemEvent::Deleted(item) => self.del_item(cx, item.clone()),
        }
    }
    pub fn add_item_model(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Item Content"));
        let _input2 = cx.new(|cx| -> InputState {
            InputState::new(window, cx).placeholder("For test focus back on modal close.")
        });
        let now = chrono::Local::now().naive_local().date();
        let item_due = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
            picker.set_date(now, window, cx);
            picker
        });

        let view = cx.entity().clone();

        window.open_modal(cx, move |modal, _, _| {
            modal
                .title("Add Item")
                .overlay(false)
                .keyboard(true)
                .show_close(true)
                .overlay_closable(true)
                .child(
                    v_flex()
                        .gap_3()
                        .child(Input::new(&input1))
                        .child(DatePicker::new(&item_due).placeholder("DueDate of Item")),
                )
                .footer({
                    let view = view.clone();
                    let input1 = input1.clone();
                    move |_, _, _, _cx| {
                        vec![
                            Button::new("add").primary().label("Add").on_click({
                                let view = view.clone();
                                let input1 = input1.clone();
                                move |_, window, cx| {
                                    window.close_modal(cx);
                                    view.update(cx, |_view, cx| {
                                        let item = ItemModel {
                                            content: input1.read(cx).value().to_string(),
                                            // due: Some(view.item_due.clone()),
                                            ..Default::default()
                                        };
                                        cx.emit(ItemEvent::Added(item.into()));
                                        cx.notify();
                                    });
                                }
                            }),
                            Button::new("cancel")
                                .label("Cancel")
                                .on_click(move |_, window, cx| {
                                    window.close_modal(cx);
                                }),
                        ]
                    }
                })
        });
    }
    // 更新items
    fn get_items(&mut self, cx: &mut Context<Self>) {
        if !self.is_loading {
            return;
        }
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this, cx| {
            let db = db.lock().await;
            let items = load_items(db.clone()).await;
            let rc_items: Vec<Rc<ItemModel>> =
                items.iter().map(|pro| Rc::new(pro.clone())).collect();

            this.update(cx, |this, cx| {
                this.item_list.update(cx, |list, cx| {
                    list.delegate_mut().update_items(rc_items);
                    cx.notify();
                });

                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn add_item(&mut self, cx: &mut Context<Self>, item: Rc<ItemModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ItemsPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::add_item(item.clone(), db.clone()).await;
            println!("add_item {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.get_items(cx);
    }
    pub fn mod_item(&mut self, cx: &mut Context<Self>, item: Rc<ItemModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ItemsPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::mod_item(item.clone(), db.clone()).await;
            println!("mod_item {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.get_items(cx);
    }
    pub fn del_item(&mut self, cx: &mut Context<Self>, item: Rc<ItemModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ItemsPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::del_item(item.clone(), db.clone()).await;
            println!("mod_item {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.get_items(cx);
    }
}

impl Render for ItemsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        List::new(&self.item_list)
            .p(px(8.))
            .flex_1()
            .w_full()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
    }
}
