pub struct ProjectBoard {
    id: ElementId,
    base: Div,
    selected: bool,
    collapsed: bool,
}
impl ProjectBoard {
    pub fn new() -> Self {
        Self {
            id: SharedString::from("ProjectBoard").into(),
            base: v_flex().gap_2().w_full(),
            selected: false,
            collapsed: false,
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
        let keyProjectBoard = self.model_keyProjectBoard;

        window.open_modal(cx, move |modal, _, _| {
            modal
                .title("Form Modal")
                .overlay(overlay)
                .keyProjectBoard(keyProjectBoard)
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
                                            .keyProjectBoard(keyProjectBoard)
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
}
