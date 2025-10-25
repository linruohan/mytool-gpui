use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, ParentElement, Render, Styled,
    Window,
};

use super::Board;
use gpui_component::{IconName, dock::PanelControl, label::Label, v_flex};
use todos::entity::LabelModel;

pub struct LabelsBoard {
    focus_handle: FocusHandle,
    labels: Vec<LabelModel>,
}

impl LabelsBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            labels: Vec::new(),
        }
    }
    pub fn labels(&self) -> &[LabelModel] {
        &self.labels
    }

    pub fn add_label(&mut self, label: LabelModel) {
        self.labels.push(label);
    }
    pub fn clear_labels(&mut self) {
        self.labels.clear();
    }
}
impl Board for LabelsBoard {
    fn icon() -> IconName {
        IconName::TagOutlineSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xcdab8f).into(), gpui::rgb(0x986a44).into()]
    }

    fn count() -> usize {
        1
    }
    fn title() -> &'static str {
        "Labels"
    }

    fn description() -> &'static str {
        "UI components for building fantastic desktop application by using GPUI."
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for LabelsBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LabelsBoard {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .p_4()
            .gap_5()
            .child(Label::new("labels"))
            .child(Label::new("label内容"))
    }
}
