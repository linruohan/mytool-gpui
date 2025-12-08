use gpui::{
    AnyView, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, div,
};
use gpui_component::{Root, v_flex};

use crate::AppTitleBar;

pub(crate) struct StoryRoot {
    title_bar: Entity<AppTitleBar>,
    view: AnyView,
}

impl StoryRoot {
    pub fn new(
        title: impl Into<SharedString>,
        view: impl Into<AnyView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new(title, window, cx));
        Self { title_bar, view: view.into() }
    }
}

impl Render for StoryRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sheet_layer = Root::render_sheet_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(self.title_bar.clone())
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())),
            )
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer)
    }
}
