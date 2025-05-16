use bonsaidb::core::schema::view;
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
    label::Label,
    modal::{Modal, ModalButtonProps},
    switch::Switch,
    v_flex, ContextModal as _,
};

use crate::section;
actions!(modal_story, [TestAction]);
#[derive(Debug, Clone)]
pub struct ProjectStory {
    focus_handle: FocusHandle,
    selected_value: Option<SharedString>,
    project_name_input: Entity<TextInput>,
    date_picker: Entity<DatePicker>,
    is_use_emoji: bool,
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
        let project_input =
            cx.new(|cx| TextInput::new(window, cx).placeholder("Give your project a name"));
        let date_picker = cx.new(|cx| {
            DatePicker::new("duedate-picker", window, cx).placeholder("Duedate of project")
        });

        Self {
            focus_handle: cx.focus_handle(),
            selected_value: None,
            project_name_input: project_input,
            date_picker,
            modal_overlay: false,
            model_show_close: true,
            model_padding: true,
            model_keyboard: true,
            overlay_closable: true,
            is_use_emoji: false,
        }
    }

    fn show_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let overlay = self.modal_overlay;
        let modal_show_close = self.model_show_close;
        let modal_padding = self.model_padding;
        let overlay_closable = self.overlay_closable;
        let project_name_input = self.project_name_input.clone();
        let date_picker = self.date_picker.clone();
        let view = cx.entity().clone();
        let keyboard = self.model_keyboard;
        let is_use_emoji = self.is_use_emoji;

        window.open_modal(cx, move |modal, _, _| {
            modal
                .title("New Project")
                .overlay(overlay)
                .keyboard(keyboard)
                .show_close(modal_show_close)
                .overlay_closable(overlay_closable)
                .when(!modal_padding, |this| this.p(px(0.)))
                .child(
                    v_flex()
                        .gap_3()
                        .child("💼")
                        .child(project_name_input.clone())
                        .child(h_flex().gap_3().child(Label::new("😄 Use Emoji")).child(
                            Switch::new("is-use-emoji").checked(is_use_emoji), // .on_click(
                                                                               //     cx.listener(move |view, checked, _, cx| {
                                                                               //         let view = view.clone();
                                                                               //         view.is_use_emoji = *checked;
                                                                               //         cx.notify();
                                                                               //     }),
                                                                               // ),
                        ))
                        .child(date_picker.clone()),
                )
                .footer({
                    let view = view.clone();
                    let project_name_input = project_name_input.clone();
                    let date_picker = date_picker.clone();
                    move |_, _, _, _cx| {
                        vec![
                            Button::new("confirm").primary().label("Confirm").on_click({
                                let view = view.clone();
                                let project_name_input = project_name_input.clone();
                                let date_picker = date_picker.clone();
                                move |_, window, cx| {
                                    window.close_modal(cx);

                                    view.update(cx, |view, cx| {
                                        view.selected_value = Some(
                                            format!(
                                                "Hello, {}, date: {}",
                                                project_name_input.read(cx).text(),
                                                date_picker.read(cx).date()
                                            )
                                            .into(),
                                        );
                                        println!("{:?}", view.selected_value.as_ref().unwrap());
                                    });
                                }
                            }),
                            Button::new("cancel")
                                .label("Cancel")
                                .on_click(move |_, window, cx| {
                                    window.close_modal(cx);
                                }),
                        ]
                    }
                })
        });

        self.project_name_input.focus_handle(cx).focus(window);
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
                    section("Section").child(
                        Button::new("new-project").label("Add Project").on_click(
                            cx.listener(|this, _, window, cx| this.show_modal(window, cx)),
                        ),
                    ),
                ),
            )
    }
}
