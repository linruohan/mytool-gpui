use gpui::{div, App, AppContext, ClickEvent, Context, Entity, FocusHandle, Focusable, Hsla, InteractiveElement, ParentElement, Render, Styled, Subscription, Window};

use crate::{Board, LabelEvent, LabelsPanel};
use gpui_component::button::Button;
use gpui_component::{dock::PanelControl, h_flex, v_flex, ActiveTheme, IconName};

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
            vec![
                cx.subscribe(&labels_panel, |this, _, event: &LabelEvent, cx| {
                    this.labels_panel.update(cx, |panel, cx| {
                        panel.handle_label_event(event, cx);
                    });
                }),
            ];
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                // Update results panel
                this.labels_panel.update(cx, |panel, cx| {
                    panel.get_labels(cx);
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
        Self {
            focus_handle: cx.focus_handle(),
            _subscriptions,
            labels_panel,
        }
    }
}
impl Board for LabelsBoard {
    fn icon() -> IconName {
        IconName::TagOutlineSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xcdab8f).into(), gpui::rgb(0x986a44).into()]
    }

    fn count() -> usize {
        1
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
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .overflow_x_hidden()
            .child(
                h_flex()
                    .id("header")
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .justify_between()
                    .items_start()
                    .child(
                        v_flex()
                            .child(div().text_xl().child(<LabelsBoard as Board>::title()))
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<LabelsBoard as Board>::description()),
                            ),
                    ),
            )
            .child(Button::new("update").label("更新labels").on_click(
                cx.listener(
                    move |this, _: &ClickEvent, _, cx| {
                        this.labels_panel.update(cx, |panel, cx| {
                            cx.emit(LabelEvent::Loaded);
                            cx.notify();
                        });
                    },
                )
            ))
            .child(self.labels_panel.clone())
    }
}
