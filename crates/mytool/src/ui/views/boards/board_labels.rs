use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, InteractiveElement,
    ParentElement, Render, Styled, Subscription, Window,
};
use gpui_component::{
    Sizable,
    button::{Button, ButtonVariants},
    dock::PanelControl,
    h_flex, v_flex,
};
use gpui_kit::assets::IconName;

use crate::{
    LabelEvent,
    todo_state::TodoStore,
    ui::views::{
        boards::{board_common::render_board_header, container_board::Board},
        label::LabelsPanel,
    },
};

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
        vec![gpui::rgb(0xebe3d6).into(), gpui::rgb(0xa08b6e).into()]
    }

    fn count(cx: &mut App) -> usize {
        cx.global::<TodoStore>().labels.len()
    }

    fn title() -> &'static str {
        "标签"
    }

    fn description() -> &'static str {
        ""
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
        let board_count = LabelsBoard::count(cx);
        let labels_panel = self.labels_panel.clone();

        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .gap_4()
            .child(render_board_header(
                cx,
                <LabelsBoard as Board>::icon(),
                <LabelsBoard as Board>::title(),
                <LabelsBoard as Board>::description(),
                board_count,
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("add-label")
                            .small()
                            .ghost()
                            .compact()
                            .icon(IconName::PlusLargeSymbolic)
                            .on_click({
                                let labels_panel = labels_panel.clone();
                                move |_event, window, cx| {
                                    labels_panel.update(cx, |labels_panel, cx| {
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
                                let labels_panel = labels_panel.clone();
                                move |_event, window, cx| {
                                    labels_panel.update(cx, |labels_panel, cx| {
                                        labels_panel.show_label_dialog(window, cx, true);
                                        cx.notify();
                                    })
                                }
                            }),
                    )
                    .child(
                        Button::new("delete-label")
                            .small()
                            .ghost()
                            .icon(IconName::UserTrashSymbolic)
                            .on_click({
                                let labels_panel = labels_panel.clone();
                                move |_event, window, cx| {
                                    labels_panel.update(cx, |labels_panel, cx| {
                                        labels_panel.show_label_delete_dialog(window, cx);
                                        cx.notify();
                                    })
                                }
                            }),
                    ),
            ))
            .child(self.labels_panel.clone())
    }
}
