use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, Render, Styled,
    Subscription, WeakEntity, Window, px,
};
use gpui_component::{
    ActiveTheme, IndexPath, WindowExt,
    button::{Button, ButtonVariants},
    color_picker::ColorPickerState,
    date_picker::DatePickerState,
    input::InputState,
    list::{List, ListEvent, ListState},
    select::SelectState,
};
use todos::entity::ItemModel;

use crate::{
    DBState, ItemEvent, ItemInfo, ItemInfoEvent, ItemInfoState, ItemListDelegate, load_items,
};

impl EventEmitter<ItemEvent> for ItemsPanel {}
pub struct ItemsPanel {
    pub item_list: Entity<ListState<ItemListDelegate>>,
    item_due: Option<String>,
    is_loading: bool,
    pub active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
    name_input: Entity<InputState>,
    desc_input: Entity<InputState>,
    color_state: Entity<ColorPickerState>,
    // item_date: Entity<DatePickerState>,
    priority_select: Entity<SelectState<Vec<String>>>,
    is_checked: bool, // 任务完成状态
    item_info: Entity<ItemInfoState>,
}

impl ItemsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let color_state = cx.new(|cx| ColorPickerState::new(window, cx));

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("task title here..."));
        let desc_input = cx.new(|cx| {
            InputState::new(window, cx).auto_grow(5, 20).placeholder("task description here...")
        });
        let _date = cx.new(|cx| DatePickerState::new(window, cx));
        let item_info = cx.new(|cx| {
            let picker = ItemInfoState::new(window, cx);
            picker
        });
        let item_list =
            cx.new(|cx| ListState::new(ItemListDelegate::new(), window, cx).selectable(true));
        let priority_select = cx.new(|cx| {
            SelectState::new(
                vec![
                    "P1".to_string(),
                    "P2".to_string(),
                    "P3".to_string(),
                    "P4".to_string(),
                    "".to_string(),
                ],
                None,
                window,
                cx,
            )
        });
        let _subscriptions = vec![
            cx.subscribe(&item_info, |_this, _, event: &ItemInfoEvent, cx| match event {
                ItemInfoEvent::Update(item) => {
                    print!("iteminfo updated after:{:?}", item);
                    cx.emit(ItemEvent::Modified(item.clone()));
                    cx.notify();
                },
                ItemInfoEvent::Add(item) => {
                    cx.emit(ItemEvent::Added(item.clone()));
                    cx.notify();
                },
            }),
            cx.subscribe_in(&item_list, window, |this, _, ev: &ListEvent, _window, cx| {
                if let ListEvent::Confirm(ix) = ev
                    && let Some(_conn) = this.get_selected_item(*ix, cx)
                {
                    this.update_active_index(Some(ix.row));
                    // this.input_esc.update(cx, |is, cx| {
                    //     is.set_value(conn.clone().content.clone(), window, cx);
                    //     cx.notify();
                    // })
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
            println!("all items: {}", items.len());
            let _ = cx
                .update_entity(&item_list_clone, |list, cx| {
                    list.delegate_mut().update_items(rc_items);
                    cx.notify();
                })
                .ok();
        })
        .detach();
        Self {
            item_due: None,
            is_loading: false,
            is_checked: false,
            item_list,
            item_info,
            priority_select,
            active_index: Some(0),
            _subscriptions,
            name_input,
            desc_input,
            color_state,
        }
    }

    pub(crate) fn get_selected_item(&self, ix: IndexPath, cx: &App) -> Option<Rc<ItemModel>> {
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
            },
            ItemEvent::Modified(item) => self.mod_item(cx, item.clone()),
            ItemEvent::Deleted(item) => self.del_item(cx, item.clone()),
            ItemEvent::Finished(item) => self.finish_item(cx, item.clone()),
        }
    }

    #[allow(unused)]
    fn toggle_finished(&mut self, selectable: &bool, _: &mut Window, _cx: &mut Context<Self>) {
        self.is_checked = *selectable;
    }

    fn initialize_item_model(&self, _is_edit: bool, _: &mut Window, cx: &mut App) -> ItemModel {
        self.active_index
            .and_then(|index| {
                println!("show_label_dialog: active index: {}", index);
                self.get_selected_item(IndexPath::new(index), &cx)
            })
            .map(|label| {
                let item_ref = label.as_ref();
                ItemModel { ..item_ref.clone() }
            })
            .unwrap_or_default()
    }

    pub fn show_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>, is_edit: bool) {
        let item_info = self.item_info.clone();
        let ori_item = self.initialize_item_model(is_edit, window, cx);
        if is_edit {
            item_info.update(cx, |state, cx| {
                state.set_item(Rc::new(ori_item.clone()), window, cx);
                cx.notify();
            });
        } else {
        }
        let view = cx.entity().clone();
        let dialog_title = if is_edit { "Edit Item" } else { "New Item" };
        let button_text = if is_edit { "Save" } else { "Add" };

        window.open_dialog(cx, move |modal, _, _| {
            let item_info_clone = item_info.clone();
            let view_clone = view.clone();

            modal
                .title(dialog_title)
                .items_center()
                .w_full()
                .overlay(true)
                .keyboard(true)
                .overlay_closable(true)
                .child(ItemInfo::new(&item_info))
                .footer(move |_, _, _, _| {
                    vec![
                        Button::new("save").primary().label(button_text).on_click({
                            let view = view_clone.clone();
                            let item_info = item_info_clone.clone();
                            move |_, window, cx| {
                                window.close_dialog(cx);
                                item_info.update(cx, |item_info, cx| {
                                    cx.emit(ItemInfoEvent::Update(item_info.item.clone()));
                                    cx.notify();
                                });
                                view.update(cx, |_view, cx| {
                                    let item = item_info.read(cx).item.clone();
                                    print!("iteminfo dialog: {:?}", item.clone());
                                    let event = if is_edit {
                                        ItemEvent::Modified(item.clone())
                                    } else {
                                        ItemEvent::Added(item.clone())
                                    };
                                    cx.emit(event);
                                    cx.notify();
                                });
                            }
                        }),
                        Button::new("cancel").label("Cancel").on_click(move |_, window, cx| {
                            window.close_dialog(cx);
                        }),
                    ]
                })
        });
    }

    pub fn show_item_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.active_index {
            let item_some = self.get_selected_item(IndexPath::new(active_index), &cx);
            if let Some(item) = item_some {
                let view = cx.entity().clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .confirm()
                        .overlay(true)
                        .overlay_closable(true)
                        .child("Are you sure to delete the item?")
                        .on_ok({
                            let view = view.clone();
                            let item = item.clone();
                            move |_, window, cx| {
                                let view = view.clone();
                                let item = item.clone();
                                view.update(cx, |_view, cx| {
                                    cx.emit(ItemEvent::Deleted(item));
                                    cx.notify();
                                });
                                window.push_notification("You have delete ok.", cx);
                                true
                            }
                        })
                        .on_cancel(|_, window, cx| {
                            window.push_notification("You have canceled delete.", cx);
                            true
                        })
                });
            };
        }
    }

    pub fn show_finish_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.active_index {
            let item_some = self.get_selected_item(IndexPath::new(active_index), &cx);
            if let Some(item) = item_some {
                let view = cx.entity().clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .confirm()
                        .overlay(true)
                        .overlay_closable(true)
                        .child("Are you sure to finish the item?")
                        .on_ok({
                            let view = view.clone();
                            let item = item.clone();
                            move |_, window, cx| {
                                let view = view.clone();
                                let mut item = item.clone();
                                let item_mut = Rc::make_mut(&mut item);
                                item_mut.checked = true; //切换为完成状态
                                println!("item_mut: {:?}", item_mut);
                                println!("item before: {:?}", item);
                                view.update(cx, |_view, cx| {
                                    cx.emit(ItemEvent::Finished(item));
                                    cx.notify();
                                });
                                window.push_notification("You have finished item ok.", cx);
                                true
                            }
                        })
                        .on_cancel(|_, window, cx| {
                            window.push_notification("You have canceled.", cx);
                            true
                        })
                });
            };
        }
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

    pub fn finish_item(&mut self, cx: &mut Context<Self>, item: Rc<ItemModel>) {
        let item = item.clone();
        self.mod_item(cx, item);
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
            .p(px(2.))
            .flex_1()
            .w_full()
            .border_1()
            .gap_3()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
    }
}
