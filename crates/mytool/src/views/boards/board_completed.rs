use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement as _,
    MouseButton, ParentElement, Render, StatefulInteractiveElement as _, Styled, Window, div,
    prelude::FluentBuilder,
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
    Board, BoardBase, ItemRow, ItemRowState, section,
    todo_actions::{add_item, delete_item, update_item},
    todo_state::{CompleteItemState, SectionState},
};

pub enum ItemClickEvent {
    ShowModal,
    ConnectionError { field1: String },
}

impl EventEmitter<ItemClickEvent> for CompletedBoard {}

pub struct CompletedBoard {
    base: BoardBase,
}

impl CompletedBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut base = BoardBase::new(window, cx);

        base._subscriptions = vec![
            cx.observe_global_in::<CompleteItemState>(window, move |this, window, cx| {
                let state_items = cx.global::<CompleteItemState>().items.clone();
                this.base.item_rows = state_items
                    .iter()
                    .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
                    .collect();

                this.base.update_items(&state_items);

                if let Some(ix) = this.base.active_index {
                    if ix >= this.base.item_rows.len() {
                        this.base.active_index =
                            if this.base.item_rows.is_empty() { None } else { Some(0) };
                    }
                } else if !this.base.item_rows.is_empty() {
                    this.base.active_index = Some(0);
                }
                cx.notify();
            }),
            cx.observe_global_in::<SectionState>(window, move |_, _, cx| {
                cx.notify();
            }),
        ];

        Self { base }
    }

    pub(crate) fn get_selected_item(
        &self,
        ix: IndexPath,
        cx: &App,
    ) -> Option<std::rc::Rc<todos::entity::ItemModel>> {
        let item_list = cx.global::<CompleteItemState>().items.clone();
        item_list.get(ix.row).cloned()
    }

    pub fn show_item_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        is_edit: bool,
        section_id: Option<String>,
    ) {
        let item_info = if is_edit {
            if let Some(active_index) = self.base.active_index {
                if let Some(item_row) = self.base.item_rows.get(active_index) {
                    item_row.read(cx).item_info.clone()
                } else {
                    self.base.item_info.clone()
                }
            } else {
                self.base.item_info.clone()
            }
        } else {
            let mut ori_item = todos::entity::ItemModel::default();

            // If adding a new item with a section_id, set it
            if let Some(sid) = section_id {
                ori_item.section_id = Some(sid);
            }

            self.base.item_info.update(cx, |state, cx| {
                state.set_item(std::rc::Rc::new(ori_item.clone()), window, cx);
                cx.notify();
            });
            self.base.item_info.clone()
        };

        let config = crate::components::ItemDialogConfig::new(
            if is_edit { "Edit Item" } else { "New Item" },
            if is_edit { "Save" } else { "Add" },
            is_edit,
        );

        crate::components::show_item_dialog(window, cx, item_info, config, move |item, cx| {
            if is_edit {
                update_item(item, cx);
            } else {
                add_item(item, cx);
            }
        });
    }

    pub fn show_item_unfinish_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.base.active_index {
            let item_some = self.get_selected_item(IndexPath::new(active_index), cx);
            if let Some(item) = item_some {
                let view = cx.entity().clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .confirm()
                        .overlay(true)
                        .overlay_closable(true)
                        .child("Are you sure to mark this item as unfinished?")
                        .on_ok({
                            let view = view.clone();
                            let item = item.clone();
                            move |_, window, cx| {
                                let _view = view.clone();
                                let mut item = item.clone();
                                let item_mut = std::rc::Rc::make_mut(&mut item);
                                item_mut.checked = false;
                                update_item(item, cx);
                                window.push_notification("Item marked as unfinished.", cx);
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

    pub fn show_item_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.base.active_index {
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
}

impl Board for CompletedBoard {
    fn icon() -> IconName {
        IconName::CheckRoundOutlineSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xffbe6f).into(), gpui::rgb(0xff7800).into()]
    }

    fn count(cx: &mut App) -> usize {
        cx.global::<CompleteItemState>().items.len()
    }

    fn title() -> &'static str {
        "Completed"
    }

    fn description() -> &'static str {
        "已完成任务"
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for CompletedBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.base.focus_handle.clone()
    }
}

impl Render for CompletedBoard {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let view = cx.entity().clone();
        let sections = cx.global::<SectionState>().sections.clone();
        let no_section_items = self.base.no_section_items.clone();
        let section_items_map = self.base.section_items_map.clone();

        v_flex()
            .track_focus(&self.base.focus_handle)
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
                                h_flex().gap_2().child(<CompletedBoard as Board>::icon()).child(
                                    div().text_base().child(<CompletedBoard as Board>::title()),
                                ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<CompletedBoard as Board>::description()),
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
                                Button::new("unfinish-item")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::Undo)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_item_unfinish_dialog(window, cx);
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
            .child(
                v_flex().flex_1().overflow_y_scrollbar().child(
                    v_flex()
                        .gap_4()
                        .when(!no_section_items.is_empty(), |this| {
                            let view_clone = view.clone();
                            this.child(section("No Section").child(
                                v_flex().gap_2().w_full().children(
                                    no_section_items.into_iter().map(|(i, _item)| {
                                        let view = view_clone.clone();
                                        let is_active = self.base.active_index == Some(i);
                                        let item_row = self.base.item_rows.get(i).cloned();
                                        div()
                                            .id(("item", i))
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.base.active_index = Some(i);
                                                    cx.notify();
                                                });
                                            })
                                            .when(is_active, |this| {
                                                this.border_color(cx.theme().list_active_border)
                                            })
                                            .children(item_row.map(|row| ItemRow::new(&row)))
                                    }),
                                ),
                            ))
                        })
                        .children(sections.iter().filter_map(|sec| {
                            let items = section_items_map.get(&sec.id)?;
                            if items.is_empty() {
                                return None;
                            }

                            let view_clone = view.clone();

                            Some(section(sec.name.clone()).child(
                                v_flex().gap_2().w_full().children(items.iter().map(
                                    |(i, _item)| {
                                        let view = view_clone.clone();
                                        let i = *i;
                                        let is_active = self.base.active_index == Some(i);
                                        let item_row = self.base.item_rows.get(i).cloned();
                                        div()
                                            .id(("item", i))
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.base.active_index = Some(i);
                                                    cx.notify();
                                                });
                                            })
                                            .when(is_active, |this| {
                                                this.border_color(cx.theme().list_active_border)
                                            })
                                            .children(item_row.map(|row| ItemRow::new(&row)))
                                    },
                                )),
                            ))
                        })),
                ),
            )
    }
}
