use std::sync::Arc;

use gpui::{
    App, AppContext, BorrowAppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    InteractiveElement as _, MouseButton, ParentElement, Render, Styled, Subscription, Window, div,
    prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme as _, Colorize, IndexPath, Sizable, StyledExt, WindowExt,
    button::{Button, ButtonVariants},
    color_picker::{ColorPickerEvent, ColorPickerState},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    dialog::{DialogAction, DialogClose, DialogFooter},
    form::{field, v_form},
    h_flex,
    input::{Input, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};
use gpui_kit::assets::IconName;
use rust_i18n::t;
use sea_orm::sqlx::types::uuid;
use todos::entity::{ItemModel, ProjectModel};

use crate::{
    ItemEvent, ItemInfoEvent, ItemInfoState, ItemRowState, VisualHierarchy, board_section,
    todo_actions::{
        add_project, add_section, delete_project, delete_project_item, delete_section,
        load_project_items, update_project, update_project_item, update_section,
    },
    todo_color_picker,
    todo_state::TodoStore,
    ui::views::boards::{
        BoardView, PinnedLayout, board_common, board_renderer, clamp_active_index,
        diff_update_item_rows, group_items, reorder_in_groups,
    },
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
    color: Entity<ColorPickerState>,
    selected_color: Option<Hsla>,
    project_due: Option<String>,
    item_row_ids: Vec<String>,
}

impl ProjectItemsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = Arc::new(ItemModel::default());
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
        let item_rows = vec![];
        let pinned_items = vec![];
        let no_section_items = vec![];
        let section_items_map = std::collections::HashMap::new();
        let color =
            cx.new(|cx| ColorPickerState::new(window, cx).default_value(cx.theme().primary));

        let _subscriptions = vec![
            cx.observe_global_in::<TodoStore>(window, move |this, window, cx| {
                if !cx.global::<TodoStore>().peek_change_mask().affects_project() {
                    return;
                }
                this.reload_project_items(window, cx);
            }),
            cx.subscribe(&color, |this, _, ev, _| match ev {
                ColorPickerEvent::Change(color) => {
                    this.selected_color = *color;
                },
            }),
            cx.observe_global::<crate::core::state::ItemSelection>(|_, cx| {
                cx.notify();
            }),
        ];

        Self {
            active_index: None,
            item_rows,
            item_info,
            _subscriptions,
            project: Arc::new(ProjectModel::default()),
            focus_handle: cx.focus_handle(),
            pinned_items,
            no_section_items,
            section_items_map,
            color,
            selected_color: None,
            project_due: None,
            item_row_ids: Vec::new(),
        }
    }

    pub fn set_project(&mut self, project: Arc<ProjectModel>, cx: &mut Context<Self>) {
        tracing::debug!(
            "ProjectItemsPanel::set_project, project_id: {}, project_name: {}",
            project.id,
            project.name
        );

        self.project = project.clone();
        self.active_index = None;

        // 检查 project_id 是否有效
        if project.id.is_empty() {
            tracing::debug!("ProjectItemsPanel::set_project: project_id 为空,跳过加载 items");
            return;
        }

        load_project_items(project.clone(), cx);
    }

    fn reload_project_items(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.project.id.is_empty() {
            tracing::debug!("ProjectItemsPanel: project.id 为空,跳过加载 items");
            return;
        }
        let state_items = cx.global::<TodoStore>().items_by_project(&self.project.id).to_vec();
        diff_update_item_rows(
            &mut self.item_rows,
            &mut self.item_row_ids,
            &state_items,
            window,
            cx,
        );
        let grouped = group_items(&state_items, PinnedLayout::Inclusive, false);
        self.pinned_items = grouped.pinned;
        self.no_section_items = grouped.no_section;
        self.section_items_map = grouped.sections;
        clamp_active_index(&mut self.active_index, self.item_rows.len());
        tracing::debug!("ProjectItemsPanel 已更新, items 数量: {}", self.item_rows.len());
        cx.notify();
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

        let title = if is_edit {
            t!("todo.item.edit").to_string()
        } else {
            t!("todo.item.new").to_string()
        };
        let button = if is_edit { t!("todo.save").to_string() } else { t!("todo.add").to_string() };
        let config = crate::ui::components::ItemDialogConfig::new(&title, &button, is_edit);

        let view = cx.entity().clone();
        crate::ui::components::show_item_dialog(
            window,
            cx,
            item_info.clone(),
            config,
            move |item, window, cx| {
                item_info.update(cx, |_item_info, cx| {
                    cx.emit(ItemInfoEvent::Updated());
                    cx.notify();
                });
                view.update(cx, |this, cx| {
                    let arc_item = Arc::new((*item).clone());
                    let event = if is_edit {
                        ProjectItemEvent::Modified(arc_item.clone())
                    } else {
                        ProjectItemEvent::Added(arc_item.clone())
                    };
                    cx.emit(event);
                    this.reload_project_items(window, cx);
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
                    &t!("todo.item.delete_confirm"),
                    move |window, cx| {
                        view.update(cx, |this, cx| {
                            cx.emit(ProjectItemEvent::Deleted(item.clone()));
                            this.reload_project_items(window, cx);
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
        let ori_section = if is_edit {
            section_id
                .as_deref()
                .and_then(|id| cx.global::<TodoStore>().get_section(id))
                .map(|s| s.as_ref().clone())
                .unwrap_or_default()
        } else {
            // 新建 section 时，绑定当前 project 的 project_id
            todos::entity::SectionModel {
                project_id: Some(self.project.id.clone()),
                ..Default::default()
            }
        };

        let name_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("todo.section.name_placeholder").to_string())
        });
        if is_edit {
            name_input.update(cx, |is, cx| {
                is.set_value(ori_section.name.clone(), window, cx);
                cx.notify();
            })
        };

        let title = if is_edit {
            t!("todo.section.edit").to_string()
        } else {
            t!("todo.section.new").to_string()
        };
        let button = if is_edit { t!("todo.save").to_string() } else { t!("todo.add").to_string() };
        let config = crate::ui::components::SectionDialogConfig::new(&title, &button, is_edit)
            .with_overlay(false);

        let view = cx.entity().clone();
        crate::ui::components::show_section_dialog(
            window,
            cx,
            name_input,
            config,
            move |name, window, cx| {
                view.update(cx, |this, cx| {
                    let section =
                        Arc::new(todos::entity::SectionModel { name, ..ori_section.clone() });
                    if is_edit {
                        update_section(section, cx);
                    } else {
                        add_section(section, cx);
                    }
                    this.reload_project_items(window, cx);
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
        if let Some(section) = cx.global::<TodoStore>().get_section(&section_id) {
            let view = cx.entity().clone();
            crate::ui::components::show_section_delete_dialog(
                window,
                cx,
                &t!("todo.section.delete_confirm"),
                move |window, cx| {
                    view.update(cx, |this, cx| {
                        delete_section(section.clone(), cx);
                        this.reload_project_items(window, cx);
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
        if let Some(section) = cx.global::<TodoStore>().get_section(&section_id) {
            let mut new_section = section.as_ref().clone();
            new_section.id = uuid::Uuid::new_v4().to_string();
            new_section.name = t!("todo.section.copy_name", name => &new_section.name).to_string();
            add_section(Arc::new(new_section), cx);
            window.push_notification(t!("todo.section.copied").to_string(), cx);
            self.reload_project_items(window, cx);
        }
    }

    pub fn archive_section(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        if let Some(section) = cx.global::<TodoStore>().get_section(&section_id) {
            let mut updated_section = section.as_ref().clone();
            updated_section.is_archived = true;
            update_section(Arc::new(updated_section), cx);
            window.push_notification(t!("todo.section.archived").to_string(), cx);
            self.reload_project_items(window, cx);
        }
    }

    /// 在当前项目下新建子项目
    pub fn show_child_project_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let parent_id = self.project.id.clone();
        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("todo.project.child_placeholder").to_string())
        });
        window.open_dialog(cx, move |modal, _, _| {
            modal
                .title(t!("todo.project.child_new").to_string())
                .overlay(true)
                .overlay_closable(true)
                .child(
                    v_form().child(
                        field()
                            .label(t!("todo.field.name").to_string())
                            .required(true)
                            .child(Input::new(&name_input)),
                    ),
                )
                .footer(
                    DialogFooter::new()
                        .child(DialogClose::new().child(
                            Button::new("cancel").label(t!("todo.cancel").to_string()).outline(),
                        ))
                        .child(
                            DialogAction::new().child(
                                Button::new("add").primary().label(t!("todo.add").to_string()),
                            ),
                        ),
                )
                .on_ok({
                    let input = name_input.clone();
                    let parent_id = parent_id.clone();
                    move |_, _, cx| {
                        let name = input.read(cx).value().to_string();
                        if name.trim().is_empty() {
                            return false;
                        }
                        let project = Arc::new(ProjectModel {
                            name,
                            parent_id: Some(parent_id.clone()),
                            ..ProjectModel::default()
                        });
                        add_project(project, cx);
                        true
                    }
                })
        });
    }

    pub fn show_project_edit_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("todo.project.name_placeholder").to_string())
        });
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
                .title(t!("todo.project.edit").to_string())
                .overlay(false)
                .keyboard(true)
                .overlay_closable(true)
                .child(
                    v_form()
                        .child(
                            field()
                                .label(t!("todo.field.name").to_string())
                                .required(true)
                                .child(Input::new(&name_input)),
                        )
                        .child(
                            field()
                                .label(t!("todo.field.color").to_string())
                                .child(todo_color_picker(&color)),
                        )
                        .child(
                            field().label(t!("todo.field.due").to_string()).child(
                                DatePicker::new(&project_due)
                                    .placeholder(t!("todo.project.due_placeholder").to_string()),
                            ),
                        ),
                )
                .footer(
                    DialogFooter::new()
                        .child(DialogClose::new().child(
                            Button::new("cancel").label(t!("todo.cancel").to_string()).outline(),
                        ))
                        .child(DialogAction::new().child(
                            Button::new("save").primary().label(t!("todo.save").to_string()),
                        )),
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
            &t!("todo.project.delete_confirm"),
            move |_window, cx| {
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

impl BoardView for ProjectItemsPanel {
    fn set_active_index(&mut self, index: Option<usize>) {
        self.active_index = index;
    }

    fn request_store_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reload_project_items(window, cx);
    }
}

impl ProjectItemsPanel {
    fn reorder_active(&mut self, delta: i32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(active_index) = self.active_index else {
            return;
        };
        let mut groups: Vec<&[_]> =
            vec![self.pinned_items.as_slice(), self.no_section_items.as_slice()];
        for items in self.section_items_map.values() {
            groups.push(items.as_slice());
        }
        let Some(updated) = reorder_in_groups(&groups, active_index, delta) else {
            return;
        };
        crate::core::actions::batch::batch_update_items(updated, cx);
        self.reload_project_items(window, cx);
    }

    pub fn reorder_by_item_id(
        &mut self,
        item_id: &str,
        delta: i32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(active_index) = self.item_row_ids.iter().position(|id| id == item_id) else {
            return;
        };
        self.active_index = Some(active_index);
        self.reorder_active(delta, window, cx);
    }
}

crate::impl_board_section_actions!(ProjectItemsPanel);

impl Render for ProjectItemsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity().clone();
        let project_sections: Vec<_> = {
            let mut sections: Vec<_> = cx
                .global::<TodoStore>()
                .sections_for_project(&self.project.id)
                .into_iter()
                .filter(|s| !s.is_archived && !s.is_deleted && !s.hidded)
                .collect();
            crate::todo_state::sort_sections_by_order(&mut sections);
            sections
        };
        let has_project_sections = !project_sections.is_empty();
        let no_section_items = &self.no_section_items;
        let section_items_map = &self.section_items_map;
        let item_rows = &self.item_rows;
        let active_index = self.active_index;
        let active_border = cx.theme().list_active_border;

        v_flex()
            .id("project-items")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &crate::MoveTaskUp, window, cx| {
                this.reorder_active(-1, window, cx);
            }))
            .on_action(cx.listener(|this, _: &crate::MoveTaskDown, window, cx| {
                this.reorder_active(1, window, cx);
            }))
            .relative()
            .size_full()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    for row in this.item_rows.clone() {
                        row.update(cx, |row, cx| row.collapse_if_open(cx));
                    }
                    let had_active = this.active_index.take().is_some();
                    let had_multi = !cx.global::<crate::core::state::ItemSelection>().is_empty();
                    if had_multi {
                        cx.update_global::<crate::core::state::ItemSelection, _>(|sel, _| {
                            sel.clear();
                        });
                    }
                    if had_active || had_multi {
                        cx.notify();
                    }
                }),
            )
            .gap(VisualHierarchy::spacing(4.0))
            .child(
                h_flex()
                    .id("header")
                    .justify_between()
                    .items_center()
                    .px(gpui::px(24.))
                    .pt(gpui::px(18.))
                    .pb(gpui::px(8.))
                    .child(
                        h_flex()
                            .items_center()
                            .gap(VisualHierarchy::spacing(2.0))
                            .child(div().text_xl().font_semibold().child(self.project.name.clone()))
                            .when(!self.item_rows.is_empty(), |this| {
                                this.child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(self.item_rows.len().to_string()),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(VisualHierarchy::spacing(2.0))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .when(has_project_sections, |this| {
                                this.child(
                                    Button::new("add-item-to-section")
                                        .small()
                                        .ghost()
                                        .compact()
                                        .icon(IconName::FolderOpen)
                                        .tooltip(t!("todo.project.add_to_section").to_string())
                                        .dropdown_menu({
                                            let view = view.clone();
                                            let project_id = self.project.id.clone();
                                            move |mut this, window, cx| {
                                                let project_sections: Vec<(String, String)> = cx
                                                    .global::<TodoStore>()
                                                    .sections
                                                    .iter()
                                                    .filter(|s| {
                                                        s.project_id.as_deref() == Some(&project_id)
                                                    })
                                                    .map(|s| (s.name.clone(), s.id.clone()))
                                                    .collect();
                                                for (section_name, section_id) in project_sections {
                                                    this = this.item(
                                                        PopupMenuItem::new(section_name).on_click(
                                                            window.listener_for(
                                                                &view,
                                                                move |this, _, window, cx| {
                                                                    this.show_item_dialog(
                                                                        window,
                                                                        cx,
                                                                        false,
                                                                        Some(section_id.clone()),
                                                                    );
                                                                    cx.notify();
                                                                },
                                                            ),
                                                        ),
                                                    );
                                                }

                                                this
                                            }
                                        }),
                                )
                            })
                            .child(
                                Button::new("section-actions")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::PlusLargeSymbolic)
                                    .tooltip(t!("todo.section.new").to_string())
                                    .on_click({
                                        let view = view.clone();
                                        move |_event, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_section_dialog(window, cx, None, false);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("project-more")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::EllipsisVertical)
                                    .tooltip(t!("todo.more").to_string())
                                    .dropdown_menu({
                                        let view = view.clone();
                                        move |this, window, _cx| {
                                            let view = view.clone();
                                            this.item(
                                                PopupMenuItem::new(
                                                    t!("todo.project.edit").to_string(),
                                                )
                                                .on_click(window.listener_for(
                                                    &view,
                                                    |this, _, window, cx| {
                                                        this.show_project_edit_dialog(window, cx);
                                                        cx.notify();
                                                    },
                                                )),
                                            )
                                            .item(
                                                PopupMenuItem::new(
                                                    t!("todo.project.child_new").to_string(),
                                                )
                                                .on_click(window.listener_for(
                                                    &view,
                                                    |this, _, window, cx| {
                                                        this.show_child_project_dialog(window, cx);
                                                        cx.notify();
                                                    },
                                                )),
                                            )
                                            .separator()
                                            .item(
                                                PopupMenuItem::new(
                                                    t!("todo.project.delete").to_string(),
                                                )
                                                .on_click(window.listener_for(
                                                    &view,
                                                    |this, _, window, cx| {
                                                        this.show_project_delete_dialog(window, cx);
                                                        cx.notify();
                                                    },
                                                )),
                                            )
                                        }
                                    }),
                            ),
                    ),
            )
            .child(board_common::render_batch_bar(view.clone(), cx))
            .child(
                v_flex().flex_1().overflow_y_scrollbar().child(
                    v_flex()
                        .gap(VisualHierarchy::spacing(2.0))
                        .px_4()
                        .pt_1()
                        .pb(board_common::FAB_BOTTOM_PAD)
                        .when(item_rows.is_empty() && !has_project_sections, |this| {
                            this.child(board_renderer::render_empty_placeholder(
                                cx,
                                IconName::FolderOpen,
                                t!("todo.empty.add_tasks").to_string(),
                                t!("todo.empty.add_hint").to_string(),
                            ))
                        })
                        .when(!self.pinned_items.is_empty(), |this| {
                            this.child(board_section(t!("todo.board.pin").to_string()).child(
                                board_renderer::render_item_list(
                                    &self.pinned_items,
                                    item_rows,
                                    active_index,
                                    active_border,
                                    view.clone(),
                                    cx,
                                ),
                            ))
                        })
                        .when(!no_section_items.is_empty(), |this| {
                            if has_project_sections {
                                this.child(board_renderer::render_no_section_block(
                                    no_section_items,
                                    item_rows,
                                    active_index,
                                    active_border,
                                    view.clone(),
                                    true,
                                    cx,
                                ))
                            } else {
                                this.child(board_renderer::render_item_list(
                                    no_section_items,
                                    item_rows,
                                    active_index,
                                    active_border,
                                    view.clone(),
                                    cx,
                                ))
                            }
                        })
                        .children(project_sections.iter().map(|sec| {
                            let items =
                                section_items_map.get(&sec.id).map(|v| v.as_slice()).unwrap_or(&[]);
                            board_renderer::render_section_block(
                                sec.name.clone(),
                                sec.id.clone(),
                                items,
                                item_rows,
                                active_index,
                                active_border,
                                view.clone(),
                                cx,
                            )
                        })),
                ),
            )
            .child(board_common::render_add_task_fab(
                "fab-add-project",
                cx.listener(|this, _, window, cx| {
                    this.show_item_dialog(window, cx, false, None);
                }),
            ))
    }
}
