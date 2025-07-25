use std::collections::HashMap;

use gpui::{
    prelude::FluentBuilder, Action, App, AppContext, ClickEvent, Context, Entity,
    Focusable, IntoElement, ParentElement, Render, SharedString, Styled, Window,
};

use crate::{play_ogg_file, ProjectItem};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::dropdown::{Dropdown, DropdownState};
use gpui_component::input::{InputState, TextInput};
use gpui_component::{breadcrumb::{Breadcrumb, BreadcrumbItem}, divider::Divider, h_flex, popup_menu::PopupMenuExt, switch::Switch, v_flex, ActiveTheme, ContextModal, IconName, Side};
use my_components::date_picker::{DatePicker, DatePickerEvent, DatePickerState};
use my_components::sidebar::{Sidebar, SidebarMenu, SidebarMenuItem, SidebarToggleButton};
use serde::Deserialize;
use todos::entity::ProjectModel;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sidebar_story, no_json)]
pub struct SelectCompany(SharedString);

pub struct SidebarStory {
    pub(crate) active_boards: HashMap<Item, bool>,
    pub(crate) last_active_board: Item,
    pub(crate) active_project: Option<ProjectItem>,
    collapsed: bool,
    side: Side,
    focus_handle: gpui::FocusHandle,
    checked: bool,
    projects: Vec<ProjectItem>,
    project_date: Option<String>,
}

impl SidebarStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut active_items = HashMap::new();
        active_items.insert(Item::Playground, true);

        Self {
            active_boards: active_items,
            last_active_board: Item::Playground,
            active_project: None,
            collapsed: false,
            side: Side::Left,
            focus_handle: cx.focus_handle(),
            checked: false,
            projects: Vec::new(),
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
                                    view.update(cx, |v, cx| {
                                        let mut project = ProjectModel::default();
                                        project.name = input1.read(cx).value().to_string();
                                        project.due_date = v.project_date.clone();
                                        // let panel = TodoContainer::panel::<ProjectItem>(window, cx);
                                        // panel.update(cx, |v, _| {
                                        //     v.name = SharedString::from(&project.name);
                                        // });
                                        let project = ProjectItem::new(project.name);
                                        v.projects.push(project);
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
    fn render_content(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().child(
            h_flex().gap_2().child(
                Switch::new("side")
                    .label("Placement Right")
                    .checked(self.side.is_right())
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.side = if *checked { Side::Right } else { Side::Left };
                        cx.notify();
                    })),
            ),
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Item {
    Playground,
    Models,
    Documentation,
    Settings,
    DesignEngineering,
    SalesAndMarketing,
    Travel,
}


impl Item {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Playground => "Playground",
            Self::Models => "Models",
            Self::Documentation => "Documentation",
            Self::Settings => "Settings",
            Self::DesignEngineering => "Design Engineering",
            Self::SalesAndMarketing => "Sales and Marketing",
            Self::Travel => "Travel",
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            Self::Playground => IconName::SquareTerminal,
            Self::Models => IconName::Bot,
            Self::Documentation => IconName::BookOpen,
            Self::Settings => IconName::Settings2,
            Self::DesignEngineering => IconName::Frame,
            Self::SalesAndMarketing => IconName::ChartPie,
            Self::Travel => IconName::Map,
        }
    }

    pub fn handler(
        &self,
    ) -> impl Fn(&mut SidebarStory, &ClickEvent, &mut Window, &mut Context<SidebarStory>) + 'static
    {
        let item = *self;
        move |this, _, _, cx| {
            if this.active_boards.contains_key(&item) {
                this.active_boards.remove(&item);
            } else {
                this.active_boards.insert(item, true);
                this.active_boards.remove(&this.last_active_board);
            }

            this.last_active_board = item;
            cx.notify();
        }
    }
    pub fn handler_project(
        &self,
        item: &ProjectItem,
    ) -> impl Fn(&mut SidebarStory, &ClickEvent, &mut Window, &mut Context<SidebarStory>) + 'static
    {
        let item = item.clone();
        move |this, _, _, cx| {
            // this.active_boards.insert(item, true);
            // this.last_active_board = item;
            this.active_project = Some(item.clone());
            cx.notify();
        }
    }
}

impl super::Mytool for SidebarStory {
    fn title() -> &'static str {
        "Sidebar"
    }

    fn description() -> &'static str {
        "A composable, themeable and customizable sidebar component."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl Focusable for SidebarStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SidebarStory {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let boards = vec![
            Item::Playground,
            Item::Models,
            Item::Documentation,
            Item::Settings,
        ];

        h_flex()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .h_full()
            .when(self.side.is_right(), |this| this.flex_row_reverse())
            .child(
                Sidebar::new(self.side)
                    .collapsed(self.collapsed)
                    .child(
                        SidebarMenu::new().children(
                            boards.iter().map(|item| {
                                SidebarMenuItem::new(item.label())
                                    .icon(item.icon())
                                    .active(self.active_boards.contains_key(item))
                                    .on_click(cx.listener(item.handler()))
                            }),
                        )
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
                    .child(
                        SidebarMenu::new().children(
                            self.projects.iter().enumerate().map(|(ix, item)| {
                                SidebarMenuItem::new(item.name.clone())
                                    // .icon(item.icon())
                                    // .active(self.last_active_item == *item)
                                    .on_click(cx.listener(self.last_active_board.handler_project(item)))
                            }),
                        ),
                    )
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
                                            this.last_active_board = Item::Playground;
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
                                    .when_some(self.active_project.clone(), move |this, subitem| {
                                        this.item(BreadcrumbItem::new("2", subitem.name))
                                    }),
                            ),
                    )
                    .child(self.render_content(window, cx)),
            )
    }
}
