use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, InteractiveElement,
    MouseButton, ParentElement, Render, Styled, Subscription, Window, div,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable,
    button::{Button, ButtonVariants},
    dock::PanelControl,
    h_flex, v_flex,
};

use crate::{Board, ItemsTodayEvent, ItemsTodayPanel};

pub struct TodayBoard {
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
    items_panel: Entity<ItemsTodayPanel>,
}

impl TodayBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items_panel = ItemsTodayPanel::view(window, cx);
        let _subscriptions =
            vec![cx.subscribe(&items_panel, |this, _, event: &ItemsTodayEvent, cx| {
                this.items_panel.update(cx, |panel, cx| {
                    panel.handle_today_event(event, cx);
                });
            })];
        Self { focus_handle: cx.focus_handle(), _subscriptions, items_panel }
    }
}
impl Board for TodayBoard {
    fn icon() -> IconName {
        IconName::StarOutlineThickSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0x33d17a).into(), gpui::rgb(0x33d17a).into()]
    }

    fn count() -> usize {
        1
    }

    fn title() -> &'static str {
        "Today"
    }

    fn description() -> &'static str {
        "今天需要完成的任务"
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for TodayBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TodayBoard {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .gap_4()
            .child(
                h_flex()
                    .id("header")
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .justify_between()
                    .items_start()
                    .child(
                        v_flex()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(<TodayBoard as Board>::icon())
                                    .child(div().text_base().child(<TodayBoard as Board>::title())),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<TodayBoard as Board>::description()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .px_2()
                            .gap_2()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                Button::new("finish-today")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::CheckRoundOutlineSymbolic)
                                    .on_click({
                                        let items_panel = self.items_panel.clone();
                                        move |_event, window, cx| {
                                            let items_panel_clone = items_panel.clone();
                                            items_panel_clone.update(cx, |items_panel, cx| {
                                                items_panel.show_finish_item_dialog(window, cx);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            ),
                    ),
            )
            .child(self.items_panel.clone())
    }
}
