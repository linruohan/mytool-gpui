use std::rc::Rc;

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
    input::InputState,
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};
use sea_orm::sqlx::types::uuid;

use crate::{
    Board, ItemInfoState, ItemRow, ItemRowState, section,
    todo_actions::{
        add_item, add_section, delete_item, delete_section, update_item, update_section,
    },
    todo_state::{InboxItemState, ItemState, ProjectState, SectionState},
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
    no_section_items: Vec<(usize, Rc<todos::entity::ItemModel>)>,
    section_items_map:
        std::collections::HashMap<String, Vec<(usize, Rc<todos::entity::ItemModel>)>>,
}

impl InboxBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = std::rc::Rc::new(todos::entity::ItemModel::default());
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
        let item_rows = vec![];
        let no_section_items = vec![];
        let section_items_map = std::collections::HashMap::new();

        let _subscriptions = vec![
            cx.observe_global_in::<InboxItemState>(window, move |this, window, cx| {
                let state_items = cx.global::<InboxItemState>().items.clone();
                this.item_rows = state_items
                    .iter()
                    .filter(|item| !item.checked)
                    .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
                    .collect();

                // 重新计算no_section_items和section_items_map
                this.no_section_items.clear();
                this.section_items_map.clear();

                for (i, item) in state_items.iter().enumerate() {
                    if !item.checked {
                        match item.section_id.as_deref() {
                            None | Some("") => this.no_section_items.push((i, item.clone())),
                            Some(sid) => {
                                this.section_items_map
                                    .entry(sid.to_string())
                                    .or_default()
                                    .push((i, item.clone()));
                            },
                        }
                    }
                }

                println!("no_section_items:{:?}", this.no_section_items.len());
                println!("section_items_map:{:?}", this.section_items_map.len());

                if let Some(ix) = this.active_index {
                    if ix >= this.item_rows.len() {
                        this.active_index = if this.item_rows.is_empty() { None } else { Some(0) };
                    }
                } else if !this.item_rows.is_empty() {
                    this.active_index = Some(0);
                }
                cx.notify();
            }),
            cx.observe_global_in::<ProjectState>(window, move |_, _, cx| {
                cx.notify();
            }),
            cx.observe_global_in::<SectionState>(window, move |_this, _, cx| {
                // When section state changes, trigger re-render to update section names
                println!("SectionState changed, triggering re-render");
                cx.notify();
            }),
        ];
        Self {
            focus_handle: cx.focus_handle(),
            _subscriptions,
            active_index: Some(0),
            item_rows,
            item_info,
            no_section_items,
            section_items_map,
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

    pub fn show_item_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        is_edit: bool,
        section_id: Option<String>,
    ) {
        // 获取当前选中的 ItemRow 的 item_info（如果是编辑模式）
        let item_info = if is_edit {
            if let Some(active_index) = self.active_index {
                if let Some(item_row) = self.item_rows.get(active_index) {
                    // 从选中的 ItemRow 中读取 item_info
                    item_row.read(cx).item_info.clone()
                } else {
                    self.item_info.clone()
                }
            } else {
                self.item_info.clone()
            }
        } else {
            // 新增模式使用默认的 item_info，并重置为空数据
            let mut ori_item = todos::entity::ItemModel::default();

            // 如果指定了section_id，设置它
            if let Some(sid) = section_id {
                ori_item.section_id = Some(sid);
            }

            self.item_info.update(cx, |state, cx| {
                state.set_item(std::rc::Rc::new(ori_item.clone()), window, cx);
                cx.notify();
            });
            self.item_info.clone()
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

    pub fn show_section_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: Option<String>,
        is_edit: bool,
    ) {
        let sections = cx.global::<SectionState>().sections.clone();
        let ori_section = if is_edit {
            sections
                .iter()
                .find(|s| s.id == section_id.clone().unwrap_or_default())
                .map(|s| s.as_ref().clone())
                .unwrap_or_default()
        } else {
            todos::entity::SectionModel::default()
        };

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Section Name"));
        if is_edit {
            name_input.update(cx, |is, cx| {
                is.set_value(ori_section.name.clone(), window, cx);
                cx.notify();
            })
        };

        let config = crate::components::SectionDialogConfig::new(
            if is_edit { "Edit Section" } else { "New Section" },
            if is_edit { "Save" } else { "Add" },
            is_edit,
        )
        .with_overlay(false);

        let view = cx.entity().clone();
        crate::components::show_section_dialog(window, cx, name_input, config, move |name, cx| {
            view.update(cx, |_view, cx| {
                let section =
                    std::rc::Rc::new(todos::entity::SectionModel { name, ..ori_section.clone() });
                if is_edit {
                    update_section(section, cx);
                } else {
                    add_section(section, cx);
                }
                cx.notify();
            });
        });
    }

    pub fn show_section_delete_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        let sections = cx.global::<SectionState>().sections.clone();
        let section_some = sections.iter().find(|s| s.id == section_id).cloned();
        if let Some(section) = section_some {
            let view = cx.entity().clone();
            window.open_dialog(cx, move |dialog, _, _| {
                dialog
                    .confirm()
                    .overlay(true)
                    .overlay_closable(true)
                    .child("Are you sure to delete the section?")
                    .on_ok({
                        let view = view.clone();
                        let section = section.clone();
                        move |_, window, cx| {
                            let view = view.clone();
                            let section = section.clone();
                            view.update(cx, |_view, cx| {
                                delete_section(section, cx);
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

    pub fn duplicate_section(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        let sections = cx.global::<SectionState>().sections.clone();
        if let Some(section) = sections.iter().find(|s| s.id == section_id) {
            let mut new_section = section.as_ref().clone();
            new_section.id = uuid::Uuid::new_v4().to_string();
            new_section.name = format!("{} (copy)", new_section.name);
            add_section(std::rc::Rc::new(new_section), cx);
            window.push_notification("Section duplicated successfully.", cx);
        }
    }

    pub fn archive_section(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        let sections = cx.global::<SectionState>().sections.clone();
        if let Some(section) = sections.iter().find(|s| s.id == section_id) {
            let mut updated_section = section.as_ref().clone();
            updated_section.is_archived = true;
            update_section(std::rc::Rc::new(updated_section), cx);
            window.push_notification("Section archived successfully.", cx);
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
        let sections = cx.global::<SectionState>().sections.clone();
        let no_section_items = self.no_section_items.clone();
        let section_items_map = self.section_items_map.clone();

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
                                    .sub_title(
                                        h_flex()
                                            .gap_1()
                                            .child(
                                                Button::new("more-no-section")
                                                    .small()
                                                    .ghost()
                                                    .compact()
                                                    .icon(IconName::EllipsisVertical)
                                                    .dropdown_menu({
                                                        let view = view_clone.clone();
                                                        move |this, window, _cx| {
                                                            this.item(
                                                                PopupMenuItem::new("+ Add Task").on_click(
                                                                    window.listener_for(&view, |this, _, window, cx| {
                                                                        this.show_item_dialog(window, cx, false, None);
                                                                        cx.notify();
                                                                    }),
                                                                ),
                                                            )
                                                                .separator()
                                                                .item(
                                                                    PopupMenuItem::new("Show Completed Tasks").on_click(
                                                                        window.listener_for(&view, |_this, _, _window, cx| {
                                                                            // TODO: Implement show completed tasks
                                                                            cx.notify();
                                                                        }),
                                                                    ),
                                                                )
                                                        }
                                                    }),
                                            ),
                                    )
                                    .child(v_flex().gap_2().w_full().children(
                                        no_section_items.into_iter().map(|(i, _item)| {
                                            let view = view_clone.clone();
                                            let is_active = self.active_index == Some(i);
                                            let item_row = self.item_rows.get(i).cloned();
                                            div()
                                                .id(("item", i))
                                                .on_click(move |_, _, cx| {
                                                    view.update(cx, |this, cx| {
                                                        this.active_index = Some(i);
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
                                                        this.show_item_dialog(window, cx, false, Some(section_id.clone()));
                                                        cx.notify();
                                                    })
                                                }
                                            }),
                                        ),
                                    )
                                    .sub_title(
                                        h_flex()
                                            .gap_1()
                                            .child(
                                                Button::new(format!("edit-section-{}", section_id))
                                                    .small()
                                                    .ghost()
                                                    .compact()
                                                    .icon(IconName::EditSymbolic)
                                                    .on_click({
                                                        let view = view_clone.clone();
                                                        let section_id = section_id.clone();
                                                        move |_, window, cx| {
                                                            view.update(cx, |this, cx| {
                                                                this.show_section_dialog(
                                                                    window,
                                                                    cx,
                                                                    Some(section_id.clone()),
                                                                    true,
                                                                );
                                                                cx.notify();
                                                            })
                                                        }
                                                    }),
                                            )
                                            .child(
                                                Button::new(format!(
                                                    "delete-section-{}",
                                                    section_id
                                                ))
                                                .small()
                                                .ghost()
                                                .compact()
                                                .icon(IconName::UserTrashSymbolic)
                                                .on_click({
                                                    let view = view_clone.clone();
                                                    let section_id = section_id.clone();
                                                    move |_, window, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.show_section_delete_dialog(
                                                                window,
                                                                cx,
                                                                section_id.clone(),
                                                            );
                                                            cx.notify();
                                                        })
                                                    }
                                                }),
                                            )
                                            .child(
                                                Button::new(format!("more-section-{}", section_id))
                                                    .small()
                                                    .ghost()
                                                    .compact()
                                                    .icon(IconName::EllipsisVertical)
                                                    .dropdown_menu({
                                                        let view = view_clone.clone();
                                                        let section_id = section_id.clone();
                                                        move |this, window, _cx| {
                                                            let view = view.clone();
                                                            let section_id = section_id.clone();
                                                            this.item({
                                                                let view = view.clone();
                                                                let section_id = section_id.clone();
                                                                PopupMenuItem::new("+ Add Task").on_click(
                                                                    window.listener_for(&view, move |this, _, window, cx| {
                                                                        this.show_item_dialog(window, cx, false, Some(section_id.clone()));
                                                                        cx.notify();
                                                                    }),
                                                                )
                                                            })
                                                                .separator()
                                                                .item({
                                                                    let view = view.clone();
                                                                    let section_id = section_id.clone();
                                                                    PopupMenuItem::new("Edit Section").on_click(
                                                                        window.listener_for(&view, move |this, _, window, cx| {
                                                                            this.show_section_dialog(window, cx, Some(section_id.clone()), true);
                                                                            cx.notify();
                                                                        })
                                                                    )
                                                                })
                                                                .separator()
                                                                .item({
                                                                    let view = view.clone();
                                                                    PopupMenuItem::new("Move Section").on_click(
                                                                        window.listener_for(&view, |_this, _, _window, cx| {
                                                                            // TODO: Implement move section to another project
                                                                            cx.notify();
                                                                        }),
                                                                    )
                                                                })
                                                                .separator()
                                                                .item({
                                                                    let view = view.clone();
                                                                    PopupMenuItem::new("Manage Sections").on_click(
                                                                        window.listener_for(&view, |_this, _, _window, cx| {
                                                                            // TODO: Implement manage sections dialog
                                                                            cx.notify();
                                                                        }),
                                                                    )
                                                                })
                                                                .separator()
                                                                .item({
                                                                    let view = view.clone();
                                                                    let section_id = section_id.clone();
                                                                    PopupMenuItem::new("Duplicate").on_click(
                                                                        window.listener_for(&view, move |this, _, window, cx| {
                                                                            this.duplicate_section(window, cx, section_id.clone());
                                                                            cx.notify();
                                                                        })
                                                                    )
                                                                })
                                                                .separator()
                                                                .item({
                                                                    let view = view.clone();
                                                                    PopupMenuItem::new("Show Completed Tasks").on_click(
                                                                        window.listener_for(&view, |_this, _, _window, cx| {
                                                                            // TODO: Implement show completed tasks
                                                                            cx.notify();
                                                                        }),
                                                                    )
                                                                })
                                                                .separator()
                                                                .item({
                                                                    let view = view.clone();
                                                                    let section_id = section_id.clone();
                                                                    PopupMenuItem::new("Archive").on_click(
                                                                        window.listener_for(&view, move |this, _, window, cx| {
                                                                            this.archive_section(window, cx, section_id.clone());
                                                                            cx.notify();
                                                                        })
                                                                    )
                                                                })
                                                                .separator()
                                                                .item({
                                                                    let view = view.clone();
                                                                    let section_id = section_id.clone();
                                                                    PopupMenuItem::new("Delete Section").on_click(
                                                                        window.listener_for(&view, move |this, _, window, cx| {
                                                                            this.show_section_delete_dialog(window, cx, section_id.clone());
                                                                            cx.notify();
                                                                        })
                                                                    )
                                                                })
                                                        }
                                                    }),
                                            ),
                                    )
                                    .child(v_flex().gap_2().w_full().children(items.iter().map(
                                        |(i, _item)| {
                                            let view = view_clone.clone();
                                            let i = *i;
                                            let is_active = self.active_index == Some(i);
                                            let item_row = self.item_rows.get(i).cloned();
                                            div()
                                                .id(("item", i))
                                                .on_click(move |_, _, cx| {
                                                    view.update(cx, |this, cx| {
                                                        this.active_index = Some(i);
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
