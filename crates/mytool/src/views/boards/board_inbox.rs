use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    InteractiveElement as _, MouseButton, ParentElement, Render, StatefulInteractiveElement as _,
    Styled, Subscription, Window, div, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme as _, IconName, IndexPath, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    dock::PanelControl,
    h_flex,
    scroll::ScrollableElement,
    v_flex,
};

use crate::{
    Board, ItemInfo, ItemInfoEvent, ItemInfoState, ItemRow, ItemRowState,
    todo_actions::{add_item, delete_item, update_item},
    todo_state::{InboxItemState, ItemState},
};

pub enum ItemClickEvent {
    ShowModal,
    ConnectionError { field1: String },
}

impl EventEmitter<ItemClickEvent> for InboxBoard {}

pub struct InboxBoard {
    _subscriptions: Vec<Subscription>,
    focus_handle: FocusHandle,
    pub active_index: Option<usize>,
    item_rows: Vec<Entity<ItemRowState>>,
    item_info: Entity<ItemInfoState>,
}

impl InboxBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = std::rc::Rc::new(todos::entity::ItemModel::default());
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
        let item_rows = vec![];

        let _subscriptions = vec![
            cx.observe_global_in::<ItemState>(window, move |this, window, cx| {
                let state_items = cx.global::<ItemState>().items.clone();
                this.item_rows = state_items
                    .iter()
                    .filter(|item| !item.checked)
                    .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
                    .collect();

                if let Some(ix) = this.active_index {
                    if ix >= this.item_rows.len() {
                        this.active_index = if this.item_rows.is_empty() { None } else { Some(0) };
                    }
                } else if !this.item_rows.is_empty() {
                    this.active_index = Some(0);
                }
                cx.notify();
            }),
            cx.subscribe(&item_info, |this, _, event: &ItemInfoEvent, cx| {
                this.item_info.update(cx, |state, cx| {
                    state.handle_item_info_event(event, cx);
                });
            }),
        ];
        Self {
            focus_handle: cx.focus_handle(),
            _subscriptions,
            active_index: Some(0),
            item_rows,
            item_info,
        }
    }

    pub(crate) fn get_selected_item(
        &self,
        ix: IndexPath,
        cx: &App,
    ) -> Option<std::rc::Rc<todos::entity::ItemModel>> {
        let item_list = cx.global::<InboxItemState>().items.clone();
        item_list.get(ix.row).cloned()
    }

    fn initialize_item_model(
        &self,
        _is_edit: bool,
        _: &mut Window,
        cx: &mut App,
    ) -> todos::entity::ItemModel {
        self.active_index
            .and_then(|index| {
                println!("show_label_dialog: active index: {}", index);
                self.get_selected_item(IndexPath::new(index), cx)
            })
            .map(|label| {
                let item_ref = label.as_ref();
                todos::entity::ItemModel { ..item_ref.clone() }
            })
            .unwrap_or_default()
    }

    pub fn show_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>, is_edit: bool) {
        let item_info = self.item_info.clone();
        let ori_item = self.initialize_item_model(is_edit, window, cx);
        if is_edit {
            item_info.update(cx, |state, cx| {
                state.set_item(std::rc::Rc::new(ori_item.clone()), window, cx);
                cx.notify();
            });
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
                                    println!(
                                        "show_iteminfo_dialog: before save: item info: {:?}",
                                        item_info.item.clone()
                                    );
                                    cx.emit(ItemInfoEvent::Updated());
                                    cx.notify();
                                });
                                view.update(cx, |_view, cx| {
                                    let item = item_info.read(cx).item.clone();
                                    print!("iteminfo dialog: {:?}", item.clone());
                                    if is_edit {
                                        update_item(item.clone(), cx);
                                    } else {
                                        add_item(item.clone(), cx);
                                    };
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
            let item_some = self.get_selected_item(IndexPath::new(active_index), cx);
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
                                let _view = view.clone();
                                delete_item(item.clone(), cx);
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
            let item_some = self.get_selected_item(IndexPath::new(active_index), cx);
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
                                let _view = view.clone();
                                let mut item = item.clone();
                                let item_mut = std::rc::Rc::make_mut(&mut item);
                                item_mut.checked = true; //切换为完成状态
                                update_item(item, cx);
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
}
impl Board for InboxBoard {
    fn icon() -> IconName {
        IconName::MailboxSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0x99c1f1).into(), gpui::rgb(0x3584e4).into()]
    }

    fn count(cx: &mut App) -> usize {
        cx.global::<ItemState>().items.iter().filter(|i| !i.checked).count()
    }

    fn title() -> &'static str {
        "Inbox"
    }

    fn description() -> &'static str {
        "所有未完成任务"
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for InboxBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InboxBoard {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let view = cx.entity().clone();
        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .gap_4()
            .child(
                h_flex()
                    .id("header")
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .justify_between()
                    .items_start()
                    .child(
                        v_flex()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(<InboxBoard as Board>::icon())
                                    .child(div().text_base().child(<InboxBoard as Board>::title())),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<InboxBoard as Board>::description()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .px_2()
                            .gap_2()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                Button::new("finish-label")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::CheckmarkSmallSymbolic)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_finish_item_dialog(window, cx);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("add-label")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::PlusLargeSymbolic)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_item_dialog(window, cx, false);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("edit-item")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::EditSymbolic)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_item_dialog(window, cx, true);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("delete-item")
                                    .icon(IconName::UserTrashSymbolic)
                                    .small()
                                    .ghost()
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_item_delete_dialog(window, cx);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            ),
                    ),
            )
            .child(v_flex().flex_1().overflow_y_scrollbar().child(v_flex().gap_2().children(
                self.item_rows.iter().enumerate().map(|(i, item)| {
                    let view = view.clone();
                    let is_active = self.active_index == Some(i);
                    div()
                        .id(("item", i))
                        .on_click(move |_, _, cx| {
                            view.update(cx, |this, cx| {
                                this.active_index = Some(i);
                                cx.notify();
                            });
                        })
                        .when(is_active, |this: gpui::Stateful<gpui::Div>| {
                            this.border_color(cx.theme().list_active_border)
                        })
                        .child(ItemRow::new(item))
                }),
            )))
    }
}
