use crate::{
    h_flex, label::Label, red_100, v_flex, ActiveTheme as _, Collapsible, Icon, IconName, StyledExt,
};
use gpui::{
    div, percentage, prelude::FluentBuilder as _, px, relative, yellow, AnyElement, App,
    ClickEvent, ElementId, Hsla, InteractiveElement as _, IntoElement, Length, ParentElement as _,
    RenderOnce, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
};
use std::rc::Rc;

#[derive(IntoElement)]
pub struct SidebarBoard {
    collapsed: bool,
    items: Vec<SidebarBoardItem>,
}

impl SidebarBoard {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            collapsed: false,
        }
    }

    pub fn child(mut self, child: impl Into<SidebarBoardItem>) -> Self {
        self.items.push(child.into());
        self
    }

    pub fn children(
        mut self,
        children: impl IntoIterator<Item = impl Into<SidebarBoardItem>>,
    ) -> Self {
        self.items = children.into_iter().map(Into::into).collect();
        self
    }
}
impl Collapsible for SidebarBoard {
    fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}
impl RenderOnce for SidebarBoard {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        h_flex()
            .flex()
            .flex_wrap()
            .size_full()
            .h(px(115.))
            .children(
                self.items
                    .into_iter()
                    .enumerate()
                    .map(|(ix, item)| item.id(ix).collapsed(self.collapsed)),
            )
    }
}

/// A sidebar menu item
#[derive(IntoElement)]
pub struct SidebarBoardItem {
    id: ElementId,
    icon: Option<Icon>,
    label: SharedString,
    handler: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
    active: bool,
    collapsed: bool,
    board_bg: Hsla,
    board_text_color: Hsla,
    board_count: usize,
    size: Length,
    children: Vec<Self>,
    suffix: Option<AnyElement>,
}

impl SidebarBoardItem {
    /// Create a new SidebarBoardItem with a label
    pub fn new(label: impl Into<SharedString>, bg: Hsla, color: Hsla, count: usize) -> Self {
        Self {
            id: ElementId::Integer(0),
            icon: None,
            label: label.into(),
            handler: Rc::new(|_, _, _| {}),
            active: false,
            board_bg: bg,
            board_count: count,
            board_text_color: color,
            size: Length::Auto,
            collapsed: false,
            children: Vec::new(),
            suffix: None,
        }
    }

    /// Set the icon for the menu item
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }
    /// Set id to the menu item.
    fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }
    /// Set id to the menu item.
    pub fn bg(mut self, id: Hsla) -> Self {
        self.board_bg = id;
        self
    }
    pub fn count(mut self, count: usize) -> Self {
        self.board_count = count;
        self
    }
    pub fn color(mut self, color: Hsla) -> Self {
        self.board_text_color = color;
        self
    }
    pub fn size(mut self, size: Length) -> Self {
        self.size = size;
        self
    }

    /// Set the active state of the menu item
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Add a click handler to the menu item
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.handler = Rc::new(handler);
        self
    }

    /// Set the collapsed state of the menu item
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Self>>) -> Self {
        self.children = children.into_iter().map(Into::into).collect();
        self
    }

    /// Set the suffix for the menu item.
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }
}

impl RenderOnce for SidebarBoardItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let handler = self.handler.clone();
        let is_collapsed = self.collapsed;
        let is_active = self.active;
        let size = self.size;
        let board_text_color = self.board_text_color;
        let board_count = self.board_count;

        v_flex()
            .id(self.id.clone())
            .gap_2()
            .p_2()
            .w_full()
            .justify_between()
            .rounded(cx.theme().radius)
            .hover(|this| {
                this.bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .size(size)
            .bg(self.board_bg.opacity(0.15))
            .rounded(cx.theme().radius)
            .child(
                v_flex()
                    .id("item")
                    .when(is_collapsed, |this| {
                        this.justify_center().when(is_active, |this| {
                            this.bg(cx.theme().sidebar_accent)
                                .text_color(cx.theme().sidebar_accent_foreground)
                        })
                    })
                    .when(!is_collapsed, |this| {
                        this // 父容器填满可用空间
                            .h_6()
                            .flex() // 启用 Flex 布局
                            .flex_col()
                            .flex_shrink_0()
                            .gap_0()
                            .text_sm()
                            .flex_1()
                            .line_height(relative(1.25))
                            .overflow_hidden()
                            .text_ellipsis()
                            .children([
                                // 第一行（上边）
                                div()
                                    .flex() // 启用 Flex 布局
                                    .justify_between() // 子元素横向两端对齐
                                    .children([
                                        div().when_some(self.icon.clone(), |this, icon| {
                                            this.child(icon.text_color(self.board_text_color))
                                        }),
                                        div().when(self.board_count.clone() > 0, |this| {
                                            this.child(
                                                Label::new(board_count.to_string())
                                                    .text_right()
                                                    .text_color(self.board_text_color),
                                            )
                                        }),
                                    ]),
                                // 第二行（下边）
                                div().flex().justify_between().children([
                                    div().child(
                                        Label::new(self.label.clone())
                                            .size(Length::Definite(gpui::DefiniteLength::Fraction(
                                                0.5,
                                            )))
                                            .text_left()
                                            .text_color(board_text_color),
                                    ), // 左下角
                                    div().when(is_active, |this| {
                                        this.child(
                                            Label::new("🔴")
                                                .text_right()
                                                .text_color(board_text_color),
                                        )
                                    }), // 右下角
                                ]),
                            ])
                    })
                    .on_click(move |ev, window, cx| handler(ev, window, cx)),
            )
    }
}
