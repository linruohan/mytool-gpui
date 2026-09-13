use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, InteractiveElement,
    ParentElement, Render, Styled, Subscription, Window, prelude::FluentBuilder,
};
use gpui_component::{
    Sizable,
    button::{Button, ButtonVariants},
    dock::PanelControl,
    h_flex,
    menu::{DropdownMenu, PopupMenuItem},
    v_flex,
};
use gpui_kit::assets::IconName;
use rust_i18n::t;

use crate::{
    LabelEvent,
    todo_state::TodoStore,
    ui::views::{
        boards::{board_common::render_board_header, board_renderer, container_board::Board},
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

    fn title() -> String {
        t!("todo.board.labels").to_string()
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
        let has_selected_label = self.labels_panel.read(cx).active_index.is_some();

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
                            .tooltip(t!("todo.label.new").to_string())
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
                    .when(has_selected_label, |this| {
                        this.child(
                            Button::new("label-more")
                                .small()
                                .ghost()
                                .compact()
                                .icon(IconName::EllipsisVertical)
                                .tooltip(t!("todo.more").to_string())
                                .dropdown_menu({
                                    let labels_panel = labels_panel.clone();
                                    move |this, window, _cx| {
                                        let labels_panel = labels_panel.clone();
                                        this.item(
                                            PopupMenuItem::new(t!("todo.label.edit").to_string())
                                                .on_click(window.listener_for(
                                                    &labels_panel,
                                                    |this, _, window, cx| {
                                                        this.show_label_dialog(window, cx, true);
                                                        cx.notify();
                                                    },
                                                )),
                                        )
                                        .separator()
                                        .item(
                                            PopupMenuItem::new(t!("todo.label.delete").to_string())
                                                .on_click(window.listener_for(
                                                    &labels_panel,
                                                    |this, _, window, cx| {
                                                        this.show_label_delete_dialog(window, cx);
                                                        cx.notify();
                                                    },
                                                )),
                                        )
                                    }
                                }),
                        )
                    }),
            ))
            .when(board_count == 0, |this| {
                this.child(board_renderer::render_empty_placeholder(
                    cx,
                    LabelsBoard::icon(),
                    t!("todo.empty.labels_title").to_string(),
                    t!("todo.empty.labels_hint").to_string(),
                ))
            })
            .when(board_count > 0, |this| this.child(self.labels_panel.clone()))
    }
}
