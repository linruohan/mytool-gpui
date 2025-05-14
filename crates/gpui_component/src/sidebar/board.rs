use gpui::{
    div, prelude::FluentBuilder as _, DefiniteLength, Div, ElementId, InteractiveElement,
    IntoElement, Length, ParentElement, RenderOnce, SharedString, Styled,
};

use crate::{
    blue_100, green_100, pink_100, popup_menu::PopupMenuExt, purple_100, red_100, v_flex,
    yellow_100, ActiveTheme as _, Collapsible, Selectable,
};

#[derive(IntoElement)]
pub struct SidebarBoard {
    id: ElementId,
    base: Div,
    selected: bool,
    collapsed: bool,
}

impl SidebarBoard {
    pub fn new() -> Self {
        Self {
            id: SharedString::from("sidebar-board").into(),
            base: v_flex().gap_2().w_full(),
            selected: false,
            collapsed: false,
        }
    }
}
impl Selectable for SidebarBoard {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn element_id(&self) -> &gpui::ElementId {
        &self.id
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
impl ParentElement for SidebarBoard {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.base.extend(elements);
    }
}
impl Styled for SidebarBoard {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}
impl PopupMenuExt for SidebarBoard {}
impl RenderOnce for SidebarBoard {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        div()
            .id(self.id)
            .flex()
            .flex_wrap()
            .size_full()
            .rounded(cx.theme().radius)
            .hover(|this| {
                this.bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .when(self.selected, |this| {
                this.bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .child(self.base)
    }
}
