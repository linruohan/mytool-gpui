use gpui::{
    actions, div, prelude::FluentBuilder as _, px, App, AppContext, Context, Entity, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, ParentElement, Render, SharedString, Styled,
    Window,
};

use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    date_picker::DatePicker,
    dropdown::Dropdown,
    h_flex,
    input::TextInput,
    modal::ModalButtonProps,
    v_flex, ContextModal as _,
};

use crate::section;
actions!(modal_story, [TestAction]);
pub struct ProjectStory {
    focus_handle: FocusHandle,
    selected_value: Option<SharedString>,
    input1: Entity<TextInput>,
    input2: Entity<TextInput>,
    date_picker: Entity<DatePicker>,
    dropdown: Entity<Dropdown<Vec<String>>>,
    modal_overlay: bool,
    model_show_close: bool,
    model_padding: bool,
    model_keyboard: bool,
    overlay_closable: bool,
}

impl crate::Mytool for ProjectStory {
    fn title() -> &'static str {
        "Project"
    }

    fn description() -> &'static str {
        "A project dialog"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl ProjectStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input1 = cx.new(|cx| TextInput::new(window, cx).placeholder("Your Name"));
        let input2 = cx.new(|cx| {
            TextInput::new(window, cx).placeholder("For test focus back on modal close.")
        });
        let date_picker = cx
            .new(|cx| DatePicker::new("birthday-picker", window, cx).placeholder("Date of Birth"));
        let dropdown = cx.new(|cx| {
            Dropdown::new(
                "dropdown1",
                vec![
                    "Option 1".to_string(),
                    "Option 2".to_string(),
                    "Option 3".to_string(),
                ],
                None,
                window,
                cx,
            )
        });

        Self {
            focus_handle: cx.focus_handle(),
            selected_value: None,
            input1,
            input2,
            date_picker,
            dropdown,
            modal_overlay: true,
            model_show_close: true,
            model_padding: true,
            model_keyboard: true,
            overlay_closable: true,
        }
    }

    fn show_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let overlay = self.modal_overlay;
        let modal_show_close = self.model_show_close;
        let modal_padding = self.model_padding;
        let overlay_closable = self.overlay_closable;
        let input1 = self.input1.clone();
        let date_picker = self.date_picker.clone();
        let dropdown = self.dropdown.clone();
        let view = cx.entity().clone();
        let keyboard = self.model_keyboard;

        window.open_modal(cx, move |modal, _, _| {
            modal
                .title("Form Modal")
                .overlay(overlay)
                .keyboard(keyboard)
                .show_close(modal_show_close)
                .overlay_closable(overlay_closable)
                .when(!modal_padding, |this| this.p(px(0.)))
                .child(
                    v_flex()
                        .gap_3()
                        .child("This is a modal dialog.")
                        .child("You can put anything here.")
                        .child(input1.clone())
                        .child(dropdown.clone())
                        .child(date_picker.clone()),
                )
                .footer({
                    let view = view.clone();
                    let input1 = input1.clone();
                    let date_picker = date_picker.clone();
                    move |_, _, _, _cx| {
                        vec![
                            Button::new("confirm").primary().label("Confirm").on_click({
                                let view = view.clone();
                                let input1 = input1.clone();
                                let date_picker = date_picker.clone();
                                move |_, window, cx| {
                                    window.close_modal(cx);

                                    view.update(cx, |view, cx| {
                                        view.selected_value = Some(
                                            format!(
                                                "Hello, {}, date: {}",
                                                input1.read(cx).text(),
                                                date_picker.read(cx).date()
                                            )
                                            .into(),
                                        )
                                    });
                                }
                            }),
                            Button::new("new-modal").label("Open Other Modal").on_click(
                                move |_, window, cx| {
                                    window.open_modal(cx, move |modal, _, _| {
                                        modal
                                            .title("Other Modal")
                                            .child("This is another modal.")
                                            .min_h(px(300.))
                                            .overlay(overlay)
                                            .keyboard(keyboard)
                                            .show_close(modal_show_close)
                                            .overlay_closable(overlay_closable)
                                            .when(!modal_padding, |this| this.p(px(0.)))
                                    });
                                },
                            ),
                            Button::new("cancel")
                                .label("Cancel")
                                .on_click(move |_, window, cx| {
                                    window.close_modal(cx);
                                }),
                        ]
                    }
                })
        });

        self.input1.focus_handle(cx).focus(window);
    }

    fn on_action_test_action(
        &mut self,
        _: &TestAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.push_notification("You have clicked the TestAction.", cx);
    }
}

impl Focusable for ProjectStory {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ProjectStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("project-story")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_test_action))
            .size_full()
            .child(
                v_flex().gap_6().child(
                    section("Normal Modal").child(
                        Button::new("show-modal").label("Open Modal...").on_click(
                            cx.listener(|this, _, window, cx| this.show_modal(window, cx)),
                        ),
                    ),
                ),
            )
    }
}
