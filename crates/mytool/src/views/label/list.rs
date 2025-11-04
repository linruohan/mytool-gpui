use gpui::prelude::FluentBuilder;
use gpui::{
    actions, div, px, App, Context, ElementId, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Task, Window,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::label::Label;
use gpui_component::{
    h_flex, list::{ListDelegate, ListItem, ListState}, ActiveTheme, ContextModal, IndexPath, Placement,
    Selectable,
};
use std::rc::Rc;
use todos::entity::LabelModel;

actions!(label, [SelectedLabel]);
pub enum LabelEvent {
    Loaded,
    Added(Rc<LabelModel>),
    Modified(Rc<LabelModel>),
    Deleted(Rc<LabelModel>),
}
#[derive(IntoElement)]
pub struct LabelListItem {
    base: ListItem,
    ix: IndexPath,
    label: Rc<LabelModel>,
    selected: bool,
}

impl LabelListItem {
    pub fn new(
        id: impl Into<ElementId>,
        label: Rc<LabelModel>,
        ix: IndexPath,
        selected: bool,
    ) -> Self {
        LabelListItem {
            label,
            ix,
            base: ListItem::new(id),
            selected,
        }
    }
}

impl Selectable for LabelListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for LabelListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.base
            .px_2()
            .py_1()
            .overflow_x_hidden()
            .border_1()
            .when(self.selected, |this| {
                this.border_color(cx.theme().list_active_border)
            })
            .rounded(cx.theme().radius)
            .child(
                h_flex().items_center().justify_between().gap_2().child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .justify_end()
                        .child(div().w(px(65.)).child(self.label.id.clone()))
                        .child(div().w(px(65.)).child(self.label.name.clone()))
                        .child(div().w(px(65.)).child(self.label.color.clone())),
                ),
            )
    }
}

pub struct LabelListDelegate {
    pub(crate) _labels: Vec<Rc<LabelModel>>,
    pub(crate) matched_labels: Vec<Vec<Rc<LabelModel>>>,
    selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    query: SharedString,
}

impl LabelListDelegate {
    pub fn new() -> Self {
        Self {
            _labels: vec![],
            matched_labels: vec![],
            selected_index: None,
            confirmed_index: None,
            query: "".into(),
        }
    }
    fn prepare(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        let labels: Vec<Rc<LabelModel>> = self
            ._labels
            .iter()
            .filter(|label| {
                label
                    .name
                    .to_lowercase()
                    .contains(&self.query.to_lowercase())
            })
            .cloned()
            .collect();
        for label in labels.into_iter() {
            self.matched_labels.push(vec![label]);
        }
    }

    pub fn update_labels(&mut self, labels: Vec<Rc<LabelModel>>) {
        self._labels = labels;
        self.matched_labels = vec![self._labels.clone()];
        if !self.matched_labels.is_empty() && self.selected_index.is_none() {
            self.selected_index = Some(IndexPath::default());
        }
    }
    pub fn add(&mut self, label: Rc<LabelModel>) {
        let mut labels = self._labels.clone();
        labels.push(label.clone());
        self.update_labels(labels);
    }
    #[allow(unused)]
    fn selected_label(&self) -> Option<Rc<LabelModel>> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.matched_labels
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }
    fn open_drawer_at_label(
        &mut self,
        label: Rc<LabelModel>,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        window.open_drawer_at(Placement::Right, cx, move |this, _, _cx| {
            this.overlay(true)
                .overlay_closable(false)
                .size(px(400.))
                .title(label.name.clone())
                .gap_4()
                .child(
                    Button::new("send-notification")
                        .child("Test Notification")
                        .on_click(|_, window, cx| {
                            window.push_notification("Hello this is message from Drawer.", cx)
                        }),
                )
                .child(
                    Label::new(label.name.clone()), // List::new(&label)
                    //     .border_1()
                    //     .border_color(cx.theme().border)
                    //     .rounded(cx.theme().radius)
                    //     .size_full()
                    //     .flex_1()
                    //     .h(px(400.)),
                )
                .footer(
                    h_flex()
                        .gap_6()
                        .items_center()
                        .child(Button::new("confirm").primary().label("确认").on_click(
                            |_, window, cx| {
                                window.close_drawer(cx);
                            },
                        ))
                        .child(
                            Button::new("cancel")
                                .label("取消")
                                .on_click(|_, window, cx| {
                                    window.close_drawer(cx);
                                }),
                        ),
                )
        });
    }
}
impl ListDelegate for LabelListDelegate {
    type Item = LabelListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.prepare(query.to_owned());
        Task::ready(())
    }
    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.matched_labels.len()
    }

    fn render_item(&self, ix: IndexPath, _: &mut Window, _: &mut App) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(company) = self.matched_labels[ix.section].get(ix.row) {
            return Some(LabelListItem::new(ix, company.clone(), ix, selected));
        }

        None
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        window.dispatch_action(Box::new(SelectedLabel), cx);
        let label_some = self.selected_label();
        if let Some(label) = label_some {
            self.open_drawer_at_label(label, window, cx)
        }
    }
}
