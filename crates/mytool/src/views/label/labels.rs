use super::label_source_row::{Label, LabelListDelegate, SelectedLabel};
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Subscription, Window, div,
};
use gpui_component::{
    ActiveTheme, Sizable,
    button::Button,
    checkbox::Checkbox,
    h_flex,
    list::{List, ListDelegate, ListEvent},
    v_flex,
};
pub struct LabelsView {
    focus_handle: FocusHandle,
    labels_list: Entity<List<LabelListDelegate>>,
    selected_label: Option<Label>,
    _subscriptions: Vec<Subscription>,
}

impl crate::Mytool for LabelsView {
    fn title() -> &'static str {
        "Labels"
    }

    fn description() -> &'static str {
        "A label displays a series of items."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl LabelsView {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let labels = (0..1_000)
            .map(|_| random_label())
            .collect::<Vec<LabelItem>>();

        let delegate = LabelListDelegate {
            matched_labels: labels.clone(),
            labels,
            selected_index: Some(0),
            confirmed_index: None,
            query: "".to_string(),
            loading: false,
            eof: false,
        };

        let company_list = cx.new(|cx| List::new(delegate, window, cx));
        // company_list.update(cx, |list, cx| {
        //     list.set_selected_index(Some(3), cx);
        // });
        let _subscriptions =
            vec![
                cx.subscribe(&company_list, |_, _, ev: &ListEvent, _| match ev {
                    ListEvent::Select(ix) => {
                        println!("List Selected: {:?}", ix);
                    }
                    ListEvent::Confirm(ix) => {
                        println!("List Confirmed: {:?}", ix);
                    }
                    ListEvent::Cancel => {
                        println!("List Cancelled");
                    }
                }),
            ];

        // Spawn a background to random refresh the list
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                this.company_list.update(cx, |picker, _| {
                    picker
                        .delegate_mut()
                        .companies
                        .iter_mut()
                        .for_each(|company| {
                            company.random_update();
                        });
                });
                cx.notify();
            })
            .ok();
        })
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            labels_list: company_list,
            selected_label: None,
            _subscriptions,
        }
    }

    fn selected_company(&mut self, _: &SelectedLabel, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.labels_list.read(cx);
        if let Some(company) = picker.delegate().selected_label() {
            self.selected_label = Some(company);
        }
    }
}

impl Focusable for LabelsView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LabelsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_company))
            .size_full()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        Button::new("scroll-top")
                            .child("Scroll to Top")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.labels_list.update(cx, |list, cx| {
                                    list.scroll_to_item(0, window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-bottom")
                            .child("Scroll to Bottom")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.labels_list.update(cx, |list, cx| {
                                    list.scroll_to_item(
                                        list.delegate().items_count(cx) - 1,
                                        window,
                                        cx,
                                    );
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-to-selected")
                            .child("Scroll to Selected")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.labels_list.update(cx, |list, cx| {
                                    if let Some(selected) = list.selected_index() {
                                        list.scroll_to_item(selected, window, cx);
                                    }
                                })
                            })),
                    )
                    .child(
                        Checkbox::new("loading")
                            .label("Loading")
                            .checked(self.labels_list.read(cx).delegate().loading)
                            .on_click(cx.listener(|this, check: &bool, _, cx| {
                                this.labels_list.update(cx, |this, cx| {
                                    this.delegate_mut().loading = *check;
                                    cx.notify();
                                })
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(self.labels_list.clone()),
            )
    }
}
