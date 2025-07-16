use gpui::{
    actions, div, px, App, Context, ElementId, IntoElement, ParentElement, RenderOnce, Styled,
    Task, Timer, Window,
};
use gpui_component::{
    h_flex,
    list::{List, ListDelegate, ListItem},
    v_flex, ActiveTheme,
};
use std::time::Duration;
use todos::entity::LabelModel;
use todos::entity::labels::Model;
use todos::objects;

actions!(story, [SelectedLabel]);

#[derive(IntoElement)]
pub struct Label {
    base: ListItem,
    ix: usize,
    label: objects::label::Label,
    selected: bool,
}

impl Label {
    pub fn new(
        id: impl Into<ElementId>,
        label: objects::label::Label,
        ix: usize,
        selected: bool,
    ) -> Self {
        Label {
            label,
            ix,
            base: ListItem::new(id),
            selected,
        }
    }
}

impl RenderOnce for Label {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_color = if self.selected {
            cx.theme().accent_foreground
        } else {
            cx.theme().foreground
        };

        let bg_color = if self.selected {
            cx.theme().list_active
        } else if self.ix % 2 == 0 {
            cx.theme().list
        } else {
            cx.theme().list_even
        };

        self.base
            .px_3()
            .py_1()
            .overflow_x_hidden()
            .bg(bg_color)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .text_color(text_color)
                    .child(
                        v_flex()
                            .gap_1()
                            .max_w(px(500.))
                            .overflow_x_hidden()
                            .flex_nowrap()
                            .child(
                                gpui_component::label::Label::new(self.label.model.name.clone())
                                    .whitespace_nowrap(),
                            )
                            .child(
                                div().text_sm().overflow_x_hidden().child(
                                    gpui_component::label::Label::new(self.label.model.color.clone())
                                        .whitespace_nowrap()
                                        .text_color(text_color.opacity(0.5)),
                                ),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .justify_end()
                            .child(
                                div()
                                    .w(px(65.))
                                    .text_color(text_color)
                                    .child(self.label.model.color.clone()),
                            )
                            .child(
                                h_flex().w(px(65.)).justify_end().child(
                                    div()
                                        .rounded(cx.theme().radius)
                                        .whitespace_nowrap()
                                        .text_size(px(12.))
                                        .px_1(),
                                ),
                            ),
                    ),
            )
    }
}

pub struct LabelListDelegate {
    pub labels: Vec<LabelModel>,
    pub matched_labels: Vec<LabelModel>,
    pub selected_index: Option<usize>,
    pub confirmed_index: Option<usize>,
    pub query: String,
    pub loading: bool,
    pub eof: bool,
}

impl ListDelegate for LabelListDelegate {
    type Item = Label;

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        self.matched_labels = self
            .labels
            .iter()
            .filter(|company| company.name.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect();
        Task::ready(())
    }

    fn items_count(&self, _: &App) -> usize {
        self.matched_labels.len()
    }

    fn render_item(
        &self,
        ix: usize,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(company) = self.matched_labels.get(ix) {
            return Some(Label::new(company, /* todos::objects::Label */, /* usize */ /* bool */));
        }

        None
    }

    fn loading(&self, _: &App) -> bool {
        self.loading
    }

    fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
        println!("Confirmed with secondary: {}", secondary);
        window.dispatch_action(Box::new(SelectedLabel), cx);
    }

    fn can_load_more(&self, _: &App) -> bool {
        return !self.loading && !self.eof;
    }

    fn load_more_threshold(&self) -> usize {
        150
    }

    fn load_more(&mut self, window: &mut Window, cx: &mut Context<List<Self>>) {
        cx.spawn_in(window, async move |view, window| {
            // Simulate network request, delay 1s to load data.
            Timer::after(Duration::from_secs(1)).await;

            _ = view.update_in(window, move |view, window, cx| {
                let query = view.delegate().query.clone();
                view.delegate_mut()
                    .labels
                    .extend((0..200).map(|_| random_label()));
                _ = view.delegate_mut().perform_search(&query, window, cx);
                view.delegate_mut().eof = view.delegate().labels.len() >= 6000;
            });
        })
        .detach();
    }
}

impl LabelListDelegate {
    pub fn selected_label(&self) -> Option<Model> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.labels.get(ix).cloned()
    }
}
