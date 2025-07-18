use std::collections::HashMap;

use crate::{
    play_ogg_file, CalendarStory, ColorPickerStory, DatePickerStory, StoryContainer, TableStory,
};
use gpui::{prelude::*, *};
use gpui_component::{
    button::{Button, ButtonVariants},
    dropdown::{Dropdown, DropdownState},
    h_flex,
    input::{InputEvent, InputState, TextInput},
    resizable::{h_resizable, resizable_panel, ResizableState},
    v_flex, ActiveTheme as _, ContextModal, IconName,
};
use my_components::date_picker::{DatePicker, DatePickerEvent, DatePickerState};
use my_components::sidebar::{
    Sidebar, SidebarBoard, SidebarBoardItem, SidebarMenu, SidebarMenuItem,
};
use todos::entity::ProjectModel;
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Board {
    Inbox,     // 未完成任务
    Today,     // 今日任务
    Scheduled, // 计划任务
    Pinboard,  // 挂起任务
    Labels,    // 标签list
    Completed, // 已完成任务
    Projects,  // project list
}

impl Board {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::Today => "Today",
            Self::Scheduled => "Scheduled",
            Self::Pinboard => "Pinboard",
            Self::Labels => "Labels",
            Self::Completed => "Completed",
            Self::Projects => "Project",
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            Self::Inbox => IconName::MailboxSymbolic,
            Self::Today => IconName::StarOutlineThickSymbolic,
            Self::Scheduled => IconName::MonthSymbolic,
            Self::Pinboard => IconName::PinSymbolic,
            Self::Labels => IconName::TagOutlineSymbolic,
            Self::Completed => IconName::CheckRoundOutlineSymbolic,
            Self::Projects => IconName::ProcessErrorSymbolic,
        }
    }
    pub fn count(&self) -> usize {
        match self {
            Self::Inbox => 10,
            Self::Today => 2,
            Self::Scheduled => 3,
            Self::Pinboard => 5,
            Self::Labels => 6,
            Self::Completed => 2,
            Self::Projects => 10,
        }
    }
    pub fn color(&self) -> Hsla {
        match self {
            Self::Inbox => gpui::rgb(0x99c1f1).into(),
            Self::Today => gpui::rgb(0x33d17a).into(),
            Self::Scheduled => gpui::rgb(0xdc8add).into(),
            Self::Pinboard => gpui::rgb(0xf66151).into(),
            Self::Labels => gpui::rgb(0xcdab8f).into(),
            Self::Completed => gpui::rgb(0xffbe6f).into(),
            Self::Projects => gpui::rgb(0x33D17A).into(),
        }
    }

    pub fn handler(
        &self,
    ) -> impl Fn(&mut TodoStory, &ClickEvent, &mut Window, &mut Context<TodoStory>) + 'static {
        let item = *self;
        move |this, _, _, cx| {
            if this.active_boards.contains_key(&item) {
                this.active_boards.remove(&item);
            } else {
                this.active_boards.insert(item, true);
                this.active_boards.remove(&this.last_active_board); // 我自己写的不一定正确
            }

            this.last_active_board = item;
            cx.notify();
        }
    }
}

pub struct TodoStory {
    stories: Vec<(&'static str, Vec<Entity<StoryContainer>>)>,
    active_group_index: Option<usize>,
    active_index: Option<usize>,
    collapsed: bool,
    focus_handle: gpui::FocusHandle,
    search_input: Entity<InputState>,
    sidebar_state: Entity<ResizableState>,
    _subscriptions: Vec<Subscription>,

    active_boards: HashMap<Board, bool>, // 所有的todo board list : today inbox schedued labels completed pinned
    last_active_board: Board,            // active board
    active_project: Option<ProjectModel>, // active project
    projects: Vec<ProjectModel>,
    project_date: Option<String>,
}

impl super::Mytool for TodoStory {
    fn title() -> &'static str {
        "Todoist1"
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
        let mut active_boards = HashMap::new();
        active_boards.insert(Board::Inbox, true);
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        let _subscriptions = vec![cx.subscribe(&search_input, |this, _, e, cx| match e {
            InputEvent::Change(_) => {
                this.active_group_index = Some(0);
                this.active_index = Some(0);
                cx.notify()
            }
            _ => {}
        })];
        let stories = vec![
            // (
            //     "Getting Started",
            //     vec![StoryContainer::panel::<WelcomeStory>(window, cx)],
            // ),
            (
                "TodoComponents",
                vec![
                    StoryContainer::panel::<ColorPickerStory>(window, cx),
                    StoryContainer::panel::<DatePickerStory>(window, cx),
                ],
            ),
        ];

        let mut this = Self {
            search_input,
            stories,
            active_group_index: Some(0),
            active_index: Some(0),
            collapsed: false,
            focus_handle: cx.focus_handle(),
            sidebar_state: ResizableState::new(cx),
            _subscriptions,
            active_boards,
            last_active_board: Board::Inbox,
            active_project: None,
            projects: vec![],
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let item_groups = [
            Board::Inbox,
            Board::Today,
            Board::Scheduled,
            Board::Pinboard,
            Board::Labels,
            Board::Completed,
        ];
        let projects = self.projects.clone();
        let query = self.search_input.read(cx).value().trim().to_lowercase();

        let stories: Vec<_> = self
            .stories
            .iter()
            .filter_map(|(name, items)| {
                let filtered_items: Vec<_> = items
                    .iter()
                    .filter(|story| story.read(cx).name.to_lowercase().contains(&query))
                    .cloned()
                    .collect();

                if !filtered_items.is_empty() {
                    Some((name, filtered_items))
                } else {
                    None
                }
            })
            .collect();

        let active_group = self.active_group_index.and_then(|index| stories.get(index));
        let active_story = self
            .active_index
            .and(active_group)
            .and_then(|group| group.1.get(self.active_index.unwrap()));
        let (_story_name, _description) =
            if let Some(story) = active_story.as_ref().map(|story| story.read(cx)) {
                (story.name.clone(), story.description.clone())
            } else {
                ("".into(), "".into())
            };

        h_resizable("gallery-container", self.sidebar_state.clone())
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
                                            item_groups
                                                .iter()
                                                .map(|item| {
                                                    SidebarBoardItem::new(
                                                        item.label(),
                                                        item.color(),
                                                        item.color(),
                                                        item.count(),
                                                    )
                                                    .size(gpui::Length::Definite(
                                                        gpui::DefiniteLength::Fraction(0.5),
                                                    ))
                                                    .icon(item.icon())
                                                    .active(self.active_boards.contains_key(item))
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
                                            .mt(px(35.0)), // .child(div().child("On This Computer").text_left())
                                                           // .child(div().child(
                                                           //     Icon::new(IconName::PlusLargeSymbolic).text_right(),
                                                           // )),
                                    ),
                            )
                            .child(
                                // SidebarGroup::new("Projects").child()
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
                            .child(
                                // 项目列表：
                                SidebarMenu::new().children(projects.into_iter().enumerate().map(
                                    |(_, project)| {
                                        SidebarMenuItem::new(project.name.clone())
                                            .active(self.active_project == Some(project.clone()))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.active_project = Some(project.clone());
                                                this.last_active_board = Board::Projects;
                                                cx.notify();
                                            }))
                                    },
                                )),
                            )
                            .children(stories.clone().into_iter().enumerate().map(
                                |(group_ix, (_group_name, sub_stories))| {
                                    SidebarMenu::new().children(sub_stories.iter().enumerate().map(
                                        |(ix, story)| {
                                            SidebarMenuItem::new(story.read(cx).name.clone())
                                                .active(
                                                    self.active_group_index == Some(group_ix)
                                                        && self.active_index == Some(ix),
                                                )
                                                .on_click(cx.listener(
                                                    move |this, _: &ClickEvent, _, cx| {
                                                        this.active_group_index = Some(group_ix);
                                                        this.active_index = Some(ix);
                                                        cx.notify();
                                                    },
                                                ))
                                        },
                                    ))
                                },
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
