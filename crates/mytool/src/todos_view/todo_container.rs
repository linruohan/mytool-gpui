use crate::{Board, ContainerEvent, Mytool, ShowPanelInfo, ToggleSearch};
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyView, App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
};
use gpui_component::dock::{PanelControl, PanelEvent};
use gpui_component::notification::Notification;
use gpui_component::{ContextModal, IconName, v_flex};
const PANEL_NAME: &str = "TodoContainer";
pub struct TodoContainer {
    focus_handle: gpui::FocusHandle,
    pub name: SharedString,
    pub title_bg: Option<Hsla>,
    pub description: SharedString,
    width: Option<gpui::Pixels>,
    height: Option<gpui::Pixels>,
    mytool: Option<AnyView>,
    story_klass: Option<SharedString>,
    closable: bool,
    zoomable: Option<PanelControl>,
    on_active: Option<fn(AnyView, bool, &mut Window, &mut App)>,
    pub board_color: Hsla,
    pub board_count: usize,
    pub board_icon: IconName,
}

impl EventEmitter<ContainerEvent> for TodoContainer {}

impl TodoContainer {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            focus_handle,
            name: "".into(),
            title_bg: None,
            description: "".into(),
            width: None,
            height: None,
            mytool: None,
            story_klass: None,
            closable: true,
            zoomable: Some(PanelControl::default()),
            on_active: None,
            board_color: Hsla::default(),
            board_count: 0,
            board_icon: IconName::ALargeSmall,
        }
    }

    pub fn panel<S: Board + Mytool>(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let name = S::title();
        let color = S::color();
        let count = S::count();
        let icon = S::icon();

        let description = S::description();
        let mytool = S::new_view(window, cx);
        let story_klass = S::klass();
        let focus_handle = mytool.focus_handle(cx);

        let view = cx.new(|cx| {
            let mut mytool = Self::new(window, cx)
                .mytool(mytool.into(), story_klass)
                .on_active(S::on_active_any);
            mytool.focus_handle = focus_handle;
            mytool.closable = S::closable();
            mytool.zoomable = S::zoomable();
            mytool.name = name.into();
            mytool.board_color = color;
            mytool.board_count = count;
            mytool.board_icon = icon;
            mytool.description = description.into();
            mytool.title_bg = S::title_bg();
            mytool
        });

        view
    }

    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: gpui::Pixels) -> Self {
        self.height = Some(height);
        self
    }

    pub fn mytool(mut self, mytool: AnyView, story_klass: impl Into<SharedString>) -> Self {
        self.mytool = Some(mytool);
        self.story_klass = Some(story_klass.into());
        self
    }

    pub fn on_active(mut self, on_active: fn(AnyView, bool, &mut Window, &mut App)) -> Self {
        self.on_active = Some(on_active);
        self
    }

    fn on_action_panel_info(
        &mut self,
        _: &ShowPanelInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        struct Info;
        let note = Notification::new()
            .message(format!("You have clicked panel info on: {}", self.name))
            .id::<Info>();
        window.push_notification(note, cx);
    }

    fn on_action_toggle_search(
        &mut self,
        _: &ToggleSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.propagate();
        if window.has_focused_input(cx) {
            return;
        }

        struct Search;
        let note = Notification::new()
            .message(format!("You have toggled search on: {}", self.name))
            .id::<Search>();
        window.push_notification(note, cx);
    }
}
impl EventEmitter<PanelEvent> for TodoContainer {}
impl Focusable for TodoContainer {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for TodoContainer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("todo-container")
            .size_full()
            .overflow_y_scroll()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_panel_info))
            .on_action(cx.listener(Self::on_action_toggle_search))
            .when_some(self.mytool.clone(), |this, mytool| {
                this.child(
                    v_flex()
                        .id("todo-children")
                        .w_full()
                        .flex_1()
                        .p_4()
                        .child(mytool),
                )
            })
    }
}
