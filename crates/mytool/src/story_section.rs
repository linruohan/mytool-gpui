use gpui::{
    AnyElement, App, Div, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement,
    Styled, Window, rems,
};
use gpui_component::{
    ActiveTheme,
    group_box::{GroupBox, GroupBoxVariants},
    h_flex,
};

#[derive(IntoElement)]
pub struct StorySection {
    pub(crate) base: Div,
    pub(crate) title: SharedString,
    pub(crate) sub_title: Vec<AnyElement>,
    pub(crate) children: Vec<AnyElement>,
}

impl StorySection {
    pub fn sub_title(mut self, sub_title: impl IntoElement) -> Self {
        self.sub_title.push(sub_title.into_any_element());
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
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for StorySection {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        GroupBox::new()
            .id(self.title.clone())
            .outline()
            .title(
                h_flex()
                    .justify_between()
                    .w_full()
                    .gap_4()
                    .child(self.title)
                    .children(self.sub_title),
            )
            .content_style(
                StyleRefinement::default()
                    .rounded(cx.theme().radius_lg)
                    .overflow_x_hidden()
                    .items_center()
                    .justify_center(),
            )
            .child(self.base.children(self.children))
    }
}
