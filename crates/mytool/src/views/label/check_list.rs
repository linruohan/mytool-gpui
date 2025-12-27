use std::rc::Rc;

use gpui::{
    App, Context, ElementId, Entity, EventEmitter, Hsla, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Task, Window, actions, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, Selectable,
    checkbox::Checkbox,
    h_flex,
    list::{ListDelegate, ListItem, ListState},
};
use todos::entity::LabelModel;

use crate::LabelsPopoverList;

actions!(label, [SelectedCheckLabel, UnSelectedCheckLabel]);
pub enum LabelCheckEvent {
    Checked(Rc<LabelModel>),
}

impl EventEmitter<LabelCheckEvent> for LabelCheckListItem {}
#[derive(IntoElement)]
pub struct LabelCheckListItem {
    base: ListItem,
    label: Rc<LabelModel>,
    selected: bool,
    checked: bool,
}

impl LabelCheckListItem {
    pub fn new(
        id: impl Into<ElementId>,
        label: Rc<LabelModel>,
        selected: bool,
        checked: bool,
    ) -> Self {
        LabelCheckListItem { label, base: ListItem::new(id), selected, checked }
    }
}

impl Selectable for LabelCheckListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }

    fn secondary_selected(mut self, secondary: bool) -> Self {
        self.checked = secondary;
        self
    }
}

impl RenderOnce for LabelCheckListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_color =
            if self.selected { cx.theme().accent_foreground } else { cx.theme().foreground };

        self.base
            .px_2()
            .py_1()
            .overflow_x_hidden()
            .border_1()
            .rounded(cx.theme().radius)
            .when(self.selected, |this| this.border_color(cx.theme().list_active_border))
            .rounded(cx.theme().radius)
            .child(
                h_flex().items_center().justify_between().gap_2().text_color(text_color).child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .justify_end()
                        .child(Checkbox::new("label-checked").checked(self.checked))
                        .child(
                            Icon::build(IconName::TagOutlineSymbolic).text_color(Hsla::from(
                                gpui::rgb(
                                    u32::from_str_radix(&self.label.color[1..], 16)
                                        .ok()
                                        .unwrap_or_default(),
                                ),
                            )),
                        )
                        .child(div().w(px(120.)).child(self.label.name.clone())),
                ),
            )
    }
}

pub struct LabelCheckListDelegate {
    parent: Entity<LabelsPopoverList>,
    pub _labels: Vec<Rc<LabelModel>>,
    pub checked_list: Vec<Rc<LabelModel>>,
    pub matched_labels: Vec<Vec<Rc<LabelModel>>>,
    checked: bool,
    selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    query: SharedString,
}

impl LabelCheckListDelegate {
    pub fn new(parent: Entity<LabelsPopoverList>) -> Self {
        Self {
            parent,
            _labels: vec![],
            checked_list: vec![],
            matched_labels: vec![],
            selected_index: None,
            confirmed_index: None,
            checked: false,
            query: "".into(),
        }
    }

    fn prepare(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        let labels: Vec<Rc<LabelModel>> = self
            ._labels
            .iter()
            .filter(|label| label.name.to_lowercase().contains(&self.query.to_lowercase()))
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

    // set_checked_labels:设置checked标签
    pub fn set_item_checked_labels(
        &mut self,
        labels: Vec<Rc<LabelModel>>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.checked_list = labels.clone();
    }

    pub fn selected_label(&self) -> Option<Rc<LabelModel>> {
        let Some(ix) = self.selected_index else {
            return None;
        };
        self.matched_labels.get(ix.section).and_then(|c| c.get(ix.row)).cloned()
    }

    fn confirm(&mut self, _select: bool, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        if let Some(label) = self.selected_label() {
            self.checked_list.push(label.clone());
        }
        self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;
            cx.notify();
        })
    }

    fn cancel(&mut self, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;

            cx.notify();
        })
    }
}
impl ListDelegate for LabelCheckListDelegate {
    type Item = LabelCheckListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.prepare(query.to_owned());
        Task::ready(())
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        self.matched_labels[section].len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        let checked =
            self.selected_label().map(|label| self.checked_list.contains(&label)).unwrap_or(false);
        if let Some(label) = self.matched_labels[ix.section].get(ix.row) {
            return Some(LabelCheckListItem::new(ix, label.clone(), selected, checked));
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

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        println!("Confirmed with secondary: {}", secondary);
        if secondary {
            window.dispatch_action(Box::new(UnSelectedCheckLabel), cx);
        }
        window.dispatch_action(Box::new(SelectedCheckLabel), cx);
    }
}
