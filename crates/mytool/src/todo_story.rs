use crate::todos_view::TodoContainer;
use crate::ProjectItem;
use crate::{
    play_ogg_file, CompletedBoard, InboxBoard, LabelsBoard, PinBoard, ScheduledBoard, TodayBoard,
};
use gpui::{prelude::*, *};
use gpui_component::dock::{Panel, PanelView};
use gpui_component::{
    button::{Button, ButtonVariants},
    dropdown::{Dropdown, DropdownState},
    h_flex,
    input::{InputEvent, InputState, TextInput},
    resizable::{h_resizable, resizable_panel, ResizableState},
    v_flex, ActiveTheme as _, ContextModal,
};
use my_components::date_picker::{DatePicker, DatePickerEvent, DatePickerState};
use my_components::sidebar::{
    Sidebar, SidebarBoard, SidebarBoardItem, SidebarMenu, SidebarMenuItem,
};
use std::option::Option;
use todos::entity::ProjectModel;

pub struct TodoStory {
    active_index: Option<usize>,
    collapsed: bool,
    focus_handle: gpui::FocusHandle,
    search_input: Entity<InputState>,
    sidebar_state: Entity<ResizableState>,
    _subscriptions: Vec<Subscription>,
    //  看板是0, project是1
    is_board_active: bool,

    // 所有看板
    boards: Vec<Entity<TodoContainer>>,
    // active_board_index: Option<usize>, // 所有的todo board list : today inbox schedued labels completed pinned
    // 所有projects
    projects: Vec<Entity<TodoContainer>>,
    // active_project_index: Option<usize>, // active project
    project_date: Option<String>,
}

impl super::Mytool for TodoStory {
    fn title() -> &'static str {
        "Todoist"
    }

    fn description() -> &'static str {
        "my todoist sidebar story"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
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
            InputEvent::Change(_) => {
                this.is_board_active = true;
                this.active_index = Some(0);
                cx.notify()
            }
            _ => {}
        })];
        let boards = vec![
            TodoContainer::panel::<InboxBoard>(window, cx),
            TodoContainer::panel::<TodayBoard>(window, cx),
            TodoContainer::panel::<ScheduledBoard>(window, cx),
            TodoContainer::panel::<PinBoard>(window, cx),
            TodoContainer::panel::<LabelsBoard>(window, cx),
            TodoContainer::panel::<CompletedBoard>(window, cx),
        ];

        let mut this = Self {
            search_input,
            // stories,
            active_index: Some(0),
            collapsed: false,
            focus_handle: cx.focus_handle(),
            sidebar_state: ResizableState::new(cx),
            _subscriptions,
            is_board_active: true,
            boards,
            // active_board_index: Some(0),
            projects: vec![],
            // active_project_index: Some(0),
            project_date: None,
        };

        if let Some(init_story) = init_story {
            this.set_active_story(init_story, window, cx);
        }

        this
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
            let mut picker = DatePickerState::new(window, cx);
            picker.set_date(now, window, cx);
            picker.set_disabled(vec![0, 6], window, cx);
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
                                        let panel = TodoContainer::panel::<ProjectItem>(window, cx);
                                        panel.update(cx, |view, _| {
                                            view.name = project.name.into();
                                        });
                                        view.projects.push(panel);
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
    pub fn handler(
        &self,
    ) -> impl Fn(&mut TodoStory, &ClickEvent, &mut Window, &mut Context<TodoStory>) + 'static {
        // let item = *self;
        move |this, _, _, cx| {
            // if this.active_boards.contains_key(&item) {
            //     this.active_boards.remove(&item);
            // } else {
            //     this.active_boards.insert(item, true);
            //     this.active_boards.remove(&this.last_active_board); // 我自己写的不一定正确
            // }

            // this.last_active_board = item;
            cx.notify();
        }
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
            .projects
            .iter()
            .filter(|story| story.read(cx).name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        let active_group = if self.is_board_active {
            boards
        } else {
            projects
        };
        let active_story = active_group.get(self.active_index.unwrap());

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
                                            self.boards
                                                .iter()
                                                .enumerate()
                                                .map(|(ix, item)| {
                                                    let item = item.read(cx);
                                                    SidebarBoardItem::new(
                                                        item.name.clone(),
                                                        item.board_color,
                                                        item.board_color,
                                                        item.board_count,
                                                        item.board_icon.clone(),
                                                    )
                                                    .size(gpui::Length::Definite(
                                                        gpui::DefiniteLength::Fraction(0.5),
                                                    ))
                                                    // .active(self.boards.contains(item))
                                                    .on_click(cx.listener(
                                                        move |this, _: &ClickEvent, _, cx| {
                                                            this.is_board_active = true;
                                                            this.active_index = Some(ix);
                                                            cx.notify();
                                                        },
                                                    ))
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
                            .child(SidebarMenu::new().children(
                                self.projects.iter().enumerate().map(|(ix, story)| {
                                    SidebarMenuItem::new(story.read(cx).name.clone())
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
                            .when_some(active_story, |this, active_story| {
                                this.child(active_story.clone())
                            }),
                    )
                    .into_any_element(),
            )
    }
}
