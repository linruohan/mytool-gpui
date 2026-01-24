use std::rc::Rc;

use gpui::{
    App, AppContext, BorrowAppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement as _, MouseButton, ParentElement, Render, StatefulInteractiveElement as _,
    Styled, Subscription, Window, div, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme as _, IconName, IndexPath, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};
use sea_orm::sqlx::types::uuid;
use todos::entity::{ItemModel, ProjectModel};

use crate::{
    ItemEvent, ItemInfo, ItemInfoEvent, ItemInfoState, ItemRow, ItemRowState, section,
    todo_actions::{
        add_project_item, add_section, delete_project_item, delete_section, load_project_items,
        update_project_item, update_section,
    },
    todo_state::ProjectState,
};

pub enum ProjectItemEvent {
    Loaded,
    Added(Rc<ItemModel>),
    Modified(Rc<ItemModel>),
    Deleted(Rc<ItemModel>),
}

impl EventEmitter<ProjectItemEvent> for ProjectItemsPanel {}
impl EventEmitter<ItemInfoEvent> for ProjectItemsPanel {}
impl EventEmitter<ItemEvent> for ProjectItemsPanel {}

pub struct ProjectItemsPanel {
    project: Rc<ProjectModel>,
    pub active_index: Option<usize>,
    item_rows: Vec<Entity<ItemRowState>>,
    item_info: Entity<ItemInfoState>,
    _subscriptions: Vec<Subscription>,
    focus_handle: FocusHandle,
    no_section_items: Vec<(usize, Rc<ItemModel>)>,
    section_items_map: std::collections::HashMap<String, Vec<(usize, Rc<ItemModel>)>>,
}

impl ProjectItemsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = Rc::new(ItemModel::default());
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
        let item_rows = vec![];
        let no_section_items = vec![];
        let section_items_map = std::collections::HashMap::new();

        let _subscriptions =
            vec![cx.observe_global_in::<ProjectState>(window, move |this, window, cx| {
                let state_items = cx.global::<ProjectState>().items.clone();
                this.item_rows = state_items
                    .iter()
                    .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
                    .collect();

                // 重新计算no_section_items和section_items_map
                this.no_section_items.clear();
                this.section_items_map.clear();

                for (i, item) in state_items.iter().enumerate() {
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

                if let Some(ix) = this.active_index {
                    if ix >= this.item_rows.len() {
                        this.active_index = this.item_rows.is_empty().then_some(0).or(None);
                    }
                } else if !this.item_rows.is_empty() {
                    this.active_index = Some(0);
                }
                cx.notify();
            })];

        Self {
            active_index: Some(0),
            item_rows,
            item_info,
            _subscriptions,
            project: Rc::new(ProjectModel::default()),
            focus_handle: cx.focus_handle(),
            no_section_items,
            section_items_map,
        }
    }

    pub fn set_project(&mut self, project: Rc<ProjectModel>, cx: &mut Context<Self>) {
        self.project = project.clone();
        cx.update_global::<ProjectState, _>(|state, _| {
            state.items.clear();
        });
        self.active_index = Some(0);
        load_project_items(project.clone(), cx);
    }

    pub(crate) fn get_selected_item(&self, ix: IndexPath, cx: &App) -> Option<Rc<ItemModel>> {
        let item_list = cx.global::<ProjectState>().items.clone();
        item_list.get(ix.row).cloned()
    }

    pub fn update_active_index(&mut self, value: Option<usize>) {
        self.active_index = value;
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn handle_project_item_event(&mut self, event: &ProjectItemEvent, cx: &mut Context<Self>) {
        match event {
            ProjectItemEvent::Added(item) => {
                add_project_item(self.project.clone(), item.clone(), cx);
            },
            ProjectItemEvent::Modified(item) => {
                update_project_item(self.project.clone(), item.clone(), cx)
            },
            ProjectItemEvent::Deleted(item) => {
                delete_project_item(self.project.clone(), item.clone(), cx)
            },
            _ => {},
        }
    }

    fn initialize_item_model(&self, _is_edit: bool, _: &mut Window, cx: &mut App) -> ItemModel {
        self.active_index
            .and_then(|index| self.get_selected_item(IndexPath::new(index), cx))
            .map(|item| {
                let item_ref = item.as_ref();
                ItemModel { ..item_ref.clone() }
            })
            .unwrap_or_default()
    }

    pub fn show_item_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        is_edit: bool,
        section_id: Option<String>,
    ) {
        let item_info = self.item_info.clone();
        let mut ori_item = self.initialize_item_model(is_edit, window, cx);

        // If adding a new item with a section_id, set it
        if !is_edit {
            if let Some(sid) = section_id {
                ori_item.section_id = Some(sid);
            }
        }

        item_info.update(cx, |state, cx| {
            state.set_item(Rc::new(ori_item.clone()), window, cx);
            cx.notify();
        });

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
                                item_info.update(cx, |_item_info, cx| {
                                    cx.emit(ItemInfoEvent::Updated());
                                    cx.notify();
                                });
                                view.update(cx, |_view, cx| {
                                    let item = item_info.read(cx).item.clone();
                                    let event = if is_edit {
                                        ProjectItemEvent::Modified(item.clone())
                                    } else {
                                        ProjectItemEvent::Added(item.clone())
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
                                view.update(cx, |_, cx| {
                                    cx.emit(ProjectItemEvent::Deleted(item.clone()));
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

    pub fn show_section_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: Option<String>,
        is_edit: bool,
    ) {
        let sections = cx.global::<ProjectState>().sections.clone();
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

        let view = cx.entity().clone();
        let dialog_title = if is_edit { "Edit Section" } else { "New Section" };
        let button_label = if is_edit { "Save" } else { "Add" };

        window.open_dialog(cx, move |modal, _, _| {
            modal
                .title(dialog_title)
                .overlay(false)
                .keyboard(true)
                .overlay_closable(true)
                .child(v_flex().gap_3().child(Input::new(&name_input)))
                .footer({
                    let view = view.clone();
                    let ori_section = ori_section.clone();
                    let name_input_clone = name_input.clone();
                    move |_, _, _, _cx| {
                        vec![
                            Button::new("save").primary().label(button_label).on_click({
                                let view = view.clone();
                                let ori_section = ori_section.clone();
                                let name_input_clone1 = name_input_clone.clone();
                                move |_, window, cx| {
                                    window.close_dialog(cx);
                                    view.update(cx, |_view, cx| {
                                        let section =
                                            std::rc::Rc::new(todos::entity::SectionModel {
                                                name: name_input_clone1
                                                    .read(cx)
                                                    .value()
                                                    .to_string(),
                                                ..ori_section.clone()
                                            });
                                        if is_edit {
                                            update_section(section, cx);
                                        } else {
                                            add_section(section, cx);
                                        }
                                        cx.notify();
                                    });
                                }
                            }),
                            Button::new("cancel").label("Cancel").on_click(move |_, window, cx| {
                                window.close_dialog(cx);
                            }),
                        ]
                    }
                })
        });
    }

    pub fn show_section_delete_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        let sections = cx.global::<ProjectState>().sections.clone();
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
        let sections = cx.global::<ProjectState>().sections.clone();
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
        let sections = cx.global::<ProjectState>().sections.clone();
        if let Some(section) = sections.iter().find(|s| s.id == section_id) {
            let mut updated_section = section.as_ref().clone();
            updated_section.is_archived = true;
            update_section(std::rc::Rc::new(updated_section), cx);
            window.push_notification("Section archived successfully.", cx);
        }
    }
}

impl Focusable for ProjectItemsPanel {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ProjectItemsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity().clone();
        let sections = cx.global::<ProjectState>().sections.clone();
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
                    .child(v_flex().child(div().text_xl().child(self.project.name.clone())))
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
                                                                this.show_section_dialog(window, cx, Some(section_id.clone()), true);
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
                                                                this.show_section_delete_dialog(window, cx, section_id.clone());
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
