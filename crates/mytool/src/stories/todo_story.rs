use std::{option::Option, rc::Rc};

use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Side, h_flex,
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    switch::Switch,
    v_flex,
};
use serde::Deserialize;
use todos::entity::ProjectModel;

use crate::{
    BoardPanel, ProjectEvent, ProjectItemEvent, ProjectItemsPanel, ProjectsPanel, play_ogg_file,
};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = todo_story, no_json)]
pub struct SelectTodo(SharedString);

pub struct TodoStory {
    collapsed: bool,
    click_to_open_submenu: bool,
    side: Side,
    focus_handle: gpui::FocusHandle,
    active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
    // 看板是0, projects是1
    pub is_board_active: bool,
    // 所有看板
    board_panel: Entity<BoardPanel>,
    // projects
    project_panel: Entity<ProjectsPanel>,
    project_items_panel: Entity<ProjectItemsPanel>,
}

impl super::Mytool for TodoStory {
    fn title() -> &'static str {
        "Todoist"
    }

    fn description() -> &'static str {
        "my todoist sidebar story"
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
        let _subscriptions = vec![
            cx.subscribe(&project_panel, |this, _, event: &ProjectEvent, cx| {
                this.project_panel.update(cx, |project_panel, cx| {
                    project_panel.handle_project_event(event, cx);
                });
            }),
            cx.subscribe(&project_items_panel, |this, _, event: &ProjectItemEvent, cx| {
                this.project_items_panel.update(cx, |project_items_panel, cx| {
                    project_items_panel.handle_project_item_event(event, cx);
                });
            }),
        ];
        Self {
            collapsed: false,
            active_index: None,
            focus_handle: cx.focus_handle(),
            _subscriptions,
            is_board_active: true,
            board_panel,
            project_panel,
            project_items_panel,
            click_to_open_submenu: false,
            side: Side::Left,
        }
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

    fn add_project(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        play_ogg_file("assets/sounds/success.ogg");
        self.project_panel.update(cx, |project_panel, cx| {
            project_panel.open_project_dialog(Rc::new(ProjectModel::default()), window, cx);
            cx.notify();
        });
    }
}

impl Render for TodoStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let board_panel = self.board_panel.read(cx);
        let boards = board_panel.boards.clone();
        let board_active_index = board_panel.active_index;
        let project_panel = self.project_panel.read(cx);
        let project_list = project_panel.project_list.read(cx).delegate()._projects.clone();
        let _view = cx.entity();
        let project_avtive_index = project_panel.active_index;

        h_flex()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .h_full()
            .child(
                Sidebar::new(self.side)
                            .collapsed(self.collapsed)
                    .w(px(250.))
                            .gap_0()
                            .board(self.board_panel.clone()) // .child(self.project_panel.clone()),
                            .child(
                                // 添加项目按钮：
                                SidebarMenu::new().child(
                                    SidebarMenuItem::new("On This Computer   ➕")
                                        .on_click(cx.listener(Self::add_project)),
                                ),
                            )
                            .child(SidebarMenu::new().children(
                                project_list.iter().enumerate().map(|(ix, story)| {
                                    SidebarMenuItem::new(story.name.clone())
                                        .active(project_avtive_index == Some(ix))
                                        .on_click({
                                            let story = story.clone();
                                            cx.listener(move |this, _: &ClickEvent, _, cx| {
                                                this.is_board_active = false;
                                                this.active_index = Some(ix);
                                                this.project_panel.update(cx, |panel, cx| {
                                                    panel.update_active_index(Some(ix));
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
                                            })
                                        })
                                }),
                            )),
            )
            .child(
                v_flex().flex_1().h_full().overflow_x_hidden().child(
                    div()
                        .id("todos")
                        .flex_1()
                        .overflow_y_scroll()
                        .when(board_active_index.is_some(), |this| {
                            let board_some = boards.get(board_active_index.unwrap());
                            if let Some(board) = board_some {
                                this.child(board.clone())
                            } else {
                                this.child(Empty)
                            }
                        })
                        .when(!self.is_board_active, |this| {
                            this.child(self.project_items_panel.clone())
                        }),
                ),
            )
    }
}
