//! Manage Sections 组件 - Section 管理面板，支持增删改查、归档、复制等操作

use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath,
    button::Button,
    h_flex,
    list::{List, ListDelegate, ListEvent, ListItem, ListState},
    v_flex,
};
use todos::entity::SectionModel;

use crate::{
    core::actions::*,
    todo_state::TodoStore,
    ui::components::{SectionDialogConfig, show_section_delete_dialog, show_section_dialog},
};

/// Section 列表委托
pub struct ManageSectionListDelegate {
    sections: Vec<Arc<SectionModel>>,
    matched_sections: Vec<Vec<Arc<SectionModel>>>,
    selected_index: Option<IndexPath>,
}

impl Default for ManageSectionListDelegate {
    fn default() -> Self {
        Self::new()
    }
}

impl ManageSectionListDelegate {
    pub fn new() -> Self {
        Self { sections: vec![], matched_sections: vec![], selected_index: None }
    }

    pub fn update_sections(&mut self, sections: Vec<Arc<SectionModel>>) {
        self.sections = sections;
        self.matched_sections = vec![self.sections.clone()];
        if !self.matched_sections.is_empty() && self.selected_index.is_none() {
            self.selected_index = Some(IndexPath::default());
        }
    }

    pub fn selected_section(&self) -> Option<Arc<SectionModel>> {
        let ix = self.selected_index?;
        self.matched_sections.get(ix.section).and_then(|c| c.get(ix.row)).cloned()
    }

    pub fn section_at(&self, row: usize) -> Option<Arc<SectionModel>> {
        self.matched_sections.first().and_then(|sections| sections.get(row).cloned())
    }
}

impl ListDelegate for ManageSectionListDelegate {
    type Item = ListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> gpui::Task<()> {
        self.matched_sections = vec![
            self.sections
                .iter()
                .filter(|section| section.name.to_lowercase().contains(&query.to_lowercase()))
                .cloned()
                .collect(),
        ];
        gpui::Task::ready(())
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        self.matched_sections.get(section).map(|s| s.len()).unwrap_or(0)
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        if let Some(section) = self.matched_sections.get(ix.section).and_then(|c| c.get(ix.row)) {
            let is_archived = section.is_archived;

            let section_color = section
                .color
                .as_ref()
                .and_then(|c| u32::from_str_radix(&c[1..], 16).ok().map(gpui::rgb))
                .unwrap_or(gpui::rgb(0x3b82f6));

            let item = div()
                .id(("section-item", ix.row))
                .p(px(8.0))
                .border_1()
                .border_color(cx.theme().border)
                .rounded(px(4.0))
                .when(is_archived, |this| this.opacity(0.5))
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .child(
                            h_flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    Icon::build(IconName::TagOutlineSymbolic)
                                        .text_color(section_color),
                                )
                                .child(div().child(section.name.clone()))
                                .when(is_archived, |this| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("(已隐藏)"),
                                    )
                                }),
                        )
                        .child(
                            h_flex()
                                .gap(px(4.0))
                                .child(
                                    Button::new(format!("edit-{}", ix.row))
                                        .icon(IconName::Pencil)
                                        .size(px(14.0))
                                        .on_click(cx.listener(move |_this, _, _, cx| {
                                            cx.emit(ListEvent::Confirm(ix));
                                        })),
                                )
                                .child(
                                    Button::new(format!("delete-{}", ix.row))
                                        .icon(IconName::Trash)
                                        .size(px(14.0))
                                        .on_click(cx.listener(move |_this, _, _, cx| {
                                            cx.emit(ListEvent::Select(ix));
                                        })),
                                ),
                        ),
                );

            return Some(ListItem::new(ix.row).child(item));
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
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        cx.emit(ListEvent::Confirm(self.selected_index.unwrap_or_default()));
    }
}

/// 管理 Sections 的面板
pub struct ManageSectionsPanel {
    focus_handle: FocusHandle,
    section_list: Entity<ListState<ManageSectionListDelegate>>,
    pending_edit: Option<Arc<SectionModel>>,
    pending_delete: Option<Arc<SectionModel>>,
}

impl ManageSectionsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let section_list = cx.new(|cx| {
            ListState::new(ManageSectionListDelegate::new(), window, cx).selectable(false)
        });

        // 订阅 List 事件
        let section_list_for_subscribe = section_list.clone();
        let _subscriptions = [
            cx.subscribe(&section_list, move |this, list, event: &ListEvent, cx| {
                match event {
                    ListEvent::Confirm(ix) => {
                        // 编辑 Section
                        if let Some(section) = list.read(cx).delegate().section_at(ix.row) {
                            this.pending_edit = Some(section);
                            cx.notify();
                        }
                    },
                    ListEvent::Select(ix) => {
                        // 删除 Section (通过删除按钮触发)
                        if let Some(section) = list.read(cx).delegate().section_at(ix.row) {
                            this.pending_delete = Some(section);
                            cx.notify();
                        }
                    },
                    _ => {},
                }
            }),
            // 订阅 TodoStore 变化
            cx.observe_global::<TodoStore>(move |_this, cx| {
                let sections = cx.global::<TodoStore>().sections.clone();
                cx.update_entity(&section_list_for_subscribe, |list, cx| {
                    list.delegate_mut().update_sections(sections);
                    cx.notify();
                });
            }),
        ];

        // 初始化时主动加载一次 Sections 数据
        let sections = cx.global::<TodoStore>().sections.clone();
        cx.update_entity(&section_list, |list, cx| {
            list.delegate_mut().update_sections(sections);
            cx.notify();
        });

        Self {
            focus_handle: cx.focus_handle(),
            section_list,
            pending_edit: None,
            pending_delete: None,
        }
    }

    /// 显示编辑 Section 对话框
    fn show_edit_section_dialog(
        &self,
        section: Arc<SectionModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let _section_id = section.id.clone();
        let section_name = section.name.clone();
        let section_clone = section.clone();

        let name_input = cx.new(|cx| {
            let mut input =
                gpui_component::input::InputState::new(window, cx).placeholder("Section Name");
            input.set_value(&section_name, window, cx);
            input
        });

        let config = SectionDialogConfig::new("Edit Section", "Save", true);

        show_section_dialog(window, cx, name_input, config, move |new_name, cx| {
            // 更新 section
            let updated_section =
                Arc::new(SectionModel { name: new_name, ..(*section_clone).clone() });
            update_section(updated_section, cx);
        });
    }

    /// 显示删除确认对话框
    fn show_delete_confirmation(
        &self,
        section: Arc<SectionModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let _section_id = section.id.clone();
        let section_name = section.name.clone();
        let section_clone = section.clone();

        show_section_delete_dialog(
            window,
            cx,
            &format!("确定要删除分区 \"{}\" 吗？", section_name),
            move |cx| {
                delete_section(section_clone.clone(), cx);
            },
        );
    }

    /// 显示新建 Section 对话框
    pub fn show_new_section_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name_input = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx).placeholder("Section Name")
        });

        let config = SectionDialogConfig::new("New Section", "Add", false);

        show_section_dialog(window, cx, name_input, config, move |name, cx| {
            let new_section = Arc::new(SectionModel {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                added_at: chrono::Utc::now().naive_utc(),
                ..Default::default()
            });
            add_section(new_section, cx);
        });
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn section_list(&self) -> &Entity<ListState<ManageSectionListDelegate>> {
        &self.section_list
    }
}

impl Focusable for ManageSectionsPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ManageSectionsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 处理 pending 的编辑和删除
        if let Some(section) = self.pending_edit.take() {
            self.show_edit_section_dialog(section, window, cx);
        }
        if let Some(section) = self.pending_delete.take() {
            self.show_delete_confirmation(section, window, cx);
        }

        v_flex()
            .size_full()
            .gap_2()
            .child(h_flex().justify_between().child(div().child("Sections")).child(
                Button::new("new-section").label("New Section").icon(IconName::Plus).on_click(
                    cx.listener(|this, _, window, cx| {
                        this.show_new_section_dialog(window, cx);
                    }),
                ),
            ))
            .child(
                List::new(&self.section_list)
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(4.0)),
            )
    }
}
