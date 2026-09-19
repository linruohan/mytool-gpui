use std::{option::Option, sync::Arc};

use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Side, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    sidebar::{Sidebar, SidebarMenu},
    switch::Switch,
    v_flex,
};
use gpui_kit::assets::IconName;
use rust_i18n::t;
use serde::Deserialize;
use todos::entity::{ItemModel, ProjectModel};

use crate::{
    AddLabel, ArchiveProject, BatchCompleteSelected, BatchDeleteSelected, BatchMoveSelected,
    BoardPanel, BoardView, ClearDueDate, ClearFilters, CompletedBoard, DeleteProject, DeleteSection,
    DeleteTask, DeselectAll, DuplicateTask, EditProject, EditSection, EditTask, FilterByLabel,
    FilterByPriority, FilterByProject, GoBack, GoForward, InboxBoard, IndentTask, ItemListItem,
    MoveTaskDown, MoveTaskToProject, MoveTaskUp, NewProject, NewSection, NewTask, NextView,
    OpenHelp, OpenSettings, OutdentTask, PinBoard, PreviousView, ProjectEvent, ProjectItemEvent,
    ProjectItemsPanel, ProjectsPanel, RedoLastTask, RefreshView, ResetZoom, ScheduleNextWeek,
    ScheduleToday, ScheduleTomorrow, ScheduledBoard, SearchTasks, SelectAllTasks, SelectNextTask,
    SelectPreviousTask, SetDueDate, SetPriorityHigh, SetPriorityLow, SetPriorityMedium,
    SetPriorityNone, SetTaskPriority, ShowAllTasks, ShowCompleted, ShowInbox, ShowLabels,
    ShowPinned, ShowScheduled, ShowToday, TodayBoard, ToggleFullscreen, ToggleLabelFavorite,
    ToggleProjectFavorite, ToggleSidebar, ToggleTaskComplete, ToggleTaskPin, UndoLastTask, ZoomIn,
    ZoomOut, play_success_sound,
    todo_state::{NavHistory, NavPlace, TodoPrefs, TodoStore},
    ui::components::{
        DueQuickPreset, apply_due_quick_preset, show_existing_item_dialog,
        show_filter_label_dialog, show_filter_priority_dialog, show_filter_project_dialog,
        show_move_to_project_dialog, show_new_item_dialog, show_set_due_dialog,
        show_todo_help_dialog, show_todo_settings_dialog,
    },
};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = todo_story, no_json)]
pub struct SelectTodo(SharedString);

#[derive(Clone)]
struct ProjectDragPayload {
    project_id: String,
    name: String,
}

impl Render for ProjectDragPayload {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let label = if self.name.is_empty() { self.project_id.clone() } else { self.name.clone() };
        div()
            .px_3()
            .py_1()
            .rounded_md()
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_md()
            .text_sm()
            .max_w(px(220.))
            .child(label)
    }
}

pub struct TodoStory {
    collapsed: bool,
    click_to_open_submenu: bool,
    side: Side,
    focus_handle: gpui::FocusHandle,
    _subscriptions: Vec<Subscription>,
    // 所有看板
    board_panel: Entity<BoardPanel>,
    // projects
    project_panel: Entity<ProjectsPanel>,
    active_project: Option<Arc<ProjectModel>>,
    project_items_panel: Entity<ProjectItemsPanel>,
    search_open: bool,
    search_input: Entity<InputState>,
    nav: NavHistory,
    nav_restoring: bool,
}

impl super::Mytool for TodoStory {
    fn title() -> String {
        t!("todo.story.title").to_string()
    }

    fn description() -> String {
        t!("todo.story.description").to_string()
    }

    fn paddings() -> Pixels {
        px(0.)
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}
impl Focusable for TodoStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl TodoStory {
    pub fn new(_init_story: Option<&str>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let project_panel = ProjectsPanel::view(window, cx);
        let project_items_panel = ProjectItemsPanel::view(window, cx);
        let board_panel = BoardPanel::view(window, cx);
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("todo.search.placeholder").to_string())
        });
        let mut _subscriptions = vec![
            cx.subscribe(&search_input, |_, _, e, cx| {
                if let InputEvent::Change = e {
                    cx.notify();
                }
            }),
            cx.subscribe(&project_panel, |this: &mut Self, _, event: &ProjectEvent, cx| {
                this.project_panel.update(cx, |project_panel, cx| {
                    project_panel.handle_project_event(event, cx);
                });
            }),
            cx.subscribe(
                &project_items_panel,
                |view: &mut Self, _, event: &ProjectItemEvent, cx| {
                    view.project_items_panel.update(cx, |project_items_panel, cx| {
                        project_items_panel.handle_project_item_event(event, cx);
                    });
                },
            ),
            // 监听 TodoStore 的变化，当 active_project 变化时更新 UI
            // 🚀 7.0修复：此观察者保持不变（问题在 InboxBoard 的 observe_global 中）
            cx.observe_global::<TodoStore>(|this, cx| {
                let todo_store = cx.global::<TodoStore>();

                // 检查 active_project 是否变化
                match &todo_store.active_project {
                    Some(active_project) => {
                        // 检查是否与当前 active_project 不同
                        let is_different = this
                            .active_project
                            .as_ref()
                            .map(|p| p.id != active_project.id)
                            .unwrap_or(true);

                        if is_different {
                            tracing::debug!(
                                "TodoStory: 检测到 active_project 变化，切换到项目: {}",
                                active_project.name
                            );

                            // 提前克隆需要的数据，避免借用冲突
                            let active_project_clone = active_project.clone();
                            this.remember_leaving(
                                &NavPlace::Project(active_project_clone.id.clone()),
                                cx,
                            );

                            // 找到新项目在列表中的索引
                            let new_index = todo_store
                                .projects
                                .iter()
                                .position(|p| p.id == active_project_clone.id);

                            // 更新 TodoStory 的 active_project
                            this.active_project = Some(active_project_clone.clone());

                            // 更新 project_panel 的 active_index
                            this.project_panel.update(cx, |panel, cx| {
                                panel.update_active_index(new_index);
                                cx.notify();
                            });

                            // 更新 project_items_panel
                            this.project_items_panel.update(cx, |panel, cx| {
                                panel.set_project(active_project_clone.clone(), cx);
                                cx.notify();
                            });

                            // 清除 board_panel 的选中状态
                            this.board_panel.update(cx, |panel, cx| {
                                panel.update_active_index(None);
                                cx.notify();
                            });

                            cx.notify();
                        }
                    },
                    None => {
                        // active_project 为 None，表示所有项目都被删除
                        // 检查之前是否有活跃项目
                        if this.active_project.is_some() {
                            tracing::debug!("TodoStory: 所有项目已删除，切换到 Inbox 视图");

                            this.remember_leaving(&NavPlace::Board(0), cx);
                            // 清除 TodoStory 的 active_project
                            this.active_project = None;

                            // 清除 project_panel 的 active_index
                            this.project_panel.update(cx, |panel, cx| {
                                panel.update_active_index(None);
                                cx.notify();
                            });

                            // 切换到 Inbox 视图（board_panel 的第一个视图，索引为 0）
                            this.board_panel.update(cx, |panel, cx| {
                                panel.update_active_index(Some(0));
                                cx.notify();
                            });

                            cx.notify();
                        }
                    },
                }
            }),
        ];
        let startup = (cx.global::<TodoPrefs>().startup_board as usize).min(5);
        board_panel.update(cx, |panel, _| {
            panel.update_active_index(Some(startup));
        });
        Self {
            collapsed: false,
            active_project: None,
            focus_handle: cx.focus_handle(),
            _subscriptions,
            board_panel,
            project_panel,
            project_items_panel,
            click_to_open_submenu: false,
            side: Side::Left,
            search_open: false,
            search_input,
            nav: NavHistory::default(),
            nav_restoring: false,
        }
    }

    fn current_place(&self, cx: &App) -> NavPlace {
        if let Some(project) = &self.active_project {
            NavPlace::Project(project.id.clone())
        } else {
            NavPlace::Board(self.board_panel.read(cx).active_index.unwrap_or(0))
        }
    }

    fn remember_leaving(&mut self, arriving: &NavPlace, cx: &App) {
        if self.nav_restoring {
            return;
        }
        let leaving = self.current_place(cx);
        self.nav.record_leaving(leaving, arriving);
    }

    fn apply_nav(&mut self, place: NavPlace, cx: &mut Context<Self>) {
        self.nav_restoring = true;
        match place {
            NavPlace::Board(index) => self.show_board(index, cx),
            NavPlace::Project(id) => {
                if let Some(project) = cx.global::<TodoStore>().get_project(&id) {
                    self.activate_project(project, cx);
                } else {
                    self.show_board(0, cx);
                }
            },
        }
        self.nav_restoring = false;
    }

    fn show_board(&mut self, index: usize, cx: &mut Context<Self>) {
        self.remember_leaving(&NavPlace::Board(index), cx);
        self.active_project = None;
        self.project_panel.update(cx, |panel, cx| {
            panel.update_active_index(None);
            cx.notify();
        });
        self.board_panel.update(cx, |panel, cx| {
            panel.update_active_index(Some(index));
            cx.notify();
        });
        cx.update_global::<TodoStore, _>(|store, _| {
            store.set_active_project(None);
        });
        cx.notify();
    }

    fn on_new_task(&mut self, _: &NewTask, window: &mut Window, cx: &mut Context<Self>) {
        let mut item = ItemModel::default();
        if let Some(project) = &self.active_project {
            item.project_id = Some(project.id.clone());
        }
        show_new_item_dialog(window, cx, item);
    }

    fn on_search_tasks(&mut self, _: &SearchTasks, window: &mut Window, cx: &mut Context<Self>) {
        self.open_search_with("", window, cx);
    }

    fn open_search_with(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = true;
        self.search_input.update(cx, |input, cx| {
            input.set_value(query, window, cx);
        });
        self.search_input.read(cx).focus_handle(cx).focus(window, cx);
        cx.notify();
    }

    fn activate_project(&mut self, project: Arc<ProjectModel>, cx: &mut Context<Self>) {
        self.remember_leaving(&NavPlace::Project(project.id.clone()), cx);
        self.active_project = Some(project.clone());
        let store_ix = cx.global::<TodoStore>().projects.iter().position(|p| p.id == project.id);
        self.project_panel.update(cx, |panel, cx| {
            panel.update_active_index(store_ix);
            cx.notify();
        });
        self.project_items_panel.update(cx, |panel, cx| {
            panel.set_project(project.clone(), cx);
            cx.notify();
        });
        self.board_panel.update(cx, |panel, cx| {
            panel.update_active_index(None);
            cx.notify();
        });
        cx.update_global::<TodoStore, _>(|store, _| {
            store.set_active_project(Some(project));
        });
        cx.notify();
    }

    fn jump_to_item(&mut self, item: Arc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = false;
        if item.checked {
            self.show_board(5, cx);
        } else if let Some(pid) = item.project_id.as_deref().filter(|id| !id.is_empty()) {
            if let Some(project) = cx.global::<TodoStore>().get_project(pid) {
                self.activate_project(project, cx);
            } else {
                self.show_board(0, cx);
            }
        } else if item.pinned {
            self.show_board(4, cx);
        } else if item.is_due_today() {
            self.show_board(1, cx);
        } else if item.due_date().is_some() {
            self.show_board(2, cx);
        } else {
            self.show_board(0, cx);
        }
        let id = item.id.clone();
        cx.update_global::<crate::core::state::ItemSelection, _>(|sel, _| sel.select_only(id));
        show_existing_item_dialog(window, cx, item);
        cx.notify();
    }

    fn on_deselect(&mut self, _: &DeselectAll, _: &mut Window, cx: &mut Context<Self>) {
        if self.search_open {
            self.search_open = false;
        }
        cx.update_global::<crate::core::state::ItemSelection, _>(|sel, _| sel.clear());
        cx.notify();
    }

    fn on_select_all(&mut self, _: &SelectAllTasks, _: &mut Window, cx: &mut Context<Self>) {
        if self.search_open {
            return;
        }
        let ids = self.visible_item_ids(cx).into_iter().collect();
        cx.update_global::<crate::core::state::ItemSelection, _>(|sel, _| sel.set_ids(ids));
        cx.notify();
    }

    fn visible_item_ids(&self, cx: &App) -> Vec<String> {
        if self.search_open {
            let q = self.search_input.read(cx).value().to_string();
            return cx
                .global::<TodoStore>()
                .search_items(&q)
                .into_iter()
                .map(|item| item.id.clone())
                .collect();
        }
        let store = cx.global::<TodoStore>();
        let cache = cx.global::<crate::core::state::QueryCache>();
        let items = if let Some(ix) = self.board_panel.read(cx).active_index {
            match ix {
                0 => store.inbox_items_cached(cache).as_ref().clone(),
                1 => store.today_items_cached(cache).as_ref().clone(),
                2 => store.scheduled_items_cached(cache).as_ref().clone(),
                4 => store.pinned_items_cached(cache).as_ref().clone(),
                5 => store.completed_items_cached(cache).as_ref().clone(),
                _ => Vec::new(),
            }
        } else if let Some(project) = &self.active_project {
            store.items_by_project(&project.id).to_vec()
        } else {
            Vec::new()
        };
        items.into_iter().map(|item| item.id.clone()).collect()
    }

    fn step_selection(&mut self, delta: i32, cx: &mut Context<Self>) {
        let ids = self.visible_item_ids(cx);
        let current =
            cx.global::<crate::core::state::ItemSelection>().primary_id().map(str::to_string);
        let Some(next) = crate::todo_state::step_visible_id(&ids, current.as_deref(), delta) else {
            return;
        };
        cx.update_global::<crate::core::state::ItemSelection, _>(|sel, _| sel.select_only(next));
        cx.notify();
    }

    fn on_batch_complete(
        &mut self,
        _: &BatchCompleteSelected,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let n = crate::todo_actions::batch_complete_selected(cx);
        if n > 0 {
            window.push_notification(t!("todo.batch.completed_n", count => n).to_string(), cx);
        }
        cx.notify();
    }

    fn on_batch_delete(
        &mut self,
        _: &BatchDeleteSelected,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let n = crate::todo_actions::batch_delete_selected(cx);
        if n > 0 {
            window.push_notification(t!("todo.batch.deleted_n", count => n).to_string(), cx);
        }
        cx.notify();
    }

    fn on_open_settings(&mut self, _: &OpenSettings, window: &mut Window, cx: &mut Context<Self>) {
        show_todo_settings_dialog(window, cx);
    }

    fn on_open_help(&mut self, _: &OpenHelp, window: &mut Window, cx: &mut Context<Self>) {
        show_todo_help_dialog(window, cx);
    }

    fn on_undo_last(&mut self, _: &UndoLastTask, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_open {
            return;
        }
        if let Some(msg) = crate::todo_actions::undo_last_task(cx) {
            window.push_notification(msg, cx);
        }
        cx.notify();
    }

    fn on_redo_last(&mut self, _: &RedoLastTask, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_open {
            return;
        }
        if let Some(msg) = crate::todo_actions::redo_last_task(cx) {
            window.push_notification(msg, cx);
        }
        cx.notify();
    }

    fn primary_item(&self, cx: &App) -> Option<Arc<ItemModel>> {
        let id = cx.global::<crate::core::state::ItemSelection>().primary_id()?.to_string();
        cx.global::<TodoStore>().get_item(&id)
    }

    fn on_edit_task(&mut self, _: &EditTask, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = self.primary_item(cx) {
            show_existing_item_dialog(window, cx, item);
        }
    }

    fn on_delete_task(&mut self, _: &DeleteTask, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.primary_item(cx) else {
            return;
        };
        if !cx.global::<crate::core::state::TodoPrefs>().confirm_on_delete {
            crate::todo_actions::delete_item_optimistic(item, cx);
            window.push_notification(t!("todo.item.deleted").to_string(), cx);
            return;
        }
        crate::show_item_delete_dialog(window, cx, &t!("todo.item.delete_confirm"), move |cx| {
            crate::todo_actions::delete_item_optimistic(item.clone(), cx);
        });
    }

    fn on_toggle_complete(
        &mut self,
        _: &ToggleTaskComplete,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(item) = self.primary_item(cx) {
            crate::todo_actions::complete_item_optimistic(item.clone(), !item.checked, cx);
            window.push_notification(
                if item.checked {
                    t!("todo.item.unfinished").to_string()
                } else {
                    t!("todo.item.finished").to_string()
                },
                cx,
            );
        }
    }

    fn on_duplicate_task(
        &mut self,
        _: &DuplicateTask,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = self.primary_item(cx) else {
            return;
        };
        let mut copy = (*item).clone();
        copy.id = uuid::Uuid::new_v4().to_string();
        copy.content = t!("todo.section.copy_name", name => copy.content.as_str()).to_string();
        copy.checked = false;
        copy.completed_at = None;
        crate::todo_actions::add_item_optimistic(Arc::new(copy), cx);
        window.push_notification(t!("todo.item.copied").to_string(), cx);
    }

    fn on_toggle_pin(&mut self, _: &ToggleTaskPin, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = self.primary_item(cx) {
            let pinned = !item.pinned;
            crate::todo_actions::set_item_pinned_optimistic(item, pinned, cx);
            window.push_notification(
                if pinned {
                    t!("todo.item.pinned").to_string()
                } else {
                    t!("todo.item.unpinned").to_string()
                },
                cx,
            );
        }
    }

    fn on_set_due_date(&mut self, _: &SetDueDate, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = self.primary_item(cx) {
            show_set_due_dialog(window, cx, item);
        }
    }

    fn selected_or_primary_items(&self, cx: &App) -> Vec<Arc<ItemModel>> {
        let ids: Vec<String> =
            cx.global::<crate::core::state::ItemSelection>().ids().iter().cloned().collect();
        let store = cx.global::<TodoStore>();
        let mut items: Vec<_> = ids.iter().filter_map(|id| store.get_item(id)).collect();
        if items.is_empty() {
            if let Some(item) = self.primary_item(cx) {
                items.push(item);
            }
        }
        items
    }

    fn apply_due_preset(
        &mut self,
        preset: DueQuickPreset,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let items = self.selected_or_primary_items(cx);
        let n = apply_due_quick_preset(items, preset, cx);
        if n == 0 {
            return;
        }
        let msg = if n > 1 {
            t!("todo.today.rescheduled_n", count => n).to_string()
        } else {
            match preset {
                DueQuickPreset::Today => t!("todo.due.scheduled_today").to_string(),
                DueQuickPreset::Tomorrow => t!("todo.due.scheduled_tomorrow").to_string(),
                DueQuickPreset::NextWeek => t!("todo.due.scheduled_next_week").to_string(),
                DueQuickPreset::Clear => t!("todo.due.cleared").to_string(),
            }
        };
        window.push_notification(msg, cx);
        cx.notify();
    }

    fn on_schedule_today(
        &mut self,
        _: &ScheduleToday,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_due_preset(DueQuickPreset::Today, window, cx);
    }

    fn on_schedule_tomorrow(
        &mut self,
        _: &ScheduleTomorrow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_due_preset(DueQuickPreset::Tomorrow, window, cx);
    }

    fn on_schedule_next_week(
        &mut self,
        _: &ScheduleNextWeek,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_due_preset(DueQuickPreset::NextWeek, window, cx);
    }

    fn on_clear_due_date(&mut self, _: &ClearDueDate, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_due_preset(DueQuickPreset::Clear, window, cx);
    }

    fn on_select_previous(
        &mut self,
        _: &SelectPreviousTask,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.step_selection(-1, cx);
    }

    fn on_select_next(&mut self, _: &SelectNextTask, _: &mut Window, cx: &mut Context<Self>) {
        self.step_selection(1, cx);
    }

    fn on_toggle_sidebar(&mut self, _: &ToggleSidebar, _: &mut Window, cx: &mut Context<Self>) {
        self.collapsed = !self.collapsed;
        cx.notify();
    }

    fn on_toggle_fullscreen(
        &mut self,
        _: &ToggleFullscreen,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.toggle_fullscreen();
    }

    fn on_new_project(&mut self, _: &NewProject, window: &mut Window, cx: &mut Context<Self>) {
        self.open_new_project(window, cx);
    }

    fn current_section_id(&self, cx: &App) -> Option<String> {
        let store = cx.global::<TodoStore>();
        if let Some(sid) = self
            .primary_item(cx)
            .and_then(|item| item.section_id.clone())
            .filter(|id| !id.is_empty())
        {
            if let Some(section) = store.get_section(&sid) {
                if !section.is_archived && !section.is_deleted && !section.hidded {
                    return Some(sid);
                }
            }
        }
        let mut sections = if let Some(project) = &self.active_project {
            store.sections_for_project(&project.id)
        } else if self.board_panel.read(cx).active_index == Some(0) {
            store
                .sections
                .iter()
                .filter(|s| s.project_id.as_deref().unwrap_or("").is_empty())
                .cloned()
                .collect()
        } else {
            return None;
        };
        sections.retain(|s| !s.is_archived && !s.is_deleted && !s.hidded);
        crate::todo_state::sort_sections_by_order(&mut sections);
        sections.first().map(|s| s.id.clone())
    }

    fn with_inbox_board(
        &self,
        cx: &mut Context<Self>,
        f: impl FnOnce(Entity<InboxBoard>, &mut Context<Self>),
    ) {
        if self.board_panel.read(cx).active_index != Some(0) {
            return;
        }
        let Some(container) = self.board_panel.read(cx).boards.first().cloned() else {
            return;
        };
        let Some(board) = container.read(cx).inner_board() else {
            return;
        };
        if let Ok(inbox) = board.downcast::<InboxBoard>() {
            f(inbox, cx);
        }
    }

    fn on_new_section(&mut self, _: &NewSection, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_project.is_some() {
            self.project_items_panel.update(cx, |panel, cx| {
                panel.show_section_dialog(window, cx, None, false);
            });
            return;
        }
        self.with_inbox_board(cx, |inbox, cx| {
            inbox.update(cx, |panel, cx| {
                panel.show_section_dialog(window, cx, None, false);
            });
        });
    }

    fn on_edit_section(&mut self, _: &EditSection, window: &mut Window, cx: &mut Context<Self>) {
        let Some(section_id) = self.current_section_id(cx) else {
            return;
        };
        if self.active_project.is_some() {
            self.project_items_panel.update(cx, |panel, cx| {
                panel.show_section_dialog(window, cx, Some(section_id), true);
            });
            return;
        }
        self.with_inbox_board(cx, |inbox, cx| {
            inbox.update(cx, |panel, cx| {
                panel.show_section_dialog(window, cx, Some(section_id), true);
            });
        });
    }

    fn on_delete_section(
        &mut self,
        _: &DeleteSection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(section_id) = self.current_section_id(cx) else {
            return;
        };
        if self.active_project.is_some() {
            self.project_items_panel.update(cx, |panel, cx| {
                panel.show_section_delete_dialog(window, cx, section_id);
            });
            return;
        }
        self.with_inbox_board(cx, |inbox, cx| {
            inbox.update(cx, |panel, cx| {
                panel.show_section_delete_dialog(window, cx, section_id);
            });
        });
    }

    fn on_edit_project(&mut self, _: &EditProject, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_project.is_none() {
            return;
        }
        self.project_items_panel.update(cx, |panel, cx| {
            panel.show_project_edit_dialog(window, cx);
        });
    }

    fn on_delete_project(
        &mut self,
        _: &DeleteProject,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_project.is_none() {
            return;
        }
        self.project_items_panel.update(cx, |panel, cx| {
            panel.show_project_delete_dialog(window, cx);
        });
    }

    fn on_archive_project(
        &mut self,
        _: &ArchiveProject,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.active_project.clone() else {
            return;
        };
        let mut archived = (*project).clone();
        archived.is_archived = true;
        crate::todo_actions::update_project(Arc::new(archived), cx);
        window.push_notification(t!("todo.project.archived").to_string(), cx);
        self.show_board(0, cx);
    }

    fn on_toggle_project_favorite(
        &mut self,
        _: &ToggleProjectFavorite,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(id) = self.active_project.as_ref().map(|p| p.id.clone()) else {
            return;
        };
        let Some(project) = cx.global::<TodoStore>().get_project(&id) else {
            return;
        };
        let next = !project.is_favorite;
        crate::todo_actions::set_project_favorite(project, next, cx);
        window.push_notification(
            if next {
                t!("todo.project.favorited").to_string()
            } else {
                t!("todo.project.unfavorited").to_string()
            },
            cx,
        );
        cx.notify();
    }

    fn on_toggle_label_favorite(
        &mut self,
        _: &ToggleLabelFavorite,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(board) = self.board_panel.read(cx).labels_board(cx) else {
            return;
        };
        let panel = board.read(cx).labels_panel.clone();
        panel.update(cx, |panel, cx| {
            panel.toggle_selected_favorite(window, cx);
        });
        cx.notify();
    }

    fn on_show_inbox(&mut self, _: &ShowInbox, _: &mut Window, cx: &mut Context<Self>) {
        self.show_board(0, cx);
    }

    fn on_show_today(&mut self, _: &ShowToday, _: &mut Window, cx: &mut Context<Self>) {
        self.show_board(1, cx);
    }

    fn on_show_scheduled(&mut self, _: &ShowScheduled, _: &mut Window, cx: &mut Context<Self>) {
        self.show_board(2, cx);
    }

    fn on_show_labels(&mut self, _: &ShowLabels, _: &mut Window, cx: &mut Context<Self>) {
        self.show_board(3, cx);
    }

    fn on_show_pinned(&mut self, _: &ShowPinned, _: &mut Window, cx: &mut Context<Self>) {
        self.show_board(4, cx);
    }

    fn on_show_completed(&mut self, _: &ShowCompleted, _: &mut Window, cx: &mut Context<Self>) {
        self.show_board(5, cx);
    }

    fn on_add_label(&mut self, _: &AddLabel, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = self.primary_item(cx) {
            show_existing_item_dialog(window, cx, item);
        }
    }

    fn on_set_priority(
        &mut self,
        _: &SetTaskPriority,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current = self.primary_item(cx).map(|item| item.priority.unwrap_or(4)).unwrap_or(4);
        let next = match current {
            1 => 2,
            2 => 3,
            3 => 4,
            _ => 1,
        };
        self.apply_priority(next, window, cx);
    }

    fn on_set_priority_high(
        &mut self,
        _: &SetPriorityHigh,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_priority(1, window, cx);
    }

    fn on_set_priority_medium(
        &mut self,
        _: &SetPriorityMedium,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_priority(2, window, cx);
    }

    fn on_set_priority_low(
        &mut self,
        _: &SetPriorityLow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_priority(3, window, cx);
    }

    fn on_set_priority_none(
        &mut self,
        _: &SetPriorityNone,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_priority(4, window, cx);
    }

    fn apply_priority(&mut self, next: i32, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.selected_or_primary_items(cx);
        let updated: Vec<_> = items
            .into_iter()
            .filter(|item| item.priority.unwrap_or(4) != next)
            .map(|item| {
                let mut updated = (*item).clone();
                updated.priority = Some(next);
                Arc::new(updated)
            })
            .collect();
        if updated.is_empty() {
            return;
        }
        let label = match next {
            1 => t!("todo.priority.high").to_string(),
            2 => t!("todo.priority.medium").to_string(),
            3 => t!("todo.priority.low").to_string(),
            _ => t!("todo.priority.none").to_string(),
        };
        if updated.len() == 1 {
            crate::todo_actions::update_item_optimistic(updated.into_iter().next().unwrap(), cx);
        } else {
            crate::todo_actions::batch_update_items(updated, cx);
        }
        window.push_notification(
            t!("todo.notify.set_priority", label => label.as_str()).to_string(),
            cx,
        );
        cx.notify();
    }

    fn on_move_to_project(
        &mut self,
        _: &MoveTaskToProject,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(item) = self.primary_item(cx) {
            show_move_to_project_dialog(window, cx, vec![item]);
        }
    }

    fn on_batch_move_selected(
        &mut self,
        _: &BatchMoveSelected,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ids: Vec<String> =
            cx.global::<crate::core::state::ItemSelection>().ids().iter().cloned().collect();
        let store = cx.global::<TodoStore>();
        let mut items: Vec<_> = ids.iter().filter_map(|id| store.get_item(id)).collect();
        if items.is_empty() {
            if let Some(item) = self.primary_item(cx) {
                items.push(item);
            }
        }
        if items.is_empty() {
            return;
        }
        show_move_to_project_dialog(window, cx, items);
    }

    fn reorder_active_task(&mut self, delta: i32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.primary_item(cx) else {
            return;
        };
        let id = item.id.clone();
        if self.active_project.is_some() {
            self.project_items_panel.update(cx, |panel, cx| {
                panel.reorder_by_item_id(&id, delta, window, cx);
            });
            return;
        }
        let Some(ix) = self.board_panel.read(cx).active_index else {
            return;
        };
        let Some(container) = self.board_panel.read(cx).boards.get(ix).cloned() else {
            return;
        };
        let Some(board) = container.read(cx).inner_board() else {
            return;
        };
        if let Ok(view) = board.clone().downcast::<InboxBoard>() {
            view.update(cx, |panel, cx| panel.reorder_by_item_id(&id, delta, window, cx));
            return;
        }
        if let Ok(view) = board.clone().downcast::<TodayBoard>() {
            view.update(cx, |panel, cx| panel.reorder_by_item_id(&id, delta, window, cx));
            return;
        }
        if let Ok(view) = board.clone().downcast::<ScheduledBoard>() {
            view.update(cx, |panel, cx| panel.reorder_by_item_id(&id, delta, window, cx));
            return;
        }
        if let Ok(view) = board.clone().downcast::<PinBoard>() {
            view.update(cx, |panel, cx| panel.reorder_by_item_id(&id, delta, window, cx));
            return;
        }
        if let Ok(view) = board.downcast::<CompletedBoard>() {
            view.update(cx, |panel, cx| panel.reorder_by_item_id(&id, delta, window, cx));
        }
    }

    fn refresh_active_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_project.is_some() {
            self.project_items_panel.update(cx, |panel, cx| {
                panel.request_store_refresh(window, cx);
            });
            return;
        }
        let Some(ix) = self.board_panel.read(cx).active_index else {
            return;
        };
        let Some(container) = self.board_panel.read(cx).boards.get(ix).cloned() else {
            return;
        };
        let Some(board) = container.read(cx).inner_board() else {
            return;
        };
        if let Ok(view) = board.clone().downcast::<InboxBoard>() {
            view.update(cx, |panel, cx| panel.request_store_refresh(window, cx));
            return;
        }
        if let Ok(view) = board.clone().downcast::<TodayBoard>() {
            view.update(cx, |panel, cx| panel.request_store_refresh(window, cx));
            return;
        }
        if let Ok(view) = board.clone().downcast::<ScheduledBoard>() {
            view.update(cx, |panel, cx| panel.request_store_refresh(window, cx));
            return;
        }
        if let Ok(view) = board.clone().downcast::<PinBoard>() {
            view.update(cx, |panel, cx| panel.request_store_refresh(window, cx));
            return;
        }
        if let Ok(view) = board.downcast::<CompletedBoard>() {
            view.update(cx, |panel, cx| panel.request_store_refresh(window, cx));
        }
    }

    fn on_move_task_up(&mut self, _: &MoveTaskUp, window: &mut Window, cx: &mut Context<Self>) {
        self.reorder_active_task(-1, window, cx);
    }

    fn on_move_task_down(&mut self, _: &MoveTaskDown, window: &mut Window, cx: &mut Context<Self>) {
        self.reorder_active_task(1, window, cx);
    }

    fn on_indent_task(&mut self, _: &IndentTask, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_open {
            return;
        }
        let Some(item) = self.primary_item(cx) else {
            return;
        };
        if crate::indent_item(&item.id, cx) {
            self.refresh_active_list(window, cx);
        }
    }

    fn on_outdent_task(&mut self, _: &OutdentTask, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_open {
            return;
        }
        let Some(item) = self.primary_item(cx) else {
            return;
        };
        if crate::outdent_item(&item.id, cx) {
            self.refresh_active_list(window, cx);
        }
    }

    fn on_next_view(&mut self, _: &NextView, _: &mut Window, cx: &mut Context<Self>) {
        self.cycle_board(1, cx);
    }

    fn on_previous_view(&mut self, _: &PreviousView, _: &mut Window, cx: &mut Context<Self>) {
        self.cycle_board(-1, cx);
    }

    fn on_go_back(&mut self, _: &GoBack, _: &mut Window, cx: &mut Context<Self>) {
        let current = self.current_place(cx);
        if let Some(dest) = self.nav.go_back(current) {
            self.apply_nav(dest, cx);
        }
    }

    fn on_go_forward(&mut self, _: &GoForward, _: &mut Window, cx: &mut Context<Self>) {
        let current = self.current_place(cx);
        if let Some(dest) = self.nav.go_forward(current) {
            self.apply_nav(dest, cx);
        }
    }

    fn cycle_board(&mut self, delta: i32, cx: &mut Context<Self>) {
        let current = self.board_panel.read(cx).active_index.unwrap_or(0);
        let next = (current as i32 + delta).rem_euclid(6) as usize;
        self.show_board(next, cx);
    }

    fn on_filter_label(&mut self, _: &FilterByLabel, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity();
        show_filter_label_dialog(window, cx, move |name, window, cx| {
            view.update(cx, |this, cx| {
                this.open_search_with(&name, window, cx);
            });
        });
    }

    fn on_filter_project(
        &mut self,
        _: &FilterByProject,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        show_filter_project_dialog(window, cx, {
            let view = cx.entity();
            move |project, _window, cx| {
                view.update(cx, |this, cx| match project {
                    Some(project) => this.activate_project(project, cx),
                    None => this.show_board(0, cx),
                });
            }
        });
    }

    fn on_filter_priority(
        &mut self,
        _: &FilterByPriority,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity();
        show_filter_priority_dialog(window, cx, move |token, window, cx| {
            view.update(cx, |this, cx| {
                this.open_search_with(token, window, cx);
            });
        });
    }

    fn on_clear_filters(&mut self, _: &ClearFilters, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = false;
        self.search_input.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
        cx.notify();
    }

    fn on_show_all_tasks(&mut self, _: &ShowAllTasks, window: &mut Window, cx: &mut Context<Self>) {
        self.on_clear_filters(&ClearFilters, window, cx);
        cx.update_global::<crate::core::state::ItemSelection, _>(|sel, _| sel.clear());
        self.show_board(0, cx);
    }

    fn on_refresh_view(&mut self, _: &RefreshView, window: &mut Window, cx: &mut Context<Self>) {
        window.push_notification(t!("todo.notify.refreshed").to_string(), cx);
        cx.notify();
    }

    fn bump_ui_scale(&mut self, delta: f32, window: &mut Window, cx: &mut Context<Self>) {
        cx.update_global::<TodoPrefs, _>(|prefs, _| {
            prefs.apply_ui_scale_delta(delta);
        });
        window.set_rem_size(px(cx.global::<TodoPrefs>().rem_px()));
        cx.notify();
    }

    fn on_zoom_in(&mut self, _: &ZoomIn, window: &mut Window, cx: &mut Context<Self>) {
        self.bump_ui_scale(0.1, window, cx);
    }

    fn on_zoom_out(&mut self, _: &ZoomOut, window: &mut Window, cx: &mut Context<Self>) {
        self.bump_ui_scale(-0.1, window, cx);
    }

    fn on_reset_zoom(&mut self, _: &ResetZoom, window: &mut Window, cx: &mut Context<Self>) {
        cx.update_global::<TodoPrefs, _>(|prefs, _| {
            prefs.ui_scale = 1.0;
            prefs.save();
        });
        window.set_rem_size(px(cx.global::<TodoPrefs>().rem_px()));
        cx.notify();
    }

    #[allow(unused)]
    fn render_content(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_3().child(
            h_flex()
                .gap_3()
                .child(
                    Switch::new("side")
                        .label("Placement Right")
                        .checked(self.side.is_right())
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.side = if *checked { Side::Right } else { Side::Left };
                            cx.notify();
                        })),
                )
                .child(
                    Switch::new("click-to-open")
                        .checked(self.click_to_open_submenu)
                        .label("Click to open submenu")
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.click_to_open_submenu = *checked;
                            cx.notify();
                        })),
                ),
        )
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(Some(""), window, cx))
    }

    fn open_new_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = play_success_sound();
        self.project_panel.update(cx, |project_panel, cx| {
            project_panel.open_project_dialog(Arc::new(ProjectModel::default()), window, cx);
            cx.notify();
        });
    }

    fn add_project(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.open_new_project(window, cx);
    }
}
impl Render for TodoStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let board_panel = self.board_panel.read(cx);
        let boards = board_panel.boards.clone();
        let board_active_index = board_panel.active_index;
        let project_list = cx.global::<TodoStore>().projects_for_sidebar();
        let _view = cx.entity();
        let search_query = self.search_input.read(cx).value().to_string();
        let search_hits = if self.search_open {
            cx.global::<TodoStore>().search_items(&search_query)
        } else {
            Vec::new()
        };

        let mut content = div().id("todos").flex_1().min_h_0().overflow_hidden();

        if let Some(active_ix) = board_active_index {
            if let Some(board_view) = boards.get(active_ix) {
                content = content.child(board_view.clone());
            } else {
                content = content.child(Empty);
            }
        } else {
            content = content.child(self.project_items_panel.clone());
        }

        h_flex()
            .id("todo-story")
            .key_context("TodoStory")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_new_task))
            .on_action(cx.listener(Self::on_search_tasks))
            .on_action(cx.listener(Self::on_deselect))
            .on_action(cx.listener(Self::on_select_all))
            .on_action(cx.listener(Self::on_batch_complete))
            .on_action(cx.listener(Self::on_batch_delete))
            .on_action(cx.listener(Self::on_open_settings))
            .on_action(cx.listener(Self::on_open_help))
            .on_action(cx.listener(Self::on_undo_last))
            .on_action(cx.listener(Self::on_redo_last))
            .on_action(cx.listener(Self::on_edit_task))
            .on_action(cx.listener(Self::on_delete_task))
            .on_action(cx.listener(Self::on_toggle_complete))
            .on_action(cx.listener(Self::on_duplicate_task))
            .on_action(cx.listener(Self::on_toggle_pin))
            .on_action(cx.listener(Self::on_set_due_date))
            .on_action(cx.listener(Self::on_schedule_today))
            .on_action(cx.listener(Self::on_schedule_tomorrow))
            .on_action(cx.listener(Self::on_schedule_next_week))
            .on_action(cx.listener(Self::on_clear_due_date))
            .on_action(cx.listener(Self::on_select_previous))
            .on_action(cx.listener(Self::on_select_next))
            .on_action(cx.listener(Self::on_toggle_sidebar))
            .on_action(cx.listener(Self::on_toggle_fullscreen))
            .on_action(cx.listener(Self::on_new_project))
            .on_action(cx.listener(Self::on_new_section))
            .on_action(cx.listener(Self::on_edit_section))
            .on_action(cx.listener(Self::on_delete_section))
            .on_action(cx.listener(Self::on_edit_project))
            .on_action(cx.listener(Self::on_delete_project))
            .on_action(cx.listener(Self::on_archive_project))
            .on_action(cx.listener(Self::on_toggle_project_favorite))
            .on_action(cx.listener(Self::on_toggle_label_favorite))
            .on_action(cx.listener(Self::on_show_inbox))
            .on_action(cx.listener(Self::on_show_today))
            .on_action(cx.listener(Self::on_show_scheduled))
            .on_action(cx.listener(Self::on_show_labels))
            .on_action(cx.listener(Self::on_show_pinned))
            .on_action(cx.listener(Self::on_show_completed))
            .on_action(cx.listener(Self::on_add_label))
            .on_action(cx.listener(Self::on_set_priority))
            .on_action(cx.listener(Self::on_set_priority_high))
            .on_action(cx.listener(Self::on_set_priority_medium))
            .on_action(cx.listener(Self::on_set_priority_low))
            .on_action(cx.listener(Self::on_set_priority_none))
            .on_action(cx.listener(Self::on_move_to_project))
            .on_action(cx.listener(Self::on_batch_move_selected))
            .on_action(cx.listener(Self::on_move_task_up))
            .on_action(cx.listener(Self::on_move_task_down))
            .on_action(cx.listener(Self::on_indent_task))
            .on_action(cx.listener(Self::on_outdent_task))
            .on_action(cx.listener(Self::on_next_view))
            .on_action(cx.listener(Self::on_previous_view))
            .on_action(cx.listener(Self::on_go_back))
            .on_action(cx.listener(Self::on_go_forward))
            .on_action(cx.listener(Self::on_filter_label))
            .on_action(cx.listener(Self::on_filter_project))
            .on_action(cx.listener(Self::on_filter_priority))
            .on_action(cx.listener(Self::on_clear_filters))
            .on_action(cx.listener(Self::on_show_all_tasks))
            .on_action(cx.listener(Self::on_refresh_view))
            .on_action(cx.listener(Self::on_zoom_in))
            .on_action(cx.listener(Self::on_zoom_out))
            .on_action(cx.listener(Self::on_reset_zoom))
            .size_full()
            .bg(cx.theme().background)
            .child(
                Sidebar::<SidebarMenu>::new("sidebar-story")
                    .side(self.side)
                    .collapsed(self.collapsed)
                    .w(px(248.))
                    .gap_0()
                    .p_2()
                    .board(
                        v_flex()
                            .w_full()
                            .gap_1()
                            .child(self.board_panel.clone())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .px_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(t!("todo.sidebar.this_computer").to_string()),
                                    )
                                    .child(
                                        Button::new("add-project")
                                            .small()
                                            .ghost()
                                            .compact()
                                            .icon(IconName::Plus)
                                            .tooltip(t!("todo.project.new").to_string())
                                            .on_click(cx.listener(|this, ev, window, cx| {
                                                this.add_project(ev, window, cx);
                                            })),
                                    ),
                            )
                            .when(project_list.is_empty(), |this| {
                                this.child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(t!("todo.project.empty").to_string()),
                                )
                            })
                            .children(project_list.iter().map(|(project, nested)| {
                                let count = cx
                                    .global::<TodoStore>()
                                    .items_by_project(&project.id)
                                    .iter()
                                    .filter(|item| !item.checked)
                                    .count();
                                let active = self
                                    .active_project
                                    .as_ref()
                                    .is_some_and(|p| p.id == project.id);
                                let story = project.clone();
                                let drag_id = project.id.clone();
                                let drop_id = project.id.clone();
                                let can_drop_id = project.id.clone();
                                let drag_name = project.name.clone();
                                let has_children =
                                    cx.global::<TodoStore>().has_child_projects(&project.id);
                                let collapsed = project.collapsed;
                                let collapse_id = project.id.clone();
                                let favorite = project.is_favorite;
                                let favorite_id = project.id.clone();
                                let favorite_project = project.clone();
                                h_flex()
                                    .id(SharedString::from(format!(
                                        "sidebar-project-{}",
                                        project.id
                                    )))
                                    .w_full()
                                    .h_7()
                                    .px_2()
                                    .when(*nested, |this| this.pl(px(22.)))
                                    .items_center()
                                    .justify_between()
                                    .rounded(cx.theme().radius)
                                    .text_sm()
                                    .when(active, |this| {
                                        this.bg(cx.theme().sidebar_accent)
                                            .text_color(cx.theme().sidebar_accent_foreground)
                                    })
                                    .hover(|this| this.bg(cx.theme().sidebar_accent))
                                    .on_drag(
                                        ProjectDragPayload { project_id: drag_id, name: drag_name },
                                        |drag, _, _, cx| cx.new(|_| drag.clone()),
                                    )
                                    .drag_over::<ProjectDragPayload>(|style, _, _, cx| {
                                        style.bg(cx.theme().accent.opacity(0.18))
                                    })
                                    .can_drop(move |drag, _, _| {
                                        drag.downcast_ref::<ProjectDragPayload>().is_some_and(
                                            |payload| payload.project_id != can_drop_id,
                                        )
                                    })
                                    .on_drop(move |drag: &ProjectDragPayload, _, cx| {
                                        crate::todo_actions::drop_reorder_projects(
                                            &drag.project_id,
                                            &drop_id,
                                            cx,
                                        );
                                    })
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        this.activate_project(story.clone(), cx);
                                    }))
                                    .when(has_children, |this| {
                                        this.child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "collapse-project-{collapse_id}"
                                                )))
                                                .flex_shrink_0()
                                                .on_mouse_down(
                                                    gpui::MouseButton::Left,
                                                    |_, _, cx| cx.stop_propagation(),
                                                )
                                                .on_click({
                                                    let project = project.clone();
                                                    move |_, _, cx| {
                                                        cx.stop_propagation();
                                                        crate::todo_actions::set_project_collapsed(
                                                            project.clone(),
                                                            !project.collapsed,
                                                            cx,
                                                        );
                                                    }
                                                })
                                                .child(
                                                    Button::new(format!(
                                                        "collapse-project-btn-{collapse_id}"
                                                    ))
                                                    .small()
                                                    .ghost()
                                                    .compact()
                                                    .icon(if collapsed {
                                                        IconName::ChevronRight
                                                    } else {
                                                        IconName::ChevronDown
                                                    })
                                                    .tooltip(if collapsed {
                                                        t!("todo.project.expand").to_string()
                                                    } else {
                                                        t!("todo.project.collapse").to_string()
                                                    }),
                                                ),
                                        )
                                    })
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .overflow_x_hidden()
                                            .whitespace_nowrap()
                                            .child(project.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "favorite-project-{favorite_id}"
                                            )))
                                            .flex_shrink_0()
                                            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                                                cx.stop_propagation()
                                            })
                                            .on_click({
                                                move |_, window, cx| {
                                                    cx.stop_propagation();
                                                    let next = !favorite_project.is_favorite;
                                                    crate::todo_actions::set_project_favorite(
                                                        favorite_project.clone(),
                                                        next,
                                                        cx,
                                                    );
                                                    window.push_notification(
                                                        if next {
                                                            t!("todo.project.favorited").to_string()
                                                        } else {
                                                            t!("todo.project.unfavorited")
                                                                .to_string()
                                                        },
                                                        cx,
                                                    );
                                                }
                                            })
                                            .child(
                                                Button::new(format!(
                                                    "favorite-project-btn-{favorite_id}"
                                                ))
                                                .small()
                                                .ghost()
                                                .compact()
                                                .icon(IconName::StarOutlineThickSymbolic)
                                                .text_color(if favorite {
                                                    cx.theme().warning
                                                } else {
                                                    cx.theme().muted_foreground
                                                })
                                                .tooltip(if favorite {
                                                    t!("todo.project.unfavorite").to_string()
                                                } else {
                                                    t!("todo.project.favorite").to_string()
                                                }),
                                            ),
                                    )
                                    .when(count > 0, |this| {
                                        this.child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(count.to_string()),
                                        )
                                    })
                            })),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .bg(cx.theme().background)
                    .overflow_x_hidden()
                    .when(self.search_open, |this| {
                        this.child(
                            v_flex()
                                .w_full()
                                .px_3()
                                .py_2()
                                .gap_1()
                                .border_b_1()
                                .border_color(cx.theme().border)
                                .child(Input::new(&self.search_input).cleanable(true))
                                .when(search_query.trim().is_empty(), |this| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(t!("todo.search.hint").to_string()),
                                    )
                                })
                                .when(
                                    !search_query.trim().is_empty() && search_hits.is_empty(),
                                    |this| {
                                        this.child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(t!("todo.search.empty").to_string()),
                                        )
                                    },
                                )
                                .children(search_hits.into_iter().enumerate().map(|(ix, item)| {
                                    let selected = cx
                                        .global::<crate::core::state::ItemSelection>()
                                        .contains(&item.id);
                                    let item_for_click = item.clone();
                                    div()
                                        .id(("search-hit", ix))
                                        .w_full()
                                        .rounded(cx.theme().radius)
                                        .hover(|this| this.bg(cx.theme().sidebar_accent))
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, window, cx| {
                                                this.jump_to_item(
                                                    item_for_click.clone(),
                                                    window,
                                                    cx,
                                                );
                                            },
                                        ))
                                        .child(ItemListItem::new(
                                            ("search-hit-item", ix),
                                            item,
                                            selected,
                                        ))
                                })),
                        )
                    })
                    .child(content),
            )
    }
}
