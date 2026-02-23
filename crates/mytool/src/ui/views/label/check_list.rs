use std::sync::Arc;

use gpui::{
    App, Context, ElementId, Entity, EventEmitter, Hsla, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Task, Window, actions, div, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, Selectable,
    checkbox::Checkbox,
    h_flex,
    list::{ListDelegate, ListItem, ListState},
};
use todos::entity::LabelModel;
use tracing::info;

use crate::LabelsPopoverList;

actions!(label, [SelectedCheckLabel, UnSelectedCheckLabel]);
pub enum LabelCheckEvent {
    Checked(Arc<LabelModel>),
}

impl EventEmitter<LabelCheckEvent> for LabelCheckListItem {}
#[derive(IntoElement)]
pub struct LabelCheckListItem {
    base: ListItem,
    label: Arc<LabelModel>,
    selected: bool,
    checked: bool,
}

impl LabelCheckListItem {
    pub fn new(
        id: impl Into<ElementId>,
        label: Arc<LabelModel>,
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
                h_flex()
                    .items_center()
                    .justify_start()
                    .gap_3()
                    .text_color(text_color)
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
                    .child(div().flex_1().child(self.label.name.clone())),
            )
    }
}

pub struct LabelCheckListDelegate {
    #[allow(dead_code)]
    parent: Entity<LabelsPopoverList>,
    pub _labels: Vec<Arc<LabelModel>>,
    pub checked_list: Vec<Arc<LabelModel>>,
    pub matched_labels: Vec<Vec<Arc<LabelModel>>>,
    #[allow(dead_code)]
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
        let labels: Vec<Arc<LabelModel>> = self
            ._labels
            .iter()
            .filter(|label| label.name.to_lowercase().contains(&self.query.to_lowercase()))
            .cloned()
            .collect();

        // 清空之前的匹配结果
        self.matched_labels.clear();

        // 添加新的匹配结果
        for label in labels.into_iter() {
            self.matched_labels.push(vec![label]);
        }

        // 如果没有匹配结果，创建一个空的 section
        if self.matched_labels.is_empty() {
            self.matched_labels.push(vec![]);
            self.selected_index = None;
        }
    }

    pub fn update_labels(&mut self, labels: Vec<Arc<LabelModel>>) {
        info!("LabelCheckListDelegate::update_labels called: {} labels", labels.len());
        self._labels = labels;
        // 如果没有标签，创建一个空的 section
        if self._labels.is_empty() {
            self.matched_labels = vec![vec![]];
            self.selected_index = None;
        } else {
            self.matched_labels = vec![self._labels.clone()];
            if self.selected_index.is_none() {
                self.selected_index = Some(IndexPath::default());
            }
        }
        info!(
            "LabelCheckListDelegate::update_labels: matched_labels sections: {}, first section \
             len: {}",
            self.matched_labels.len(),
            self.matched_labels.first().map(|s| s.len()).unwrap_or(0)
        );
        // 保持 checked_list 不变，确保选中状态在标签更新后仍然保留
    }

    // set_checked_labels:设置checked标签
    pub fn set_item_checked_labels(
        &mut self,
        labels: Vec<Arc<LabelModel>>,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.checked_list = labels.clone();
        cx.notify();
    }

    pub fn selected_label(&self) -> Option<Arc<LabelModel>> {
        let ix = self.selected_index?;
        self.matched_labels.get(ix.section).and_then(|c| c.get(ix.row)).cloned()
    }

    #[allow(dead_code)]
    fn confirm(&mut self, _select: bool, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        if let Some(label) = self.selected_label() {
            self.checked_list.push(label.clone());
        }
        self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;
            cx.notify();
        })
    }

    #[allow(dead_code)]
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
        // 检查 section 是否在范围内
        if section < self.matched_labels.len() { self.matched_labels[section].len() } else { 0 }
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        // 检查 matched_labels 是否为空或索引是否越界
        if ix.section >= self.matched_labels.len() {
            return None;
        }

        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(label) =
            self.matched_labels.get(ix.section).and_then(|section| section.get(ix.row))
        {
            let checked = self.checked_list.iter().any(|l| l.id == label.id);
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
        info!("LabelCheckListDelegate::set_selected_index called: {:?}", ix);
        self.selected_index = ix;
        cx.notify();
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        info!("LabelCheckListDelegate::confirm called: secondary={}", secondary);
        if let Some(label) = self.selected_label() {
            info!("LabelCheckListDelegate::confirm: selected label={}", label.name);
            let is_checked = self.checked_list.iter().any(|l| l.id == label.id);
            info!("LabelCheckListDelegate::confirm: is_checked={}", is_checked);

            if secondary {
                // Shift+Enter: 取消选中
                if is_checked {
                    self.checked_list.retain(|l| l.id != label.id);
                    window.dispatch_action(Box::new(UnSelectedCheckLabel), cx);
                }
            } else {
                // Enter: 切换选中状态
                if is_checked {
                    self.checked_list.retain(|l| l.id != label.id);
                    window.dispatch_action(Box::new(UnSelectedCheckLabel), cx);
                } else {
                    self.checked_list.push(label.clone());
                    window.dispatch_action(Box::new(SelectedCheckLabel), cx);
                }
            }
        } else {
            info!("LabelCheckListDelegate::confirm: no selected label found");
        }
    }
}
