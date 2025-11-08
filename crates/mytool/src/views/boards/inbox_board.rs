use crate::{Board, ItemEvent, ItemsPanel};

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    InteractiveElement as _, MouseButton, ParentElement, Render, Styled, Subscription, Window, div,
};

use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{ActiveTheme as _, IconName, Sizable, dock::PanelControl, h_flex, v_flex};

pub enum ItemClickEvent {
    ShowModal,
    ConnectionError { field1: String },
}

impl EventEmitter<ItemClickEvent> for InboxBoard {}

pub struct InboxBoard {
    _subscriptions: Vec<Subscription>,
    focus_handle: FocusHandle,
    items_panel: Entity<ItemsPanel>,
}

impl InboxBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items_panel = ItemsPanel::view(window, cx);
        let _subscriptions = vec![
            cx.subscribe(&items_panel, |this, _, event: &ItemEvent, cx| {
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
impl Board for InboxBoard {
    fn icon() -> IconName {
        IconName::MailboxSymbolic
    }
    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0x99c1f1).into(), gpui::rgb(0x3584e4).into()]
    }

    fn count() -> usize {
        1
    }
    fn title() -> &'static str {
        "Inbox"
    }

    fn description() -> &'static str {
        "所有未完成任务"
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for InboxBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InboxBoard {
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
                            .child(div().text_xl().child(<InboxBoard as Board>::title()))
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<InboxBoard as Board>::description()),
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
                                        let items_panel = self.items_panel.clone();
                                        move |_event, window, cx| {
                                            let items_panel_clone = items_panel.clone();
                                            items_panel_clone.update(cx, |items_panel, cx| {
                                                items_panel.show_item_dialog(window, cx, false);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("edit-item")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::EditSymbolic)
                                    .on_click({
                                        let items_panel = self.items_panel.clone();
                                        move |_event, window, cx| {
                                            let items_panel_clone = items_panel.clone();
                                            items_panel_clone.update(cx, |items_panel, cx| {
                                                items_panel.show_item_dialog(window, cx, true);
                                                cx.notify();
                                            })
                                        }
                                    }),
                            )
                            .child(
                                Button::new("delete-item")
                                    .icon(IconName::UserTrashSymbolic)
                                    .small()
                                    .ghost()
                                    .on_click({
                                        let items_panel = self.items_panel.clone();
                                        move |_event, window, cx| {
                                            let items_panel_clone = items_panel.clone();
                                            items_panel_clone.update(cx, |items_panel, cx| {
                                                items_panel.show_item_delete_dialog(window, cx);
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
