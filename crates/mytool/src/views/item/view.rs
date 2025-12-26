use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, px,
};
use gpui_component::{
    ActiveTheme, IndexPath, WindowExt,
    button::{Button, ButtonVariants},
    list::{List, ListEvent, ListState},
};
use todos::entity::ItemModel;

use crate::{
    ItemEvent, ItemInfo, ItemInfoEvent, ItemInfoState, ItemListDelegate,
    todo_actions::{add_item, delete_item, update_item},
    todo_state::ItemState,
};

impl EventEmitter<ItemEvent> for ItemsPanel {}
pub struct ItemsPanel {
    pub item_list: Entity<ListState<ItemListDelegate>>,
    pub active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
    is_checked: bool, // 任务完成状态
    item_info: Entity<ItemInfoState>,
}

impl ItemsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item_info = cx.new(|cx| {
            let picker = ItemInfoState::new(window, cx);
            picker
        });
        let item_list =
            cx.new(|cx| ListState::new(ItemListDelegate::new(), window, cx).selectable(true));
        let item_list_clone = item_list.clone();
        let _subscriptions = vec![
            cx.observe_global::<ItemState>(move |_this, cx| {
                let items = cx.global::<ItemState>().items.clone();
                let _ = cx.update_entity(&item_list_clone, |list, cx| {
                    list.delegate_mut().update_items(items);
                    cx.notify();
                });
                cx.notify();
            }),
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

        Self { is_checked: false, item_list, item_info, active_index: Some(0), _subscriptions }
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
        println!("handle_item_event:");
        match event {
            ItemEvent::Added(item) => add_item(item.clone(), cx),
            ItemEvent::Modified(item) => update_item(item.clone(), cx),
            ItemEvent::Deleted(item) => delete_item(item.clone(), cx),
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

    pub fn finish_item(&mut self, cx: &mut Context<Self>, item: Rc<ItemModel>) {
        let mut binding = item.clone();
        let item = Rc::make_mut(&mut binding);
        item.checked = true;
        update_item(binding, cx);
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
