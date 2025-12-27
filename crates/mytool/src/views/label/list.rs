use std::rc::Rc;

use gpui::{
    App, Context, ElementId, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Task, Window, actions, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, Selectable, h_flex,
    list::{ListDelegate, ListItem, ListState},
};
use todos::entity::LabelModel;

use crate::UnSelectedCheckLabel;

actions!(label, [SelectedLabel, UnSelectedLabel]);
pub enum LabelEvent {
    Checked(Rc<LabelModel>),
    UnChecked(Rc<LabelModel>),
    Loaded,
    Added(Rc<LabelModel>),
    Modified(Rc<LabelModel>),
    Deleted(Rc<LabelModel>),
}

#[derive(IntoElement)]
pub struct LabelListItem {
    base: ListItem,
    label: Rc<LabelModel>,
    selected: bool,
    checked: bool,
}

impl LabelListItem {
    pub fn new(
        id: impl Into<ElementId>,
        label: Rc<LabelModel>,
        selected: bool,
        checked: bool,
    ) -> Self {
        LabelListItem { label, base: ListItem::new(id), selected, checked }
    }
}

impl Selectable for LabelListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self.checked = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }

    fn secondary_selected(mut self, secondary: bool) -> Self {
        if secondary {
            self.checked = !self.checked;
        }
        self
    }
}

impl RenderOnce for LabelListItem {
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

pub struct LabelListDelegate {
    pub _labels: Vec<Rc<LabelModel>>,
    pub checked_labels: Vec<Rc<LabelModel>>,
    pub matched_labels: Vec<Vec<Rc<LabelModel>>>,
    pub(crate) selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    query: SharedString,
}

impl LabelListDelegate {
    pub fn new() -> Self {
        Self {
            _labels: vec![],
            checked_labels: vec![],
            matched_labels: vec![],
            selected_index: None,
            confirmed_index: None,
            query: "".into(),
        }
    }

    // 获取已选中的标签
    pub fn checked_labels(&mut self) -> Vec<Rc<LabelModel>> {
        self.checked_labels.clone()
    }

    // 获取已选中的标签
    pub fn set_checked_labels(mut self, labels: Vec<Rc<LabelModel>>) -> Self {
        self.checked_labels = labels;
        self
    }

    // 添加已选中的标签
    pub fn add_selected_label(&mut self, label: Rc<LabelModel>) {
        // 避免重复添加
        if !self.checked_labels.contains(&label) {
            self.checked_labels.push(label.clone());
        }
    }

    // 删除已选中的标签
    pub fn del_selected_label(&mut self, label: Rc<LabelModel>) {
        self.checked_labels.retain(|l| l != &label);
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

    pub fn selected_label(&self) -> Option<Rc<LabelModel>> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.matched_labels.get(ix.section).and_then(|c| c.get(ix.row)).cloned()
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
        if let Some(label) = self.matched_labels[ix.section].get(ix.row) {
            let checked = self.checked_labels.contains(&label);
            return Some(LabelListItem::new(ix, label.clone(), selected, checked));
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
        if let Some(label) = self.selected_label() {
            self.checked_labels.push(label.clone());
        }
        window.dispatch_action(
            if secondary { Box::new(UnSelectedCheckLabel) } else { Box::new(SelectedLabel) },
            cx,
        );
    }
}
