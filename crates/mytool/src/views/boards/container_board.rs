use gpui::{
    AnyView, App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, SharedString, StatefulInteractiveElement, Styled,
    Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    IconName, WindowExt,
    dock::{PanelControl, PanelEvent},
    notification::Notification,
    v_flex,
};

use crate::{ShowPanelInfo, ToggleSearch};
#[derive(Debug)]
pub enum BoardContainerEvent {
    Close,
}

pub struct BoardContainer {
    focus_handle: gpui::FocusHandle,
    pub name: SharedString,
    pub title_bg: Option<Hsla>,
    pub description: SharedString,
    width: Option<gpui::Pixels>,
    height: Option<gpui::Pixels>,
    board: Option<AnyView>,
    pub(crate) board_klass: Option<SharedString>,
    closable: bool,
    zoomable: Option<PanelControl>,
    on_active: Option<fn(AnyView, bool, &mut Window, &mut App)>,
    pub colors: Vec<Hsla>,
    pub count: usize,
    pub icon: IconName,
}

impl EventEmitter<BoardContainerEvent> for BoardContainer {}

pub trait Board: Render + Sized {
    fn icon() -> IconName;
    fn colors() -> Vec<Hsla>;
    fn count(cx: &mut App) -> usize;
    fn klass() -> &'static str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }
    fn title() -> &'static str;

    fn description() -> &'static str {
        ""
    }
    fn closable() -> bool {
        true
    }

    fn zoomable() -> Option<PanelControl> {
        Some(PanelControl::default())
    }

    fn title_bg() -> Option<Hsla> {
        None
    }

    fn paddings() -> Pixels {
        px(16.)
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render>;

    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut App) {
        let _ = active;
        let _ = window;
        let _ = cx;
    }

    fn on_active_any(view: AnyView, active: bool, window: &mut Window, cx: &mut App)
    where
        Self: 'static,
    {
        if let Ok(board) = view.downcast::<Self>() {
            cx.update_entity(&board, |board, cx| {
                board.on_active(active, window, cx);
            });
        }
    }
}

impl BoardContainer {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            focus_handle,
            name: "".into(),
            title_bg: None,
            description: "".into(),
            width: None,
            height: None,
            board: None,
            board_klass: None,
            closable: true,
            zoomable: Some(PanelControl::default()),
            on_active: None,
            colors: Vec::new(),
            count: 0,
            icon: IconName::ALargeSmall,
        }
    }

    pub fn panel<S: Board>(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let name = S::title();
        let colors = S::colors();
        let count = S::count(cx);
        let icon = S::icon();

        let description = S::description();
        let mytool = S::new_view(window, cx);
        let story_klass = S::klass();
        let focus_handle = cx.focus_handle();

        cx.new(|cx| {
            let mut mytool =
                Self::new(window, cx).board(mytool.into(), story_klass).on_active(S::on_active_any);
            mytool.focus_handle = focus_handle;
            mytool.closable = S::closable();
            mytool.zoomable = S::zoomable();
            mytool.name = name.into();
            mytool.colors = colors;
            mytool.count = count;
            mytool.icon = icon;
            mytool.description = description.into();
            mytool.title_bg = S::title_bg();
            mytool
        })
    }

    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: gpui::Pixels) -> Self {
        self.height = Some(height);
        self
    }

    pub fn board(mut self, board: AnyView, board_klass: impl Into<SharedString>) -> Self {
        self.board = Some(board);
        self.board_klass = Some(board_klass.into());
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
impl EventEmitter<PanelEvent> for BoardContainer {}
impl Focusable for BoardContainer {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for BoardContainer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("board-container")
            .size_full()
            .overflow_y_scroll()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_panel_info))
            .on_action(cx.listener(Self::on_action_toggle_search))
            .when_some(self.board.clone(), |this, board| {
                this.child(v_flex().id("board-children").w_full().flex_1().p_4().child(board))
            })
    }
}
