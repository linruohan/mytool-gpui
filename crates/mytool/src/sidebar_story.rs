use std::collections::HashMap;

use gpui::{
    blue, green, impl_internal_actions, prelude::FluentBuilder, px, App, AppContext, ClickEvent,
    Context, Entity, Focusable, Hsla, IntoElement, ParentElement, Render, SharedString, Styled,
    Window,
};

use crate::play_ogg_file;
use gpui_component::{
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    divider::Divider,
    gray_400, h_flex, purple_100, red_400,
    sidebar::{
        Sidebar, SidebarBoard, SidebarBoardItem, SidebarMenu, SidebarMenuItem, SidebarToggleButton,
    },
    switch::Switch,
    v_flex, yellow_400, ActiveTheme, IconName, Side,
};
use serde::Deserialize;
use todos::enums::object_type::ObjectType;

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct SelectCompany(SharedString);

impl_internal_actions!(sidebar_story, [SelectCompany]);

pub struct SidebarStory {
    active_items: HashMap<Item, bool>,
    last_active_item: Item,
    active_subitem: Option<SubItem>,
    collapsed: bool,
    side: Side,
    focus_handle: gpui::FocusHandle,
    checked: bool,
}

impl SidebarStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut active_items = HashMap::new();
        active_items.insert(Item::Inbox, true);

        Self {
            active_items,
            last_active_item: Item::Inbox,
            active_subitem: None,
            collapsed: false,
            side: Side::Left,
            focus_handle: cx.focus_handle(),
            checked: false,
        }
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
    Inbox,
    Today,
    Scheduled,
    Pinboard,
    Labels,
    Completed,
    Projects,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubItem {
    History,
    Starred,
    Settings,
}

impl Item {
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
            Self::Projects => self.items().len(),
        }
    }
    pub fn color(&self) -> Hsla {
        match self {
            // Self::Inbox => gpui::rgb(0xf0f0f0).into(),
            Self::Inbox => blue(),
            Self::Today => green(),
            Self::Scheduled => purple_100(),
            Self::Pinboard => red_400(),
            Self::Labels => gray_400(),
            Self::Completed => yellow_400(),
            Self::Projects => Hsla::default(),
        }
    }

    pub fn handler(
        &self,
    ) -> impl Fn(&mut SidebarStory, &ClickEvent, &mut Window, &mut Context<SidebarStory>) + 'static
    {
        let item = *self;
        move |this, _, _, cx| {
            if this.active_items.contains_key(&item) {
                this.active_items.remove(&item);
            } else {
                this.active_items.insert(item, true);
                this.active_items.remove(&this.last_active_item);
            }

            this.last_active_item = item;
            cx.notify();
        }
    }
    pub fn items(&self) -> Vec<SubItem> {
        match self {
            Self::Projects => vec![SubItem::History, SubItem::Starred, SubItem::Settings],
            _ => Vec::new(),
        }
    }
}

impl SubItem {
    pub fn label(&self) -> &'static str {
        match self {
            Self::History => "History",
            Self::Starred => "Starred",
            Self::Settings => "Settings",
        }
    }

    pub fn handler(
        &self,
        item: &Item,
    ) -> impl Fn(&mut SidebarStory, &ClickEvent, &mut Window, &mut Context<SidebarStory>) + 'static
    {
        let item = *item;
        let subitem = *self;
        move |this, _, _, cx| {
            println!(
                "Clicked on item: {}, child: {}",
                item.label(),
                subitem.label()
            );
            this.active_items.insert(item, true);
            this.last_active_item = item;
            this.active_subitem = Some(subitem);
            cx.notify();
        }
    }
}

impl super::Mytool for SidebarStory {
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
        let item_groups = vec![
            Item::Inbox,
            Item::Today,
            Item::Scheduled,
            Item::Pinboard,
            Item::Labels,
            Item::Completed,
        ];
        let projects = Item::Projects;
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
                                            .active(self.active_items.contains_key(item))
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
                                .on_click(cx.listener(move |_this, _, _, cx| {
                                    // let projects = projects.read(cx);
                                    println!("{}", "add projects");
                                    play_ogg_file("assets/sounds/success.ogg").ok();
                                    projects.items().push(SubItem::History);
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        // SidebarGroup::new("Projects").child(),
                        // 项目列表：
                        SidebarMenu::new().children(projects.items().into_iter().enumerate().map(
                            |(_, project)| {
                                SidebarMenuItem::new(project.label())
                                    .active(self.active_subitem == Some(project))
                                    .on_click(cx.listener(project.handler(&projects)))
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
                                            this.last_active_item = Item::Inbox;
                                            cx.notify();
                                        },
                                    )))
                                    .item(
                                        BreadcrumbItem::new("1", self.last_active_item.label())
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.active_subitem = None;
                                                cx.notify();
                                            })),
                                    )
                                    .when_some(self.active_subitem, |this, subitem| {
                                        this.item(BreadcrumbItem::new("2", subitem.label()))
                                    }),
                            ),
                    )
                    .child(self.render_content(window, cx)),
            )
    }
}
