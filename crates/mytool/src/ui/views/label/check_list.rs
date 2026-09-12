use std::sync::Arc;

use gpui::{
    App, Context, ElementId, EventEmitter, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Task, Window, actions, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, IndexPath, Selectable, Sizable,
    h_flex,
    list::{ListDelegate, ListItem, ListState},
};
use gpui_kit::assets::IconName;
use todos::entity::LabelModel;
use tracing::info;

use crate::{label_color, label_color_dot};

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

    fn secondary_selected(self, _: bool) -> Self {
        // List 用 secondary_selected 表示右键高亮，不能覆盖标签勾选状态。
        self
    }
}

impl RenderOnce for LabelCheckListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let fill = if self.checked {
            cx.theme().list_active_border.opacity(0.18)
        } else if self.selected {
            cx.theme().list_hover
        } else {
            cx.theme().transparent
        };

        self.base.px_1().py_px().overflow_x_hidden().child(
            h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .px_2()
                .py_1()
                .rounded(cx.theme().radius)
                .bg(fill)
                .child(label_color_dot(label_color(&self.label.color)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .text_ellipsis()
                        .text_color(cx.theme().foreground)
                        .child(self.label.name.clone()),
                )
                .when(self.checked, |this| {
                    this.child(
                        Icon::new(IconName::CheckSquare)
                            .small()
                            .text_color(cx.theme().list_active_border),
                    )
                }),
        )
    }
}

pub struct LabelCheckListDelegate {
    pub _labels: Vec<Arc<LabelModel>>,
    pub checked_list: Vec<Arc<LabelModel>>,
    pub matched_labels: Vec<Vec<Arc<LabelModel>>>,
    selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    query: SharedString,
}

impl LabelCheckListDelegate {
    pub fn new() -> Self {
        Self {
            _labels: vec![],
            checked_list: vec![],
            matched_labels: vec![],
            selected_index: None,
            confirmed_index: None,
            query: "".into(),
        }
    }

    fn visible_labels(&self) -> Vec<Arc<LabelModel>> {
        let query = self.query.to_lowercase();
        let mut labels: Vec<Arc<LabelModel>> = self
            ._labels
            .iter()
            .filter(|label| !label.is_deleted)
            .filter(|label| query.is_empty() || label.name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        labels.sort_by(|a, b| {
            let a_checked = self.checked_list.iter().any(|l| l.id == a.id);
            let b_checked = self.checked_list.iter().any(|l| l.id == b.id);
            b_checked.cmp(&a_checked).then_with(|| a.name.cmp(&b.name))
        });
        labels
    }

    fn set_matched(&mut self, labels: Vec<Arc<LabelModel>>) {
        let len = labels.len();
        self.matched_labels = vec![labels];
        if len == 0 {
            self.selected_index = None;
            return;
        }
        if let Some(ix) = self.selected_index
            && (ix.section != 0 || ix.row >= len)
        {
            self.selected_index = None;
        }
    }

    fn prepare(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        self.set_matched(self.visible_labels());
    }

    pub fn update_labels(&mut self, labels: Vec<Arc<LabelModel>>) {
        if self._labels == labels {
            return;
        }
        self._labels = labels;
        self.set_matched(self.visible_labels());
    }

    // set_checked_labels:设置checked标签
    pub fn set_item_checked_labels(
        &mut self,
        labels: Vec<Arc<LabelModel>>,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.checked_list = labels.clone();
        self.set_matched(self.visible_labels());
        cx.notify();
    }

    pub fn selected_label(&self) -> Option<Arc<LabelModel>> {
        let ix = self.selected_index?;
        self.matched_labels.get(ix.section).and_then(|c| c.get(ix.row)).cloned()
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
        self.matched_labels.get(section).map(|s| s.len()).unwrap_or(0)
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        h_flex()
            .w_full()
            .h(px(64.))
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(if self.query.is_empty() { "还没有标签" } else { "没有匹配的标签" })
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(section) = self.matched_labels.get(ix.section)
            && let Some(label) = section.get(ix.row)
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
