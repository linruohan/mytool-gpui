use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, InteractiveElement,
    MouseButton, ParentElement, Render, Styled, Subscription, Window, div,
};

use crate::{Board, ItemCompletedEvent, ItemsCompletedPanel};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{ActiveTheme, IconName, Sizable, dock::PanelControl, h_flex, v_flex};

pub struct CompletedBoard {
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
    items_panel: Entity<ItemsCompletedPanel>,
}

impl CompletedBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items_panel = ItemsCompletedPanel::view(window, cx);
        let _subscriptions =
            vec![
                cx.subscribe(&items_panel, |this, _, event: &ItemCompletedEvent, cx| {
                    this.items_panel.update(cx, |panel, cx| {
                        panel.handle_item_event(event, cx);
                    });
                }),
            ];
        Self {
            focus_handle: cx.focus_handle(),
            _subscriptions,
            items_panel,
        }
    }
}
impl Board for CompletedBoard {
    fn icon() -> IconName {
        IconName::CheckRoundOutlineSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xffbe6f).into(), gpui::rgb(0xff7800).into()]
    }

    fn count() -> usize {
        1
    }

    fn title() -> &'static str {
        "Completed"
    }

    fn description() -> &'static str {
        "已完成任务"
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for CompletedBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CompletedBoard {
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
                            .child(div().text_xl().child(<CompletedBoard as Board>::title()))
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<CompletedBoard as Board>::description()),
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
                                Button::new("finish-delete")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::Delete)
                                    .on_click({
                                        let items_panel = self.items_panel.clone();
                                        move |_event, window, cx| {
                                            let items_panel_clone = items_panel.clone();
                                            items_panel_clone.update(cx, |items_panel, cx| {
                                                items_panel.show_unfinish_item_dialog(window, cx);
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
