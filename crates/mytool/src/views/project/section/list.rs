use std::sync::Arc;

use gpui::{
    App, Context, ElementId, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Task, Window, actions, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, Selectable, h_flex,
    list::{ListDelegate, ListItem, ListState},
};
use todos::entity::SectionModel;

use crate::VisualHierarchy;

actions!(section, [SelectedSection, UnSelectedSection]);
#[allow(unused)]
pub enum SectionEvent {
    Checked(Arc<SectionModel>),
    UnChecked(Arc<SectionModel>),
    Loaded,
    Added(Arc<SectionModel>),
    Modified(Arc<SectionModel>),
    Deleted(Arc<SectionModel>),
}

#[derive(IntoElement)]
#[allow(dead_code)]
pub struct SectionListItem {
    base: ListItem,
    section: Arc<SectionModel>,
    selected: bool,
    checked: bool,
}

#[allow(dead_code)]
impl SectionListItem {
    pub fn new(
        id: impl Into<ElementId>,
        section: Arc<SectionModel>,
        selected: bool,
        checked: bool,
    ) -> Self {
        SectionListItem { section, base: ListItem::new(id), selected, checked }
    }
}

impl Selectable for SectionListItem {
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

impl RenderOnce for SectionListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_color =
            if self.selected { cx.theme().accent_foreground } else { cx.theme().foreground };

        self.base
            .p(VisualHierarchy::spacing(2.0))
            .overflow_x_hidden()
            .border_1()
            .rounded(cx.theme().radius)
            .when(self.selected, |this| this.border_color(cx.theme().list_active_border))
            .rounded(cx.theme().radius)
            .child(
                h_flex().items_center().justify_between().gap(VisualHierarchy::spacing(2.0)).text_color(text_color).child(
                    h_flex()
                        .gap(VisualHierarchy::spacing(2.0))
                        .items_center()
                        .justify_end()
                        .child(
                            Icon::build(IconName::TagOutlineSymbolic).text_color(Hsla::from(
                                gpui::rgb(
                                    u32::from_str_radix(
                                        &self.section.color.clone().unwrap_or_default()[1..],
                                        16,
                                    )
                                    .ok()
                                    .unwrap_or_default(),
                                ),
                            )),
                        )
                        .child(div().w(px(120.)).child(self.section.name.clone())),
                ),
            )
    }
}

#[allow(dead_code)]
pub struct SectionListDelegate {
    pub _sections: Vec<Arc<SectionModel>>,
    pub checked_sections: Vec<Arc<SectionModel>>,
    pub matched_sections: Vec<Vec<Arc<SectionModel>>>,
    pub(crate) selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    query: SharedString,
}

#[allow(dead_code)]
impl SectionListDelegate {
    pub fn new() -> Self {
        Self {
            _sections: vec![],
            checked_sections: vec![],
            matched_sections: vec![],
            selected_index: None,
            confirmed_index: None,
            query: "".into(),
        }
    }

    // 获取已选中的标签
    pub fn checked_sections(&mut self) -> Vec<Arc<SectionModel>> {
        self.checked_sections.clone()
    }

    // 获取已选中的标签
    pub fn set_checked_sections(mut self, sections: Vec<Arc<SectionModel>>) -> Self {
        self.checked_sections = sections;
        self
    }

    // 添加已选中的标签
    pub fn add_selected_section(&mut self, section: Arc<SectionModel>) {
        // 避免重复添加
        if !self.checked_sections.contains(&section) {
            self.checked_sections.push(section.clone());
        }
    }

    // 删除已选中的标签
    pub fn del_selected_section(&mut self, section: Arc<SectionModel>) {
        self.checked_sections.retain(|l| l != &section);
    }

    fn prepare(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        let sections: Vec<Arc<SectionModel>> = self
            ._sections
            .iter()
            .filter(|section| section.name.to_lowercase().contains(&self.query.to_lowercase()))
            .cloned()
            .collect();
        for section in sections.into_iter() {
            self.matched_sections.push(vec![section]);
        }
    }

    pub fn update_sections(&mut self, sections: Vec<Arc<SectionModel>>) {
        self._sections = sections;
        self.matched_sections = vec![self._sections.clone()];
        if !self.matched_sections.is_empty() && self.selected_index.is_none() {
            self.selected_index = Some(IndexPath::default());
        }
    }

    pub fn selected_section(&self) -> Option<Arc<SectionModel>> {
        let ix = self.selected_index?;
        self.matched_sections.get(ix.section).and_then(|c| c.get(ix.row)).cloned()
    }
}
impl ListDelegate for SectionListDelegate {
    type Item = SectionListItem;

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
        self.matched_sections[section].len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(sec) = self.matched_sections[ix.section].get(ix.row) {
            let checked = self.checked_sections.contains(sec);
            return Some(SectionListItem::new(ix, sec.clone(), selected, checked));
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
        if let Some(section) = self.selected_section() {
            self.checked_sections.push(section.clone());
        }
        window.dispatch_action(
            if secondary { Box::new(UnSelectedSection) } else { Box::new(SelectedSection) },
            cx,
        );
    }
}
