use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement as _, MouseButton, ParentElement, Render, StatefulInteractiveElement as _,
    Styled, Subscription, Window, div, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme as _, IconName, IndexPath, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    input::InputState,
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};
use sea_orm::sqlx::types::uuid;
use todos::entity::{ItemModel, ProjectModel};

use crate::{
    ItemEvent, ItemInfoEvent, ItemInfoState, ItemRow, ItemRowState, VisualHierarchy, section,
    todo_actions::{
        add_project_item, add_section, delete_project_item, delete_section, load_project_items,
        update_project_item, update_section,
    },
    todo_state::TodoStore,
};

pub enum ProjectItemEvent {
    Loaded,
    Added(Arc<ItemModel>),
    Modified(Arc<ItemModel>),
    Deleted(Arc<ItemModel>),
}

impl EventEmitter<ProjectItemEvent> for ProjectItemsPanel {}
impl EventEmitter<ItemInfoEvent> for ProjectItemsPanel {}
impl EventEmitter<ItemEvent> for ProjectItemsPanel {}

pub struct ProjectItemsPanel {
    project: Arc<ProjectModel>,
    pub active_index: Option<usize>,
    item_rows: Vec<Entity<ItemRowState>>,
    item_info: Entity<ItemInfoState>,
    _subscriptions: Vec<Subscription>,
    focus_handle: FocusHandle,
    no_section_items: Vec<(usize, Arc<ItemModel>)>,
    section_items_map: std::collections::HashMap<String, Vec<(usize, Arc<ItemModel>)>>,
    /// 缓存的 TodoStore 版本号，用于优化性能
    cached_version: usize,
}

impl ProjectItemsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = Arc::new(ItemModel::default());
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
        let item_rows = vec![];
        let no_section_items = vec![];
        let section_items_map = std::collections::HashMap::new();

        let _subscriptions =
            vec![cx.observe_global_in::<TodoStore>(window, move |this, window, cx| {
                let todo_store = cx.global::<TodoStore>();

                // 性能优化：检查版本号，只在数据变化时更新
                if this.cached_version == todo_store.version() {
                    return;
                }
                this.cached_version = todo_store.version();

                // 检查 project.id 是否有效
                if this.project.id.is_empty() {
                    println!("[DEBUG] ProjectItemsPanel: project.id 为空,跳过加载 items");
                    return;
                }

                let state_items = todo_store.items_by_project(&this.project.id);
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

                println!("[DEBUG] ProjectItemsPanel 已更新, items 数量: {}", this.item_rows.len());
                cx.notify();
            })];

        Self {
            active_index: Some(0),
            item_rows,
            item_info,
            _subscriptions,
            project: Arc::new(ProjectModel::default()),
            focus_handle: cx.focus_handle(),
            no_section_items,
            section_items_map,
            cached_version: 0,
        }
    }

    pub fn set_project(&mut self, project: Arc<ProjectModel>, cx: &mut Context<Self>) {
        println!(
            "[DEBUG] ProjectItemsPanel::set_project, project_id: {}, project_name: {}",
            project.id, project.name
        );

        self.project = project.clone();
        self.active_index = Some(0);

        // 检查 project_id 是否有效
        if project.id.is_empty() {
            println!("[ERROR] ProjectItemsPanel::set_project: project_id 为空,跳过加载 items");
            return;
        }

        load_project_items(project.clone(), cx);
    }

    pub(crate) fn get_selected_item(&self, ix: IndexPath, cx: &App) -> Option<Arc<ItemModel>> {
        let todo_store = cx.global::<TodoStore>();
        let item_list = todo_store.items_by_project(&self.project.id);
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
        if !is_edit && let Some(sid) = section_id {
            ori_item.section_id = Some(sid);
        }

        item_info.update(cx, |state, cx| {
            state.set_item(Arc::new(ori_item.clone()), window, cx);
            cx.notify();
        });

        let config = crate::ui::components::ItemDialogConfig::new(
            if is_edit { "Edit Item" } else { "New Item" },
            if is_edit { "Save" } else { "Add" },
            is_edit,
        );

        let view = cx.entity().clone();
        crate::ui::components::show_item_dialog(
            window,
            cx,
            item_info.clone(),
            config,
            move |item, cx| {
                item_info.update(cx, |_item_info, cx| {
                    cx.emit(ItemInfoEvent::Updated());
                    cx.notify();
                });
                view.update(cx, |_view, cx| {
                    let arc_item = Arc::new((*item).clone());
                    let event = if is_edit {
                        ProjectItemEvent::Modified(arc_item.clone())
                    } else {
                        ProjectItemEvent::Added(arc_item.clone())
                    };
                    cx.emit(event);
                    cx.notify();
                });
            },
        );
    }

    pub fn show_item_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.active_index {
            let item_some = self.get_selected_item(IndexPath::new(active_index), cx);
            if let Some(item) = item_some {
                let view = cx.entity().clone();
                crate::ui::components::show_item_delete_dialog(
                    window,
                    cx,
                    "Are you sure to delete the item?",
                    move |cx| {
                        view.update(cx, |_, cx| {
                            cx.emit(ProjectItemEvent::Deleted(item.clone()));
                        });
                    },
                );
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
        let sections = cx.global::<TodoStore>().sections.clone();
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

        let config = crate::ui::components::SectionDialogConfig::new(
            if is_edit { "Edit Section" } else { "New Section" },
            if is_edit { "Save" } else { "Add" },
            is_edit,
        )
        .with_overlay(false);

        let view = cx.entity().clone();
        crate::ui::components::show_section_dialog(
            window,
            cx,
            name_input,
            config,
            move |name, cx| {
                view.update(cx, |_view, cx| {
                    let section =
                        Arc::new(todos::entity::SectionModel { name, ..ori_section.clone() });
                    if is_edit {
                        update_section(section, cx);
                    } else {
                        add_section(section, cx);
                    }
                    cx.notify();
                });
            },
        );
    }

    pub fn show_section_delete_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        let sections = cx.global::<TodoStore>().sections.clone();
        let section_some = sections.iter().find(|s| s.id == section_id).cloned();
        if let Some(section) = section_some {
            let view = cx.entity().clone();
            crate::ui::components::show_section_delete_dialog(
                window,
                cx,
                "Are you sure to delete the section?",
                move |cx| {
                    view.update(cx, |_view, cx| {
                        delete_section(section.clone(), cx);
                        cx.notify();
                    });
                },
            );
        };
    }

    pub fn duplicate_section(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        let sections = cx.global::<TodoStore>().sections.clone();
        if let Some(section) = sections.iter().find(|s| s.id == section_id) {
            let mut new_section = section.as_ref().clone();
            new_section.id = uuid::Uuid::new_v4().to_string();
            new_section.name = format!("{} (copy)", new_section.name);
            add_section(Arc::new(new_section), cx);
            window.push_notification("Section duplicated successfully.", cx);
        }
    }

    pub fn archive_section(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        let sections = cx.global::<TodoStore>().sections.clone();
        if let Some(section) = sections.iter().find(|s| s.id == section_id) {
            let mut updated_section = section.as_ref().clone();
            updated_section.is_archived = true;
            update_section(Arc::new(updated_section), cx);
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
        let sections = cx.global::<TodoStore>().sections.clone();
        let no_section_items = self.no_section_items.clone();
        let section_items_map = self.section_items_map.clone();

        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .gap(VisualHierarchy::spacing(4.0))
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
                            .gap(VisualHierarchy::spacing(2.0))
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
                        .gap(VisualHierarchy::spacing(4.0))
                        .when(!no_section_items.is_empty(), |this| {
                            let view_clone = view.clone();
                            this.child(
                                section("No Section")
                                    .sub_title(
                                        h_flex().gap(VisualHierarchy::spacing(1.0)).child(
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
                                    .child(v_flex().gap(VisualHierarchy::spacing(2.0)).w_full().children(
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
                                        h_flex().gap(VisualHierarchy::spacing(1.0)).child(
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
                                            .gap(VisualHierarchy::spacing(1.0))
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
                                    .child(v_flex().gap(VisualHierarchy::spacing(2.0)).w_full().children(items.iter().map(
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
