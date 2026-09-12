use gpui::{
    AnyElement, App, Div, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder, rems,
};
use gpui_component::{
    ActiveTheme, StyledExt,
    group_box::{GroupBox, GroupBoxVariants},
    h_flex, v_flex,
};

#[derive(IntoElement)]
pub struct StorySection {
    pub(crate) base: Div,
    pub(crate) title: SharedString,
    pub(crate) sub_title: Vec<AnyElement>,
    pub(crate) children: Vec<AnyElement>,
    pub(crate) plain: bool,
}

impl StorySection {
    pub fn sub_title(mut self, sub_title: impl IntoElement) -> Self {
        self.sub_title.push(sub_title.into_any_element());
        self
    }

    pub fn plain(mut self) -> Self {
        self.plain = true;
        self
    }

    #[allow(unused)]
    pub(crate) fn max_w_md(mut self) -> Self {
        self.base = self.base.max_w(rems(48.));
        self
    }

    #[allow(unused)]
    fn max_w_lg(mut self) -> Self {
        self.base = self.base.max_w(rems(64.));
        self
    }

    #[allow(unused)]
    fn max_w_xl(mut self) -> Self {
        self.base = self.base.max_w(rems(80.));
        self
    }

    #[allow(unused)]
    fn max_w_2xl(mut self) -> Self {
        self.base = self.base.max_w(rems(96.));
        self
    }
}

impl ParentElement for StorySection {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for StorySection {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for StorySection {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let header = h_flex()
            .justify_between()
            .w_full()
            .items_center()
            .gap_2()
            .when(!self.title.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .text_color(cx.theme().muted_foreground)
                        .child(self.title.clone()),
                )
            })
            .children(self.sub_title);

        if self.plain {
            return v_flex()
                .id(self.title.clone())
                .w_full()
                .gap_1()
                .pt_1()
                .child(header)
                .child(v_flex().w_full().items_start().justify_start().children(self.children))
                .into_any_element();
        }

        GroupBox::new()
            .id(self.title.clone())
            .outline()
            .title(header)
            .content_style(
                StyleRefinement::default()
                    .rounded(cx.theme().radius_lg)
                    .overflow_x_hidden()
                    .items_center()
                    .justify_center(),
            )
            .child(self.base.children(self.children))
            .into_any_element()
    }
}
