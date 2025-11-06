use crate::{
    BoardPanel, ProjectEvent, ProjectItemEvent, ProjectItemsPanel, ProjectsPanel, play_ogg_file,
};
use gpui::{prelude::*, *};
use gpui_component::menu::{DropdownMenu, PopupMenuItem};
use gpui_component::sidebar::{SidebarMenu, SidebarMenuItem};
use gpui_component::{
    IconName, IndexPath,
    button::{Button, ButtonVariants},
    resizable::{h_resizable, resizable_panel},
    sidebar::Sidebar,
    v_flex,
};
use serde::Deserialize;
use std::option::Option;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = todo_story, no_json)]
pub struct SelectTodo(SharedString);

pub struct TodoStory {
    collapsed: bool,
    focus_handle: gpui::FocusHandle,
    active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
    // 看板是0, projects是1
    pub is_board_active: bool,
    // 所有看板
    board_panel: Entity<BoardPanel>,
    // projects
    project_panel: Entity<ProjectsPanel>,
    project_item_panel: Entity<ProjectItemsPanel>,
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
        let board_panel = BoardPanel::view(window, cx);
        let project_item_panel = ProjectItemsPanel::view(None, window, cx);
        let _subscriptions = vec![
            cx.subscribe(&project_panel, |this, _, event: &ProjectEvent, cx| {
                this.project_panel.update(cx, |project_panel, cx| {
                    project_panel.handle_project_event(event, cx);
                });
            }),
            cx.subscribe(
                &project_item_panel,
                |this, _, event: &ProjectItemEvent, cx| {
                    this.project_item_panel
                        .update(cx, |project_item_panel, cx| {
                            project_item_panel.handle_project_item_event(&event, cx);
                        });
                },
            ),
            // cx.subscribe(&board_panel, |this, _, event: ItemEvent, cx| {
            //     this.board_panel.update(cx, |mut panel, cx| {
            //         panel.handle_item_event(event, cx);
            //     });
            // }),
        ];
        Self {
            collapsed: false,
            active_index: None,
            focus_handle: cx.focus_handle(),
            _subscriptions,
            is_board_active: true,
            board_panel,
            project_panel,
            project_item_panel,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(Some(""), window, cx))
    }
}

impl Render for TodoStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let board_panel = self.board_panel.read(cx);
        let boards = board_panel.boards.clone();
        let mut board_active_index = board_panel.active_index;
        let project_panel = self.project_panel.read(cx);
        let project_list = project_panel
            .project_list
            .read(cx)
            .delegate()
            ._projects
            .clone();
        let project_avtive_index = project_panel.active_index;
        let view = cx.entity();
        h_resizable("todos-container")
            .child(
                resizable_panel()
                    .size(px(255.))
                    .size_range(px(200.)..px(320.))
                    .child(
                        Sidebar::left()
                            .width(relative(1.))
                            .border_width(px(0.))
                            .collapsed(self.collapsed)
                            .board(self.board_panel.clone()) // .child(self.project_panel.clone()),
                            .child(
                                // 添加项目按钮：
                                SidebarMenu::new().child(
                                    SidebarMenuItem::new("On This Computer                     ➕")
                                        .on_click(cx.listener(
                                            move |this, _, _window: &mut Window, cx| {
                                                // let projects = projects.read(cx);
                                                println!("click to add project");
                                                play_ogg_file("assets/sounds/success.ogg");
                                                this.project_panel.update(cx, |_panel, cx| {
                                                    // panel.show_model(window, cx);
                                                    cx.notify();
                                                });
                                                cx.notify();
                                            },
                                        )),
                                ),
                            )
                            .child(SidebarMenu::new().children(
                                project_list.iter().enumerate().map(|(ix, story)| {
                                    SidebarMenuItem::new(story.name.clone())
                                        .active(project_avtive_index == Some(ix))
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, _, cx| {
                                                this.is_board_active = false;
                                                this.active_index = Some(ix);
                                                this.project_panel.update(cx, |panel, cx| {
                                                    panel.update_active_index(Some(ix));
                                                    cx.notify();
                                                });
                                                this.board_panel.update(cx, |panel, cx| {
                                                    panel.update_active_index(None);
                                                    cx.notify();
                                                });
                                                cx.notify();
                                            },
                                        ))
                                        .suffix(
                                            Button::new("project-popup-menu")
                                                .icon(IconName::EllipsisVertical)
                                                .dropdown_menu({
                                                    let view = view.clone();
                                                    move |this, window, _cx| {
                                                        this.link(
                                                            "About",
                                                            "https://github.com/longbridge/gpui-component",
                                                        )
                                                            .separator()
                                                            .item(PopupMenuItem::new("Edit project").on_click(
                                                                window.listener_for(&view, |this, _c, _window, cx| {
                                                                    println!("index: {:?}", this.active_index);
                                                                    this.project_panel.update(cx, |_panel, cx| {
                                                                        // panel.show_model(window, cx);
                                                                        cx.notify();
                                                                    });
                                                                    cx.notify();
                                                                }),
                                        ))
                                                            .separator()
                                                            .item(
                                                                PopupMenuItem::new("Delete project").on_click(
                                                                    window.listener_for(
                                                                        &view,
                                                                        |this, _, _window, cx| {
                                                                            this.project_panel.update(cx, |panel, cx| {
                                                                                let index = this.active_index.unwrap();
                                                                                let project_some = panel
                                                                                    .get_selected_project(
                                                                                        IndexPath::new(index),
                                                                                        cx,
                                                                                    );
                                                                                if let Some(project) = project_some {
                                                                                    panel.del_project(cx, project.clone());
                                                                                }
                                                                                cx.notify();
                                                                            });
                                                                            cx.notify();
                                                                        },
                                                                    ),
                                                                ),
                                                            )
                                                    }
                                                }),
                                        )
                                }),
                            )),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .overflow_x_hidden()
                    .child(
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
                                let project_some = project_list.get(self.active_index.unwrap());
                                if let Some(project) = project_some {
                                    board_active_index = None;
                                    self.project_item_panel.update(cx, |panel, cx| {
                                        panel.set_active_project(project.clone(), cx);
                                        cx.notify();
                                    });
                                    this.child(self.project_item_panel.clone())
                                } else {
                                    this.child(Empty)
                                }
                            }),
                    )
                    .into_any_element(),
            )
    }
}
