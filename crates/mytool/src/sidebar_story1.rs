use std::collections::HashMap;

use gpui::{
    prelude::FluentBuilder, px, Action, App, AppContext, ClickEvent, Context, Entity, Focusable,
    Hsla, IntoElement, ParentElement, Render, SharedString, Styled, Window,
};

use crate::play_ogg_file;
use gpui_component::{
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    button::{Button, ButtonVariants},
    divider::Divider,
    dropdown::{Dropdown, DropdownState},
    h_flex,
    input::{InputState, TextInput},
    switch::Switch,
    v_flex, ActiveTheme, ContextModal, IconName, Side,
};
use my_components::date_picker::{DatePicker, DatePickerEvent, DatePickerState};
use my_components::sidebar::{
    Sidebar, SidebarBoard, SidebarBoardItem, SidebarMenu, SidebarMenuItem, SidebarToggleButton,
};
use serde::Deserialize;
use todos::entity::ProjectModel;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sidebar_story, no_json)]
pub struct SelectCompany(SharedString);

pub struct TodoSidebarStory {
    active_boards: HashMap<Board, bool>, // 所有的todo board list : today inbox schedued labels completed pinned
    last_active_board: Board,            // active board
    active_project: Option<ProjectModel>, // active project
    collapsed: bool,
    side: Side,
    focus_handle: gpui::FocusHandle,
    checked: bool,
    projects: Vec<ProjectModel>,
    _search_input: Entity<InputState>,
    project_date: Option<String>,
}

impl TodoSidebarStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut active_boards = HashMap::new();
        active_boards.insert(Board::Inbox, true);
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        Self {
            active_boards,
            last_active_board: Board::Inbox,
            active_project: None,
            collapsed: false,
            side: Side::Left,
            focus_handle: cx.focus_handle(),
            checked: false,
            projects: vec![],
            _search_input: input,
            project_date: None,
        }
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

    fn render_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().child(
            h_flex().gap_2().child(
                Switch::new("side")
                    .label("Placement Right")
                    .checked(self.side.is_right())
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.side = if *checked { Side::Right } else { Side::Left };
                        cx.notify();
                    })),
            ), // .child(TodayView::new(window, cx)),
        )
    }
}

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
    ) -> impl Fn(&mut TodoSidebarStory, &ClickEvent, &mut Window, &mut Context<TodoSidebarStory>) + 'static
    {
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

impl super::Mytool for TodoSidebarStory {
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

impl Focusable for TodoSidebarStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TodoSidebarStory {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let item_groups = [
            Board::Inbox,
            Board::Today,
            Board::Scheduled,
            Board::Pinboard,
            Board::Labels,
            Board::Completed,
        ];
        let projects = self.projects.clone();
        // let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));

        h_flex()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .h_full()
            .when(self.side.is_right(), |this| this.flex_row_reverse())
            .child(
                Sidebar::new(self.side)
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
                                .on_click(cx.listener(move |this, _, window: &mut Window, cx| {
                                    // let projects = projects.read(cx);
                                    println!("{}", "add projects");
                                    play_ogg_file("assets/sounds/success.ogg");
                                    this.add_project(window, cx);
                                    cx.notify();
                                })),
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
                    ),
            )
            .child(
                v_flex()
                    .size_full()
                    .gap_4()
                    .p_4()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .when(self.side.is_right(), |this| {
                                this.flex_row_reverse().justify_between()
                            })
                            .child(
                                SidebarToggleButton::left()
                                    .side(self.side)
                                    .collapsed(self.collapsed)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.collapsed = !this.collapsed;
                                        cx.notify();
                                    })),
                            )
                            .child(Divider::vertical().h_4())
                            .child(
                                Breadcrumb::new()
                                    .item(BreadcrumbItem::new("0", "Home").on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.last_active_board = Board::Inbox;
                                            cx.notify();
                                        },
                                    )))
                                    .item(
                                        BreadcrumbItem::new("1", self.last_active_board.label())
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.active_project = None;
                                                cx.notify();
                                            })),
                                    )
                                    .when_some(self.active_project.clone(), |this, subitem| {
                                        this.item(BreadcrumbItem::new("2", subitem.name))
                                    }),
                            ),
                    )
                    .child(self.render_content(window, cx)),
            )
    }
}
