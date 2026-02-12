use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, InteractiveElement as _,
    MouseButton, ParentElement, Render, StatefulInteractiveElement as _, Styled, Window, div,
    prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme as _, IconName, IndexPath, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    scroll::ScrollableElement,
    v_flex,
};

use crate::{
    Board, BoardBase, ItemRow, ItemRowState, section,
    todo_actions::{add_item, delete_item, update_item},
    todo_state::{SectionState, TodayItemState},
};

pub enum ItemClickEvent {
    ShowModal,
    ConnectionError { field1: String },
}

impl EventEmitter<ItemClickEvent> for TodayBoard {}

pub struct TodayBoard {
    base: BoardBase,
}

impl TodayBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut base = BoardBase::new(window, cx);
        base.is_today_board = true;

        base._subscriptions = vec![
            cx.observe_global_in::<TodayItemState>(window, move |this, window, cx| {
                let state_items = cx.global::<TodayItemState>().items.clone();
                this.base.item_rows = state_items
                    .iter()
                    .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
                    .collect();

                this.base.update_items(&state_items);
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
    ) -> Option<std::sync::Arc<todos::entity::ItemModel>> {
        let item_list = cx.global::<TodayItemState>().items.clone();
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
                state.set_item(std::sync::Arc::new(ori_item.clone()), window, cx);
                cx.notify();
            });
            self.base.item_info.clone()
        };

        let config = crate::components::EditDialogConfig::new(
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

impl Board for TodayBoard {
    fn icon() -> IconName {
        IconName::StarOutlineThickSymbolic
    }

    fn colors() -> Vec<gpui::Hsla> {
        vec![gpui::rgb(0x33d17a).into(), gpui::rgb(0x33d17a).into()]
    }

    fn count(cx: &mut gpui::App) -> usize {
        cx.global::<TodayItemState>().items.len()
    }

    fn title() -> &'static str {
        "Today"
    }

    fn description() -> &'static str {
        "今天需要完成的任务"
    }

    fn zoomable() -> Option<gpui_component::dock::PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut gpui::App) -> Entity<impl gpui::Render> {
        Self::view(window, cx)
    }
}

impl Focusable for TodayBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.base.focus_handle.clone()
    }
}

impl Render for TodayBoard {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let view = cx.entity().clone();
        let sections = cx.global::<SectionState>().sections.clone();
        let pinned_items = self.base.pinned_items.clone();
        let overdue_items = self.base.overdue_items.clone();
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
                                h_flex()
                                    .gap_2()
                                    .child(<TodayBoard as Board>::icon())
                                    .child(div().text_base().child(<TodayBoard as Board>::title())),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<TodayBoard as Board>::description()),
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
                                Button::new("add-label")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::PlusLargeSymbolic)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_item_dialog(window, cx, false, None);
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
                                                this.show_item_dialog(window, cx, true, None);
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
                        .when(!pinned_items.is_empty(), |this| {
                            let view_clone = view.clone();
                            this.child(section("Pinned").child(v_flex().gap_2().w_full().children(
                                pinned_items.into_iter().map(|(i, _item)| {
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
                            )))
                        })
                        .when(!overdue_items.is_empty(), |this| {
                            let view_clone = view.clone();
                            this.child(section("Overdue").child(
                                v_flex().gap_2().w_full().children(overdue_items.into_iter().map(
                                    |(i, _item)| {
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
                                    },
                                )),
                            ))
                        })
                        .when(!no_section_items.is_empty(), |this| {
                            let view_clone = view.clone();
                            this.child(
                                section("No Section")
                                    .sub_title(
                                        h_flex().gap_1().child(
                                            Button::new("add-item-to-no-section")
                                                .small()
                                                .ghost()
                                                .compact()
                                                .icon(IconName::PlusLargeSymbolic)
                                                .label("Add Task")
                                                .on_click({
                                                    let view = view_clone.clone();
                                                    move |_, window, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.show_item_dialog(
                                                                window, cx, false, None,
                                                            );
                                                            cx.notify();
                                                        })
                                                    }
                                                }),
                                        ),
                                    )
                                    .child(v_flex().gap_2().w_full().children(
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
                                    )),
                            )
                        })
                        .children(sections.iter().filter_map(|sec| {
                            let items = section_items_map.get(&sec.id)?;
                            if items.is_empty() {
                                return None;
                            }

                            let view_clone = view.clone();
                            let section_id = sec.id.clone();

                            Some(
                                section(sec.name.clone())
                                    .sub_title(
                                        h_flex().gap_1().child(
                                            Button::new(format!(
                                                "add-item-to-section-{}",
                                                section_id
                                            ))
                                            .small()
                                            .ghost()
                                            .compact()
                                            .icon(IconName::PlusLargeSymbolic)
                                            .label("Add Task")
                                            .on_click({
                                                let view = view_clone.clone();
                                                let section_id = section_id.clone();
                                                move |_, window, cx| {
                                                    view.update(cx, |this, cx| {
                                                        this.show_item_dialog(
                                                            window,
                                                            cx,
                                                            false,
                                                            Some(section_id.clone()),
                                                        );
                                                        cx.notify();
                                                    })
                                                }
                                            }),
                                        ),
                                    )
                                    .child(v_flex().gap_2().w_full().children(items.iter().map(
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
                                    ))),
                            )
                        })),
                ),
            )
    }
}
