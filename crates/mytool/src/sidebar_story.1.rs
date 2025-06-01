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
use todos::objects::filters::{
    all_items::AllItems, completed::Completed, labels::Labels, pinboard::Pinboard,
    scheduled::Scheduled, today::Today,
};
use todos::objects::project::ProjectLogic;
#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct SelectCompany(SharedString);

impl_internal_actions!(sidebar_story, [SelectCompany]);

pub struct SidebarStory {
    active_items: Vec<ViewItem>,
    last_active_item: ViewItem,
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
        let mut active_items = Vec::new();
        let inbox = ViewItem::Inbox(AllItems::default());
        active_items.insert(0, inbox.clone());

        Self {
            active_items,
            last_active_item: inbox.clone(),
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

#[derive(Clone, PartialEq, Eq)]
enum ViewItem {
    Inbox(AllItems),
    Today(Today),
    Scheduled(Scheduled),
    Pinboard(Pinboard),
    Labels(Labels),
    Completed(Completed),
    Projects(ProjectLogic),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubItem {
    History,
    Starred,
    Settings,
}

impl ViewItem {
    pub fn label(&self) -> String {
        match self {
            ViewItem::Inbox(_) => "Inbox".to_string(),
            ViewItem::Today(Today { base, .. })
            | ViewItem::Scheduled(Scheduled { base, .. })
            | ViewItem::Pinboard(Pinboard { base, .. })
            | ViewItem::Labels(Labels { base, .. })
            | ViewItem::Completed(Completed { base, .. })
            | ViewItem::Projects(ProjectLogic { base, .. }) => base.name().to_string(),
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            ViewItem::Inbox(_) => IconName::MailboxSymbolic,
            ViewItem::Today(Today { base, .. })
            | ViewItem::Scheduled(Scheduled { base, .. })
            | ViewItem::Pinboard(Pinboard { base, .. })
            | ViewItem::Labels(Labels { base, .. })
            | ViewItem::Completed(Completed { base, .. })
            | ViewItem::Projects(ProjectLogic { base, .. }) => {
                IconName::from_str(&base.icon_name().to_string())
            }
        }
    }
    pub fn count(&self) -> usize {
        match self {
            ViewItem::Inbox(_) => 0,
            ViewItem::Today(today) => today.count(),
            ViewItem::Scheduled(s) => s.count(),
            ViewItem::Pinboard(s) => s.pinboard_count(),
            ViewItem::Labels(s) => s.count(),
            ViewItem::Completed(s) => s.count(),
            ViewItem::Projects(s) => s.project_count(),
        }
    }
    pub fn color(&self) -> Hsla {
        match self {
            // Self::Inbox => gpui::rgb(0xf0f0f0).into(),
            Self::Inbox(_) => blue(),
            Self::Today(_) => green(),
            Self::Scheduled(_) => purple_100(),
            Self::Pinboard(_) => red_400(),
            Self::Labels(_) => gray_400(),
            Self::Completed(_) => yellow_400(),
            Self::Projects(_) => Hsla::default(),
        }
    }

    pub fn handler(
        &self,
    ) -> impl Fn(&mut SidebarStory, &ClickEvent, &mut Window, &mut Context<SidebarStory>) + 'static
    {
        let item = self.clone();
        move |this, _, _, cx| {
            if this.active_items.contains(&item) {
                // 存在则移除
                this.active_items.retain(|x| *x != item);
            } else {
                // 不存在则添加
                this.active_items.push(item.clone());

                // 移除上一次活动的项目
                this.active_items.retain(|x| x != &this.last_active_item);
            }

            this.last_active_item = item.clone();
            cx.notify();
        }
    }
    pub fn items(&self) -> Vec<SubItem> {
        match self {
            ViewItem::Projects(_) => vec![SubItem::History, SubItem::Starred, SubItem::Settings],
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
        item: &ViewItem,
    ) -> impl Fn(&mut SidebarStory, &ClickEvent, &mut Window, &mut Context<SidebarStory>) + 'static
    {
        let item = item.clone();
        let subitem = *self;
        move |this, _, _, cx| {
            println!(
                "Clicked on item: {}, child: {}",
                item.label(),
                subitem.label()
            );
            this.active_items.push(item.clone());
            this.last_active_item = item.clone();
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
            ViewItem::Inbox(AllItems::default()),
            ViewItem::Today(Today::default()),
            ViewItem::Scheduled(Scheduled::default()),
            ViewItem::Pinboard(Pinboard::default()),
            ViewItem::Labels(Labels::default()),
            ViewItem::Completed(Completed::default()),
        ];
        let projects = ViewItem::Projects(ProjectLogic::default());
        // let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));

        let projects_for_add = projects.clone();
        let projects_for_menu = projects.clone();

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
                                            .active(self.active_items.contains(item))
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
                                    projects_for_add.items().push(SubItem::History);
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        // SidebarGroup::new("Projects").child(),
                        // 项目列表：
                        SidebarMenu::new().children(
                            projects_for_menu
                                .clone()
                                .items()
                                .into_iter()
                                .enumerate()
                                .map(|(_, project)| {
                                    SidebarMenuItem::new(project.label())
                                        .active(self.active_subitem == Some(project))
                                        .on_click(cx.listener(project.handler(&projects_for_menu)))
                                }),
                        ),
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
                                            this.last_active_item =
                                                ViewItem::Inbox(AllItems::default());
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
