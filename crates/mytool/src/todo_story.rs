use crate::{load_items, load_labels, play_ogg_file, BoardPanel, ProjectEvent, ProjectListPanel};
use crate::{DBState, ItemListDelegate, LabelListDelegate};
use gpui::{prelude::*, *};
use gpui_component::date_picker::DatePickerEvent;
use gpui_component::label::Label;
use gpui_component::sidebar::{SidebarMenu, SidebarMenuItem};
use gpui_component::switch::Switch;
use gpui_component::{
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerState},
    h_flex,
    input::{Input, InputState},
    list::List,
    resizable::{h_resizable, resizable_panel},
    sidebar::Sidebar,
    v_flex,
    ContextModal,
};
use gpui_component::{Placement, Sizable};
use serde::Deserialize;
use std::option::Option;
use std::rc::Rc;
use todos::entity::{ItemModel, LabelModel, ProjectModel};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = todo_story, no_json)]
pub struct SelectTodo(SharedString);

pub struct TodoStory {
    collapsed: bool,
    focus_handle: gpui::FocusHandle,
    active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
    //  看板是0, projects是1
    pub is_board_active: bool,
    // 所有看板
    board_panel: Entity<BoardPanel>,
    // 所有projects
    project_panel: Entity<ProjectListPanel>,
    project_due: Option<String>,
    // labels
    label_list: Entity<List<LabelListDelegate>>,
    item_list: Entity<List<ItemListDelegate>>,
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
        let project_panel = ProjectListPanel::view(window, cx);
        let board_panel = BoardPanel::view(window, cx);
        let label_list = cx.new(|cx| List::new(LabelListDelegate::new(), window, cx));
        let item_list = cx.new(|cx| List::new(ItemListDelegate::new(), window, cx));
        let _subscriptions =
            vec![
                cx.subscribe(&project_panel, |this, _, event: &ProjectEvent, cx| {
                    this.project_panel.update(cx, |project_panel, cx| {
                        project_panel.handle_project_event(event, cx);
                    });
                }),
            ];

        let label_list_clone = label_list.clone();
        let item_list_clone = item_list.clone();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |_view, cx| {
            let db = db.lock().await;
            let labels = load_labels(db.clone()).await;
            let items = load_items(db.clone()).await;
            let rc_labels: Vec<Rc<LabelModel>> =
                labels.iter().map(|i| Rc::new(i.clone())).collect();
            let rc_items: Vec<Rc<ItemModel>> = items.iter().map(|i| Rc::new(i.clone())).collect();

            let _ = cx
                .update_entity(&label_list_clone, |list, cx| {
                    list.delegate_mut().update_labels(rc_labels);
                    cx.notify();
                })
                .ok();
            let _ = cx
                .update_entity(&item_list_clone, |list, cx| {
                    list.delegate_mut().update_items(rc_items);
                    cx.notify();
                })
                .ok();
        })
        .detach();
        Self {
            collapsed: false,
            active_index: None,
            focus_handle: cx.focus_handle(),
            _subscriptions,
            is_board_active: true,
            board_panel,
            project_panel,
            project_due: None,
            label_list,
            item_list,
        }
    }

    #[allow(unused)]
    fn open_drawer_at(
        &mut self,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let _list_h = match placement {
            Placement::Left | Placement::Right => px(400.),
            Placement::Top | Placement::Bottom => px(160.),
        };

        let _overlay = true;
        let _overlay_closable = true;
        let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Your Name"));
        let _input2 = cx.new(|cx| {
            InputState::new(window, cx).placeholder("For test focus back on modal close.")
        });
        let date = cx.new(|cx| DatePickerState::new(window, cx));
        window.open_drawer_at(placement, cx, move |this, _, _| {
            this.size(px(400.))
                .title("Item 详情:")
                .gap_4()
                .child(Input::new(&input1))
                .child(DatePicker::new(&date).placeholder("Date of Birth"))
                .child(
                    Button::new("send-notification")
                        .child("Test Notification")
                        .on_click(|_, window, cx| {
                            window.push_notification("Hello this is message from Drawer.", cx)
                        }),
                )
                .footer(
                    h_flex()
                        .gap_6()
                        .items_center()
                        .child(Button::new("confirm").primary().label("确认").on_click(
                            |_, window, cx| {
                                window.close_drawer(cx);
                            },
                        ))
                        .child(
                            Button::new("cancel")
                                .label("取消")
                                .on_click(|_, window, cx| {
                                    window.close_drawer(cx);
                                }),
                        ),
                )
        });
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(Some(""), window, cx))
    }
    fn add_project_model(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Project Name"));
        let _input2 = cx.new(|cx| -> InputState {
            InputState::new(window, cx).placeholder("For test focus back on modal close.")
        });
        let now = chrono::Local::now().naive_local().date();
        let project_due = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
            picker.set_date(now, window, cx);
            picker
        });
        let _ = cx.subscribe(&project_due, |this, _, ev, _| match ev {
            DatePickerEvent::Change(date) => {
                this.project_due = date.format("%Y-%m-%d").map(|s| s.to_string());
            }
        });
        let view = cx.entity().clone();

        window.open_modal(cx, move |modal, _, _| {
            modal
                .title("Add Project")
                .overlay(false)
                .keyboard(true)
                .show_close(true)
                .overlay_closable(true)
                .child(
                    v_flex()
                        .gap_3()
                        .child(Input::new(&input1))
                        .child(DatePicker::new(&project_due).placeholder("DueDate of Project")),
                )
                .footer({
                    let view = view.clone();
                    let input1 = input1.clone();
                    move |_, _, _, _cx| {
                        vec![
                            Button::new("add").primary().label("Add").on_click({
                                let view = view.clone();
                                let input1 = input1.clone();
                                move |_, window, cx| {
                                    window.close_modal(cx);
                                    view.update(cx, |view, cx| {
                                        let project = ProjectModel {
                                            name: input1.read(cx).value().to_string(),
                                            due_date: view.project_due.clone(),
                                            ..Default::default()
                                        };
                                        println!("TODO db add project {:?}", project);
                                        cx.notify();
                                    });
                                }
                            }),
                            Button::new("cancel")
                                .label("Cancel")
                                .on_click(move |_, window, cx| {
                                    window.close_modal(cx);
                                }),
                        ]
                    }
                })
        });
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
                                            move |_this, _, _window: &mut Window, cx| {
                                                // let projects = projects.read(cx);
                                                println!("click to add project");
                                                play_ogg_file("assets/sounds/success.ogg");
                                                // this.project_panel.read(cx).add_project_model(window,cx);
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
                                                println!("project idx:{:?}", project_avtive_index);
                                                cx.notify();
                                            },
                                        ))
                                        .suffix(Switch::new("dark-mode").checked(true).xsmall())
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
                                let project_some = project_list.get(self.project_panel.read(cx).active_index.unwrap());
                                if let Some(project) = project_some {
                                    println!("project:{:?}", project_some);
                                    board_active_index = None;
                                    this.child(Label::new(project.name.clone()))
                                } else {
                                    this.child(Empty)
                                }
                            }),
                    )
                    .into_any_element(),
            )
    }
}
