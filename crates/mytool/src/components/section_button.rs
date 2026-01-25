use std::rc::Rc;

use gpui::{
    Action, App, Context, Corner, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, px,
};
use gpui_component::{
    Sizable, Size, StyleSized, StyledExt as _, button::Button, menu::DropdownMenu, v_flex,
};
use serde::Deserialize;
use todos::entity::SectionModel;

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = section, no_json)]
struct SectionInfo(String);

pub enum SectionEvent {
    Selected(String),
}

pub struct SectionState {
    focus_handle: FocusHandle,
    pub selected_section: Option<Rc<SectionModel>>,
    pub sections: Option<Vec<Rc<SectionModel>>>,
}

impl EventEmitter<SectionEvent> for SectionState {}

impl Focusable for SectionState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl SectionState {
    pub(crate) fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle(), selected_section: None, sections: None }
    }

    pub fn section(&self) -> Option<Rc<SectionModel>> {
        self.selected_section.clone()
    }

    pub fn set_section(
        &mut self,
        section: Option<Rc<SectionModel>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_section = section;
        cx.notify()
    }

    pub fn set_sections(
        &mut self,
        sections: Option<Vec<Rc<SectionModel>>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sections = sections;
        // 清空已选择的section，因为project改变了
        self.selected_section = None;
        cx.notify()
    }

    fn on_action_info(&mut self, info: &SectionInfo, _window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(SectionEvent::Selected(info.0.clone()));
        cx.notify();
    }
}

/// A SectionButton element.
#[derive(IntoElement)]
pub struct SectionButton {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<SectionState>,
    #[allow(dead_code)]
    sections: Vec<Rc<SectionModel>>,
}

impl Sizable for SectionButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for SectionButton {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for SectionButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for SectionState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        // 优先使用传入的sections，否则从全局状态获取
        let sections = self
            .sections
            .clone()
            .unwrap_or_else(|| cx.global::<crate::todo_state::SectionState>().sections.clone());

        v_flex().on_action(cx.listener(Self::on_action_info)).child(
            Button::new("section")
                .outline()
                .tooltip("select section")
                .label(
                    self.selected_section
                        .as_ref()
                        .map(|s| SharedString::from(s.name.clone()))
                        .unwrap_or_else(|| SharedString::from("No Section")),
                )
                .dropdown_menu_with_anchor(Corner::TopLeft, move |this, _, _| {
                    let mut this = this.scrollable(true).max_h(px(400.));

                    // Add "No Section" option
                    this = this.menu(
                        SharedString::from("No Section"),
                        Box::new(SectionInfo(String::new())),
                    );

                    // Add all sections
                    for section in sections.iter() {
                        let section_id = section.id.clone();
                        this = this.menu(
                            SharedString::from(section.name.clone()),
                            Box::new(SectionInfo(section_id)),
                        )
                    }
                    this.min_w(px(150.))
                }),
        )
    }
}

impl SectionButton {
    /// Create a new SectionButton with the given [`SectionState`].
    pub fn new(state: &Entity<SectionState>, sections: Vec<Rc<SectionModel>>) -> Self {
        Self {
            id: ("item-section", state.entity_id()).into(),
            state: state.clone(),
            sections,
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for SectionButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .flex_none()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
