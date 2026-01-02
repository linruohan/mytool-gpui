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

use crate::{Board, LabelEvent, LabelsPanel, todo_state::LabelState};

pub struct LabelsBoard {
    _subscriptions: Vec<Subscription>,
    focus_handle: FocusHandle,
    pub labels_panel: Entity<LabelsPanel>,
}

impl LabelsBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let labels_panel = LabelsPanel::view(window, cx);
        let _subscriptions =
            vec![cx.subscribe(&labels_panel, |this, _, event: &LabelEvent, cx| {
                this.labels_panel.update(cx, |panel, cx| {
                    panel.handle_label_event(event, cx);
                });
            })];
        Self { focus_handle: cx.focus_handle(), _subscriptions, labels_panel }
    }
}
impl Board for LabelsBoard {
    fn icon() -> IconName {
        IconName::TagOutlineSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xcdab8f).into(), gpui::rgb(0x986a44).into()]
    }

    fn count(cx: &mut App) -> usize {
        cx.global::<LabelState>().labels.len()
    }

    fn title() -> &'static str {
        "Labels"
    }

    fn description() -> &'static str {
        "所有的标签"
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for LabelsBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LabelsBoard {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
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
                                h_flex().gap_2().child(<LabelsBoard as Board>::icon()).child(
                                    div().text_base().child(<LabelsBoard as Board>::title()),
                                ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<LabelsBoard as Board>::description()),
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
                                Button::new("add-label")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::PlusLargeSymbolic)
                                    .on_click({
                                        let labels_panel = self.labels_panel.clone();
                                        move |_event, window, cx| {
                                            let labels_panel_clone = labels_panel.clone();
                                            labels_panel_clone.update(cx, |labels_panel, cx| {
                                                labels_panel.show_label_dialog(window, cx, false);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("edit-label")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::EditSymbolic)
                                    .on_click({
                                        let labels_panel = self.labels_panel.clone();
                                        move |_event, window, cx| {
                                            let labels_panel_clone = labels_panel.clone();
                                            labels_panel_clone.update(cx, |labels_panel, cx| {
                                                labels_panel.show_label_dialog(window, cx, true);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("delete-label")
                                    .icon(IconName::UserTrashSymbolic)
                                    .small()
                                    .ghost()
                                    .on_click({
                                        let labels_panel = self.labels_panel.clone();
                                        move |_event, window, cx| {
                                            let labels_panel_clone = labels_panel.clone();
                                            labels_panel_clone.update(cx, |labels_panel, cx| {
                                                labels_panel.show_label_delete_dialog(window, cx);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            ),
                    ),
            )
            .child(self.labels_panel.clone())
    }
}
