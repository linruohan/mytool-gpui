use crate::{BoardType, ProjectListItem};
use crate::{DBState, load_projects, play_ogg_file};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, ContextModal,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    dropdown::{Dropdown, DropdownState},
    h_flex,
    input::{InputEvent, InputState, TextInput},
    resizable::{ResizableState, h_resizable, resizable_panel},
    sidebar::{Sidebar, SidebarBoard, SidebarBoardItem, SidebarMenu, SidebarMenuItem},
    v_flex,
};
use std::collections::HashMap;
use std::option::Option;
use todos::entity::ProjectModel;

pub fn init(_cx: &mut App) {
    println!("todos initialize");
    // let database_future = cx
    //     .spawn(|cx| async move { todo_database_init().await });
    //
    // cx.set_global(TodoState {
    //     conn: database_future,
    // });
}
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
    boards: Vec<BoardType>,
    pub active_boards: HashMap<BoardType, bool>,
    pub active_board: Option<BoardType>,
    // 所有projects
    projects: Vec<ProjectModel>,
    project_date: Option<String>,
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
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        let _subscriptions = vec![cx.subscribe(&search_input, |this, _, e, cx| match e {
            InputEvent::Change => {
                this.is_board_active = true;
                this.active_index = Some(0);
                cx.notify()
            }
            _ => {}
        })];
        let mut active_boards = HashMap::new();
        active_boards.insert(BoardType::Inbox, true);

        let boards = vec![
            BoardType::Inbox,
            BoardType::Today,
            BoardType::Scheduled,
            BoardType::Pinboard,
            BoardType::Labels,
            BoardType::Completed,
        ];
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |_view, _cx| {
            let db = db.lock().await;
            let projects = load_projects(db.clone()).await;
            println!("get rc_projects:{}", projects.len());
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
            active_boards,
            active_board: Some(BoardType::Inbox),
            projects: vec![],
            project_date: None,
        };

        if let Some(init_story) = init_story {
            this.set_active_story(init_story, window, cx);
        }

        this
    }
    fn render_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.is_board_active {
            let board = self.active_board.unwrap();
            v_flex().child(board.container(window, cx))
        } else {
            let project = self.projects.get(self.active_index.unwrap()).unwrap();
            v_flex()
                .child(cx.new(|_| ProjectListItem::new(SharedString::from(project.name.clone()))))
        }
    }
    fn set_active_story(&mut self, name: &str, window: &mut Window, cx: &mut App) {
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
        let date_picker = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
            picker.set_date(now, window, cx);
            picker
        });
        let _ = cx.subscribe(&date_picker, |this, _, ev, _| match ev {
            DatePickerEvent::Change(date) => {
                this.project_date = date.format("%Y-%m-%d").map(|s| s.to_string());
            }
        });
        let dropdown = cx.new(|cx| {
            DropdownState::new(
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
                        .child(TextInput::new(&input1))
                        .child(Dropdown::new(&dropdown))
                        .child(DatePicker::new(&date_picker).placeholder("DueDate of Project")),
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
                                        // let panel = TodoContainer::panel::<ProjectItem>(window, cx);
                                        // panel.update(cx, |view, _| {
                                        //     view.name = project.name.into();
                                        // });
                                        view.projects.push(project);
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
            .filter(|story| story.label().to_lowercase().contains(&query))
            .cloned()
            .collect();

        let projects: Vec<_> = self
            .projects
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
                                        SidebarBoard::new().children(
                                            boards
                                                .iter()
                                                .enumerate()
                                                .map(|(_ix, item)| {
                                                    SidebarBoardItem::new(
                                                        item.label(),
                                                        item.color(),
                                                        item.color(),
                                                        item.count(),
                                                        item.icon(),
                                                    )
                                                    .size(gpui::Length::Definite(
                                                        gpui::DefiniteLength::Fraction(0.5),
                                                    ))
                                                    .active(self.active_board == Some(*item))
                                                    .on_click(cx.listener(item.handler()))
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
                                                this.active_board = None;
                                                this.active_index = Some(ix);
                                                cx.notify();
                                            },
                                        ))
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
