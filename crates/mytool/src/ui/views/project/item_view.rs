use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    InteractiveElement as _, MouseButton, ParentElement, Render, StatefulInteractiveElement as _,
    Styled, Subscription, Window, div, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme as _, Colorize, IconName, IndexPath, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    dialog::{DialogAction, DialogClose, DialogFooter},
    h_flex,
    input::{Input, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};
use sea_orm::sqlx::types::uuid;
use todos::entity::{ItemModel, ProjectModel};

use crate::{
    ColorGroup, ColorGroupEvent, ColorGroupState, ItemEvent, ItemInfoEvent, ItemInfoState, ItemRow,
    ItemRowState, VisualHierarchy, section,
    todo_actions::{
        add_section, delete_project, delete_project_item, delete_section, load_project_items,
        update_project, update_project_item, update_section,
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
    pinned_items: Vec<(usize, Arc<ItemModel>)>,
    no_section_items: Vec<(usize, Arc<ItemModel>)>,
    section_items_map: std::collections::HashMap<String, Vec<(usize, Arc<ItemModel>)>>,
    cached_version: usize,
    color: Entity<ColorGroupState>,
    selected_color: Option<Hsla>,
    project_due: Option<String>,
}

impl ProjectItemsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = Arc::new(ItemModel::default());
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
        let item_rows = vec![];
        let pinned_items = vec![];
        let no_section_items = vec![];
        let section_items_map = std::collections::HashMap::new();
        let color = cx.new(|cx| ColorGroupState::new(window, cx).default_value(cx.theme().primary));

        let _subscriptions = vec![
            cx.observe_global_in::<TodoStore>(window, move |this, window, cx| {
                let todo_store = cx.global::<TodoStore>();

                if this.cached_version == todo_store.version() {
                    return;
                }
                this.cached_version = todo_store.version();

                if this.project.id.is_empty() {
                    tracing::debug!("ProjectItemsPanel: project.id 为空,跳过加载 items");
                    return;
                }

                let state_items = todo_store.items_by_project(&this.project.id);
                this.item_rows = state_items
                    .iter()
                    .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
                    .collect();

                this.pinned_items.clear();
                this.no_section_items.clear();
                this.section_items_map.clear();

                for (i, item) in state_items.iter().enumerate() {
                    if !item.checked && item.pinned {
                        this.pinned_items.push((i, item.clone()));
                    }

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

                tracing::debug!("ProjectItemsPanel 已更新, items 数量: {}", this.item_rows.len());
                cx.notify();
            }),
            cx.subscribe(&color, |this, _, ev, _| match ev {
                ColorGroupEvent::Change(color) => {
                    this.selected_color = *color;
                },
            }),
        ];

        Self {
            active_index: Some(0),
            item_rows,
            item_info,
            _subscriptions,
            project: Arc::new(ProjectModel::default()),
            focus_handle: cx.focus_handle(),
            pinned_items,
            no_section_items,
            section_items_map,
            cached_version: 0,
            color,
            selected_color: None,
            project_due: None,
        }
    }

    pub fn set_project(&mut self, project: Arc<ProjectModel>, cx: &mut Context<Self>) {
        tracing::debug!(
            "ProjectItemsPanel::set_project, project_id: {}, project_name: {}",
            project.id,
            project.name
        );

        self.project = project.clone();
        self.active_index = Some(0);

        // 检查 project_id 是否有效
        if project.id.is_empty() {
            tracing::debug!("ProjectItemsPanel::set_project: project_id 为空,跳过加载 items");
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
            // 注意：Added 事件不再调用 add_project_item，因为 save_all_changes 已经处理了保存
            // add_item_optimistic 会在保存时自动添加到 TodoStore
            ProjectItemEvent::Added(_item) => {
                // 已经在 save_all_changes 中通过 add_item_optimistic 处理
                tracing::debug!(
                    "ProjectItemEvent::Added - item already saved via save_all_changes"
                );
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

    fn initialize_item_model(&self, is_edit: bool, _: &mut Window, cx: &mut App) -> ItemModel {
        // 新建 item 时直接返回默认值，不复制选中 item 的内容
        if !is_edit {
            return ItemModel::default();
        }

        // 编辑时才获取当前选中的 item
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

        // If adding a new item with a section_id, set project_id and section_id
        if !is_edit {
            // 设置默认的 project_id
            ori_item.project_id = Some(self.project.id.clone());
            // 设置 section_id（如果有）
            if let Some(sid) = section_id {
                ori_item.section_id = Some(sid);
            }
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
            // 新建 section 时，绑定当前 project 的 project_id
            todos::entity::SectionModel {
                project_id: Some(self.project.id.clone()),
                ..Default::default()
            }
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

    /// 显示项目编辑对话框，支持修改项目名称、颜色和截止日期
    pub fn show_project_edit_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Project Name"));
        name_input.update(cx, |is, cx| {
            is.set_value(self.project.name.clone(), window, cx);
            cx.notify();
        });

        let color = self.color.clone();
        if let Some(project_color) = &self.project.color
            && let Ok(hsla_color) = gpui::Hsla::parse_hex(project_color)
        {
            color.update(cx, |cs, cx| {
                cs.set_value(hsla_color, window, cx);
                cx.notify();
            });
        }

        let now = chrono::Local::now().naive_local().date();
        let project_due = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
            if let Some(due) = &self.project.due_date {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(due, "%Y-%m-%d") {
                    picker.set_date(date, window, cx);
                }
            } else {
                picker.set_date(now, window, cx);
            }
            picker
        });

        let view = cx.entity().clone();
        let ori_project = self.project.as_ref().clone();
        let _ = cx.subscribe(&project_due, |this, _, ev, _| match ev {
            DatePickerEvent::Change(date) => {
                this.project_due = date.format("%Y-%m-%d").map(|s| s.to_string());
            },
        });

        window.open_dialog(cx, move |modal, _, _| {
            modal
                .title("Edit Project")
                .overlay(false)
                .keyboard(true)
                .overlay_closable(true)
                .child(
                    v_flex()
                        .gap(VisualHierarchy::spacing(3.0))
                        .child(Input::new(&name_input))
                        .child(ColorGroup::new(&color))
                        .child(DatePicker::new(&project_due).placeholder("DueDate of Project")),
                )
                .footer(
                    DialogFooter::new()
                        .child(
                            DialogClose::new()
                                .child(Button::new("cancel").label("Cancel").outline()),
                        )
                        .child(
                            DialogAction::new().child(Button::new("save").primary().label("Save")),
                        ),
                )
                .on_ok({
                    let view = view.clone();
                    let ori_project = ori_project.clone();
                    let name_input = name_input.clone();
                    move |_, _window: &mut Window, cx| {
                        view.update(cx, |view, cx| {
                            let updated_project = Arc::new(ProjectModel {
                                name: name_input.read(cx).value().to_string(),
                                due_date: view.project_due.clone().or(ori_project.due_date.clone()),
                                color: Some(
                                    view.selected_color.map(|c| c.to_hex()).unwrap_or_default(),
                                ),
                                ..ori_project.clone()
                            });
                            update_project(updated_project, cx);
                            cx.notify();
                        });
                        true
                    }
                })
        });
    }

    /// 显示项目删除确认对话框
    pub fn show_project_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let project = self.project.clone();
        let view = cx.entity().clone();
        crate::ui::components::show_delete_dialog(
            window,
            cx,
            "Are you sure to delete this project? All tasks and sections will be deleted.",
            move |cx| {
                view.update(cx, |_view, cx| {
                    delete_project(project.clone(), cx);
                    cx.notify();
                });
            },
        );
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
                    .items_center()
                    .child(
                        h_flex()
                            .items_center()
                            .gap(VisualHierarchy::spacing(2.0))
                            .child(div().text_xl().child(self.project.name.clone()))
                            .child(
                                Button::new("edit-project")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::EditSymbolic)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_project_edit_dialog(window, cx);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("delete-project")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::UserTrashSymbolic)
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_project_delete_dialog(window, cx);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(VisualHierarchy::spacing(2.0))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                Button::new("add-item")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::PlusLargeSymbolic)
                                    .label("Add Task")
                                    .dropdown_menu({
                                        let view = view.clone();
                                        move |this, window, _cx| {
                                            // 添加 "No Section" 选项
                                            this.item(
                                                PopupMenuItem::new("No Section").on_click(
                                                    window.listener_for(&view, |this, _, window, cx| {
                                                        this.show_item_dialog(window, cx, false, None);
                                                        cx.notify();
                                                    }),
                                                ),
                                            )
                                            .separator()
                                        }
                                    }),
                            )
                            .child(
                                Button::new("add-item-to-section")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::FolderOpen)
                                    .label("Add to Section")
                                    .dropdown_menu({
                                        let view = view.clone();
                                        let project_id = self.project.id.clone();
                                        move |mut this, window, cx| {
                                            // 获取当前 project 的所有 sections
                                            let sections = cx.global::<TodoStore>().sections.clone();
                                            let project_sections: Vec<_> = sections
                                                .iter()
                                                .filter(|s| s.project_id.as_deref() == Some(&project_id))
                                                .collect();

                                            // 为每个 section 添加菜单项
                                            for section in project_sections {
                                                let section_id = section.id.clone();
                                                let section_name = section.name.clone();
                                                this = this.item(
                                                    PopupMenuItem::new(section_name).on_click(
                                                        window.listener_for(&view, move |this, _, window, cx| {
                                                            this.show_item_dialog(window, cx, false, Some(section_id.clone()));
                                                            cx.notify();
                                                        }),
                                                    ),
                                                );
                                            }

                                            this
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
                            )
                            .child(
                                Button::new("section-actions")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::PlusLargeSymbolic)
                                    .label("Add Section")
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_section_dialog(window, cx, None, false);
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
                        // 1. Pinned 分组
                        .when(!self.pinned_items.is_empty(), |this| {
                            let view_clone = view.clone();
                            let view_clone_for_dropdown = view_clone.clone();
                            let pinned_items = self.pinned_items.clone();
                            let item_rows = self.item_rows.clone();
                            let active_index = self.active_index;
                            let active_border = cx.theme().list_active_border;

                            // 渲染 pinned items 列表
                            let pinned_items_view = v_flex()
                                .gap(VisualHierarchy::spacing(2.0))
                                .w_full()
                                .children(pinned_items.into_iter().map(move |(i, _item)| {
                                    let view = view_clone.clone();
                                    let is_active = active_index == Some(i);
                                    let item_row = item_rows.get(i).cloned();
                                    div()
                                        .id(("pinned-item", i))
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.active_index = Some(i);
                                                cx.notify();
                                            });
                                        })
                                        .when(is_active, |this| {
                                            this.border_color(active_border)
                                        })
                                        .children(item_row.map(|row| ItemRow::new(&row)))
                                }));

                            this.child(
                                section("Pinned")
                                    .sub_title(
                                        h_flex().gap(VisualHierarchy::spacing(1.0)).child(
                                            Button::new("more-pinned")
                                                .small()
                                                .ghost()
                                                .compact()
                                                .icon(IconName::EllipsisVertical)
                                                .dropdown_menu({
                                                    let view = view_clone_for_dropdown.clone();
                                                    move |this, window, _cx| {
                                                        this.item(
                                                            PopupMenuItem::new("Show Completed Tasks")
                                                                .on_click(
                                                                    window.listener_for(&view, |_this, _, _window, cx| {
                                                                        cx.notify();
                                                                    }),
                                                                ),
                                                        )
                                                    }
                                                }),
                                        ),
                                    )
                                    .child(pinned_items_view),
                            )
                        })
                        // 2. No Section 分组
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
