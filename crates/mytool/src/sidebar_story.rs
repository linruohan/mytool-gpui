use std::collections::HashMap;

use gpui::{
    div, impl_internal_actions, prelude::FluentBuilder, px, relative, App, AppContext, ClickEvent,
    Context, Entity, Focusable, IntoElement, ParentElement, Render, SharedString, Styled, Window,
};

use gpui_component::{
    badge::Badge,
    blue_500,
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    divider::Divider,
    h_flex,
    popup_menu::PopupMenuExt,
    sidebar::{
        Sidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem,
        SidebarToggleButton,
    },
    switch::Switch,
    v_flex, white, ActiveTheme, Icon, IconName, Side, Sizable,
};
use serde::Deserialize;

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
        v_flex()
        // .child(
        //     h_flex().gap_2().child(
        //         Switch::new("side")
        //             .label("Placement Right")
        //             .checked(self.side.is_right())
        //             .on_click(cx.listener(|this, checked: &bool, _, cx| {
        //                 this.side = if *checked { Side::Right } else { Side::Left };
        //                 cx.notify();
        //             })),
        //     ),
        // )
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
    pub fn size(&self) -> usize {
        self.label().len()
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
        // "A composable, themeable and customizable sidebar component."
        ""
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
        let groups: [Vec<Item>; 2] = [
            vec![
                Item::Inbox,
                Item::Today,
                Item::Scheduled,
                Item::Pinboard,
                Item::Labels,
                Item::Completed,
            ],
            vec![Item::Projects],
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
                    .width(px(200.))
                    .child(
                        SidebarGroup::new("header").child(SidebarMenu::new().children(
                            groups[0].iter().map(|item| {
                                SidebarMenuItem::new(format!(
                                    "{:<10}{:>10}",
                                    item.label(),
                                    item.size()
                                ))
                                .icon(item.icon())
                                .active(self.active_items.contains_key(item))
                                .on_click(cx.listener(item.handler()))
                            }),
                        )),
                    )
                    .child(SidebarGroup::new("On This Computer").child(
                        SidebarMenu::new().children(groups[1].iter().map(|item| {
                            SidebarMenuItem::new(item.label())
                                .icon(item.icon())
                                .active(self.last_active_item == *item)
                                .children(item.items().into_iter().map(|sub_item| {
                                    SidebarMenuItem::new(sub_item.label())
                                        .active(self.active_subitem == Some(sub_item))
                                        .on_click(cx.listener(sub_item.handler(&item)))
                                }))
                                .on_click(cx.listener(item.handler()))
                        })),
                    )),
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
