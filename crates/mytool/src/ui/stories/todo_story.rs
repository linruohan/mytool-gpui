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
use serde::Deserialize;
use todos::entity::{ItemModel, ProjectModel};

use crate::{
    BatchCompleteSelected, BatchDeleteSelected, BoardPanel, DeleteTask, DeselectAll, DuplicateTask,
    EditTask, NewProject, NewTask, OpenHelp, OpenSettings, ProjectEvent, ProjectItemEvent,
    ProjectItemsPanel, ProjectsPanel, SearchTasks, SelectAllTasks, SelectNextTask,
    SelectPreviousTask, SetDueDate, ShowCompleted, ShowInbox, ShowLabels, ShowPinned,
    ShowScheduled, ShowToday, ToggleSidebar, ToggleTaskComplete, ToggleTaskPin, UndoLastTask,
    play_ogg_file,
    todo_state::TodoStore,
    ui::components::{
        show_existing_item_dialog, show_new_item_dialog, show_set_due_dialog,
        show_todo_help_dialog, show_todo_settings_dialog,
    },
};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = todo_story, no_json)]
pub struct SelectTodo(SharedString);

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
}

impl super::Mytool for TodoStory {
    fn title() -> &'static str {
        "Todoist 任务管理"
    }

    fn description() -> &'static str {
        "侧栏看板、项目与今日任务"
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
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("搜索任务..."));
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
        }
    }

    fn show_board(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_project = None;
        self.project_panel.update(cx, |panel, cx| {
            panel.update_active_index(None);
            cx.notify();
        });
        self.board_panel.update(cx, |panel, cx| {
            panel.update_active_index(Some(index));
            cx.notify();
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
        self.search_open = true;
        self.search_input.read(cx).focus_handle(cx).focus(window, cx);
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
            window.push_notification(format!("已完成 {n} 个任务"), cx);
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
            window.push_notification(format!("已删除 {n} 个任务"), cx);
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
            window.push_notification("已删除任务。", cx);
            return;
        }
        crate::show_item_delete_dialog(window, cx, "确定删除这个任务吗？", move |cx| {
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
            window
                .push_notification(if item.checked { "已标记为未完成" } else { "已完成任务" }, cx);
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
        copy.content = format!("{}（副本）", copy.content);
        copy.checked = false;
        copy.completed_at = None;
        crate::todo_actions::add_item_optimistic(Arc::new(copy), cx);
        window.push_notification("已复制任务", cx);
    }

    fn on_toggle_pin(&mut self, _: &ToggleTaskPin, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = self.primary_item(cx) {
            let pinned = !item.pinned;
            crate::todo_actions::set_item_pinned_optimistic(item, pinned, cx);
            window.push_notification(if pinned { "已置顶" } else { "已取消置顶" }, cx);
        }
    }

    fn on_set_due_date(&mut self, _: &SetDueDate, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = self.primary_item(cx) {
            show_set_due_dialog(window, cx, item);
        }
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

    fn on_new_project(&mut self, _: &NewProject, window: &mut Window, cx: &mut Context<Self>) {
        self.open_new_project(window, cx);
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
        let _ = play_ogg_file("assets/sounds/success.ogg");
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
            .on_action(cx.listener(Self::on_edit_task))
            .on_action(cx.listener(Self::on_delete_task))
            .on_action(cx.listener(Self::on_toggle_complete))
            .on_action(cx.listener(Self::on_duplicate_task))
            .on_action(cx.listener(Self::on_toggle_pin))
            .on_action(cx.listener(Self::on_set_due_date))
            .on_action(cx.listener(Self::on_select_previous))
            .on_action(cx.listener(Self::on_select_next))
            .on_action(cx.listener(Self::on_toggle_sidebar))
            .on_action(cx.listener(Self::on_new_project))
            .on_action(cx.listener(Self::on_show_inbox))
            .on_action(cx.listener(Self::on_show_today))
            .on_action(cx.listener(Self::on_show_scheduled))
            .on_action(cx.listener(Self::on_show_labels))
            .on_action(cx.listener(Self::on_show_pinned))
            .on_action(cx.listener(Self::on_show_completed))
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
                                            .child("此电脑"),
                                    )
                                    .child(
                                        Button::new("add-project")
                                            .small()
                                            .ghost()
                                            .compact()
                                            .icon(IconName::Plus)
                                            .tooltip("新建项目")
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
                                        .child("还没有项目"),
                                )
                            })
                            .children(project_list.iter().enumerate().map(
                                |(ix, (project, nested))| {
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
                                    h_flex()
                                        .id(("sidebar-project", ix))
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
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, _, cx| {
                                                this.active_project = Some(story.clone());
                                                let store_ix = cx
                                                    .global::<TodoStore>()
                                                    .projects
                                                    .iter()
                                                    .position(|p| p.id == story.id);
                                                this.project_panel.update(cx, |panel, cx| {
                                                    panel.update_active_index(store_ix);
                                                    cx.notify();
                                                });
                                                this.project_items_panel.update(cx, |panel, cx| {
                                                    panel.set_project(story.clone(), cx);
                                                    cx.notify();
                                                });
                                                this.board_panel.update(cx, |panel, cx| {
                                                    panel.update_active_index(None);
                                                    cx.notify();
                                                });
                                                cx.notify();
                                            },
                                        ))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .overflow_x_hidden()
                                                .whitespace_nowrap()
                                                .child(project.name.clone()),
                                        )
                                        .when(count > 0, |this| {
                                            this.child(
                                                div()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(count.to_string()),
                                            )
                                        })
                                },
                            )),
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
                                            .child("输入关键字，Esc 关闭"),
                                    )
                                })
                                .when(
                                    !search_query.trim().is_empty() && search_hits.is_empty(),
                                    |this| {
                                        this.child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("没有匹配的任务"),
                                        )
                                    },
                                )
                                .children(search_hits.into_iter().enumerate().map(|(ix, item)| {
                                    let label = item.content.clone();
                                    let checked = item.checked;
                                    h_flex()
                                        .id(("search-hit", ix))
                                        .w_full()
                                        .h_7()
                                        .px_2()
                                        .rounded(cx.theme().radius)
                                        .items_center()
                                        .justify_between()
                                        .hover(|this| this.bg(cx.theme().sidebar_accent))
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, window, cx| {
                                                this.search_open = false;
                                                show_existing_item_dialog(window, cx, item.clone());
                                                cx.notify();
                                            },
                                        ))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .overflow_x_hidden()
                                                .whitespace_nowrap()
                                                .when(checked, |this| {
                                                    this.text_color(cx.theme().muted_foreground)
                                                })
                                                .child(label),
                                        )
                                })),
                        )
                    })
                    .child(content),
            )
    }
}
