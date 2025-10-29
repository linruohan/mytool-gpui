use crate::views::BoardContainer;
use crate::{
    CompletedBoard, InboxBoard, LabelsBoard, PinBoard, ProjectListDelegate, ScheduledBoard,
    TodayBoard, load_items, load_labels,
};
use crate::{DBState, ItemListDelegate, LabelListDelegate, load_projects, play_ogg_file};
use gpui::{prelude::*, *};
use gpui_component::select::{Select, SelectState};
use gpui_component::sidebar::{SidebarBoard, SidebarBoardItem};
use gpui_component::switch::Switch;
use gpui_component::{
    ActiveTheme as _, ContextModal, List,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    h_flex,
    input::{Input, InputEvent, InputState},
    label::Label,
    resizable::{ResizableState, h_resizable, resizable_panel},
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    v_flex,
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
    active_index: Option<usize>,
    collapsed: bool,
    focus_handle: gpui::FocusHandle,
    search_input: Entity<InputState>,
    sidebar_state: Entity<ResizableState>,
    _subscriptions: Vec<Subscription>,
    //  看板是0, projects是1
    pub is_board_active: bool,

    // 所有看板
    boards: Vec<Entity<BoardContainer>>,
    // 所有projects
    project_list: Entity<List<ProjectListDelegate>>,
    project_date: Option<String>,
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
    pub fn new(init_story: Option<&str>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let boards = vec![
            BoardContainer::panel::<InboxBoard>(window, cx),
            BoardContainer::panel::<TodayBoard>(window, cx),
            BoardContainer::panel::<ScheduledBoard>(window, cx),
            BoardContainer::panel::<PinBoard>(window, cx),
            BoardContainer::panel::<LabelsBoard>(window, cx),
            BoardContainer::panel::<CompletedBoard>(window, cx),
        ];
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        let project_list = cx.new(|cx| List::new(ProjectListDelegate::new(), window, cx));
        let label_list = cx.new(|cx| List::new(LabelListDelegate::new(), window, cx));
        let item_list = cx.new(|cx| List::new(ItemListDelegate::new(), window, cx));

        let _subscriptions = vec![
            cx.subscribe(&search_input, |this, _, e, cx| match e {
                InputEvent::Change => {
                    this.is_board_active = true;
                    this.active_index = Some(0);
                    cx.notify()
                }
                _ => {}
            }),
            // cx.subscribe(&search_input, |this, _, event: &ItemClickEvent, cx| {
            //     this.open_drawer_at(Placement::Right, window, cx);
            //     cx.notify()
            // }),
        ];

        let project_list_clone = project_list.clone();
        let label_list_clone = label_list.clone();
        let item_list_clone = item_list.clone();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |_view, cx| {
            let db = db.lock().await;
            let projects = load_projects(db.clone()).await;
            let labels = load_labels(db.clone()).await;
            let items = load_items(db.clone()).await;
            let rc_projects: Vec<Rc<ProjectModel>> =
                projects.iter().map(|pro| Rc::new(pro.clone())).collect();
            let rc_labels: Vec<Rc<LabelModel>> =
                labels.iter().map(|i| Rc::new(i.clone())).collect();
            let rc_items: Vec<Rc<ItemModel>> = items.iter().map(|i| Rc::new(i.clone())).collect();
            println!("get rc_projects:{}", rc_projects.len());
            let _ = cx
                .update_entity(&project_list_clone, |list, cx| {
                    list.delegate_mut().update_projects(rc_projects);
                    cx.notify();
                })
                .ok();
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
        let mut this = Self {
            search_input,
            active_index: Some(0),
            collapsed: false,
            focus_handle: cx.focus_handle(),
            sidebar_state: ResizableState::new(cx),
            _subscriptions,
            is_board_active: true,
            boards,
            project_list,
            project_date: None,
            label_list,
            item_list,
        };

        if let Some(init_story) = init_story {
            this.set_active_todo(init_story, window, cx);
        }
        this
    }
    fn render_content(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.is_board_active {
            let board = self.boards.get(self.active_index.unwrap()).unwrap();
            v_flex().child(board.clone())
        } else {
            let projects = self.project_list.read(cx).delegate()._projects.clone();
            let project = projects.get(self.active_index.unwrap());
            if let Some(project) = project {
                v_flex().child(Label::new(project.name.clone()))
            } else {
                v_flex().child(Empty)
            }
        }
    }
    fn set_active_todo(&mut self, name: &str, window: &mut Window, cx: &mut App) {
        let name = name.to_string();
        self.search_input.update(cx, |this, cx| {
            this.set_value(&name, window, cx);
        })
    }

    fn add_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
                this.project_date = date.format("%Y-%m-%d").map(|s| s.to_string());
            }
        });
        let dropdown = cx.new(|cx| {
            SelectState::new(
                vec![
                    "Option 1".to_string(),
                    "Option 2".to_string(),
                    "Option 3".to_string(),
                ],
                None,
                window,
                cx,
            )
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
                        .child(Select::new(&dropdown))
                        .child(DatePicker::new(&project_due).placeholder("DueDate of Project")),
                )
                .footer({
                    let view = view.clone();
                    let input1 = input1.clone();
                    move |_, _, _, _cx| {
                        vec![
                            Button::new("confirm").primary().label("Confirm").on_click({
                                let view = view.clone();
                                let input1 = input1.clone();
                                move |_, window, cx| {
                                    window.close_modal(cx);
                                    view.update(cx, |view, cx| {
                                        let mut project = ProjectModel::default();
                                        project.name = input1.read(cx).value().to_string();
                                        project.due_date = view.project_date.clone();
                                        println!("added project: {:?}", project);
                                        // let panel = TodoContainer::panel::<ProjectItem>(window, cx);
                                        // panel.update(cx, |view, _| {
                                        //     view.name = project.name.into();
                                        // });
                                        // let _ = cx.update_entity(&self.project_list, |list, cx| {
                                        //     list.delegate_mut().add(project.into());
                                        // });
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
    #[allow(unused)]
    fn open_drawer_at(
        &mut self,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        println!("奥斯丁发射点法速度发生的");
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
}

impl Render for TodoStory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search_input.read(cx).value().trim().to_lowercase();
        let boards: Vec<_> = self
            .boards
            .iter()
            .filter(|story| story.read(cx).name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        let projects: Vec<_> = self
            .project_list
            .read(cx)
            .delegate()
            ._projects
            .iter()
            .filter(|story| story.name.to_lowercase().contains(&query))
            .cloned()
            .collect();

        h_resizable("todos-container", self.sidebar_state.clone())
            .child(
                resizable_panel()
                    .size(px(255.))
                    .size_range(px(200.)..px(320.))
                    .child(
                        Sidebar::left()
                            .width(relative(1.))
                            .border_width(px(0.))
                            .collapsed(self.collapsed)
                            .board(
                                //项目分类：
                                v_flex()
                                    .w_full()
                                    .gap_4()
                                    .child(
                                        div()
                                            .bg(cx.theme().sidebar_accent)
                                            .rounded_full()
                                            .when(cx.theme().radius.is_zero(), |this| {
                                                this.rounded(px(0.))
                                            })
                                            .flex_1()
                                            .mx_1()
                                            .child(
                                                Input::new(&self.search_input)
                                                    .appearance(false)
                                                    .cleanable(),
                                            ),
                                    )
                                    .child(
                                        SidebarBoard::new().children(
                                            boards
                                                .iter()
                                                .enumerate()
                                                .map(|(ix, item)| {
                                                    let board = item.read(cx);
                                                    SidebarBoardItem::new(
                                                        board.name.clone(),
                                                        board.colors.clone(),
                                                        board.count,
                                                        board.icon.clone(),
                                                    )
                                                    .size(gpui::Length::Definite(
                                                        gpui::DefiniteLength::Fraction(0.5),
                                                    ))
                                                    .active(
                                                        self.is_board_active
                                                            && self.active_index == Some(ix),
                                                    )
                                                    .on_click(cx.listener(
                                                        move |this, _: &ClickEvent, _, cx| {
                                                            this.is_board_active = true;
                                                            this.active_index = Some(ix);
                                                            cx.notify();
                                                        },
                                                    ))
                                                    // .on_click(cx.listener(item.handler()))
                                                })
                                                .collect::<Vec<_>>(),
                                        ),
                                    )
                                    .child(
                                        h_flex()
                                            .bg(cx.theme().sidebar_border)
                                            .px_1()
                                            .flex_1()
                                            .justify_between()
                                            .mt(px(35.0)),
                                    ),
                            )
                            .child(
                                // 添加项目按钮：
                                SidebarMenu::new().child(
                                    SidebarMenuItem::new("On This Computer                     ➕")
                                        .on_click(cx.listener(
                                            move |this, _, window: &mut Window, cx| {
                                                // let projects = projects.read(cx);
                                                println!("{}", "add projects");
                                                play_ogg_file("assets/sounds/success.ogg");
                                                this.add_project(window, cx);
                                                cx.notify();
                                            },
                                        )),
                                ),
                            )
                            .child(SidebarMenu::new().children(projects.iter().enumerate().map(
                                |(ix, story)| {
                                    SidebarMenuItem::new(story.name.clone())
                                        .active(
                                            !self.is_board_active && self.active_index == Some(ix),
                                        )
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, _, cx| {
                                                this.is_board_active = false;
                                                this.active_index = Some(ix);
                                                cx.notify();
                                            },
                                        ))
                                        .suffix(Switch::new("dark-mode").checked(true).xsmall())
                                },
                            ))),
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
                            .child(self.render_content(window, cx)),
                    )
                    .into_any_element(),
            )
    }
}
