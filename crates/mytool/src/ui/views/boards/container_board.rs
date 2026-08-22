use gpui::{
    AnyView, App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, SharedString, StatefulInteractiveElement, Styled,
    Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IconName,
    button::Button,
    dock::{BasePanel, Panel, PanelControl, PanelEvent, PanelInfo, PanelState, TitleStyle},
    menu::PopupMenu,
    v_flex,
};

use crate::{ShowPanelInfo, VisualHierarchy};

pub struct BoardContainer {
    focus_handle: gpui::FocusHandle,
    pub name: SharedString,
    pub title_bg: Option<Hsla>,
    pub description: SharedString,
    board: Option<AnyView>,
    pub(crate) board_klass: Option<SharedString>,
    closable: bool,
    zoomable: Option<PanelControl>,
    on_active: Option<fn(AnyView, bool, &mut Window, &mut App)>,
    pub colors: Vec<Hsla>,
    pub count: usize,
    pub icon: IconName,
}

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

    pub fn board(mut self, board: AnyView, board_klass: impl Into<SharedString>) -> Self {
        self.board = Some(board);
        self.board_klass = Some(board_klass.into());
        self
    }

    pub fn on_active(mut self, on_active: fn(AnyView, bool, &mut Window, &mut App)) -> Self {
        self.on_active = Some(on_active);
        self
    }
}
impl EventEmitter<PanelEvent> for BoardContainer {}
impl Focusable for BoardContainer {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for BoardContainer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("board-container")
            .size_full()
            .overflow_y_scroll()
            .track_focus(&self.focus_handle)
            .when_some(self.board.clone(), |this, board| {
                this.child(
                    v_flex()
                        .id("board-children")
                        .w_full()
                        .flex_1()
                        .p(VisualHierarchy::spacing(4.0))
                        .child(board),
                )
            })
    }
}

/// BasePanel 实现：BoardContainer 的行为层（名称、可见性、关闭、缩放、回调等）
impl BasePanel for BoardContainer {
    /// 面板的唯一标识名称
    fn panel_name(&self) -> &'static str {
        "BoardContainer"
    }

    /// 面板是否可关闭
    fn closable(&self, _cx: &App) -> bool {
        self.closable
    }

    /// 面板是否支持缩放（返回 bool）
    fn zoomable(&self, _cx: &App) -> bool {
        self.zoomable.is_some()
    }

    /// 面板是否可见
    fn visible(&self, cx: &App) -> bool {
        // Mirror StoryContainer: visible when not listed in AppState::invisible_panels
        !crate::AppState::global(cx).invisible_panels.read(cx).contains(&self.name)
    }

    /// 面板缩放状态变更回调
    fn set_zoomed(&mut self, zoomed: bool, _window: &mut Window, _cx: &mut Context<Self>) {
        tracing::debug!("panel: {} zoomed: {}", self.name, zoomed);
    }

    /// 面板激活状态变更回调
    fn set_active(&mut self, active: bool, _window: &mut Window, cx: &mut Context<Self>) {
        tracing::debug!("panel: {} active: {}", self.name, active);
        if let Some(on_active) = self.on_active
            && let Some(board) = self.board.clone()
        {
            on_active(board, active, _window, cx);
        }
    }

    /// 序列化面板状态
    fn dump(&self, _cx: &App) -> PanelState {
        let mut state = PanelState::new(self.panel_name());
        // Avoid panic: if board_klass is missing, fall back to ListStory
        let klass = self.board_klass.clone().unwrap_or_else(|| SharedString::from("ListStory"));
        let story_state = crate::ui::layout::story_state::StoryState { story_klass: klass };
        state.info = PanelInfo::panel(story_state.to_value());
        state
    }
}

/// Panel 实现：BoardContainer 的展示层（标题、样式、工具栏、菜单等）
impl Panel for BoardContainer {
    /// 面板标题元素
    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.name.clone().into_any_element()
    }

    /// 面板标题样式
    fn title_style(&self, cx: &App) -> Option<TitleStyle> {
        self.title_bg.map(|bg| TitleStyle { background: bg, foreground: cx.theme().foreground })
    }

    /// 缩放控件的显示位置（返回 PanelControl）
    fn zoom_control(&self, _cx: &App) -> Option<PanelControl> {
        self.zoomable
    }

    /// 下拉菜单项
    fn dropdown_menu(
        &mut self,
        menu: PopupMenu,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> PopupMenu {
        menu.menu("Info", Box::new(ShowPanelInfo))
    }

    /// 工具栏按钮
    fn toolbar_buttons(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Vec<Button>> {
        Some(vec![
            Button::new("info").icon(IconName::Info).on_click(|_, _window, cx| {
                // Dispatch a global ShowPanelInfo action; StoryRoot listens and handles
                // notification
                cx.dispatch_action(&crate::ShowPanelInfo);
            }),
            Button::new("search").icon(IconName::Search).on_click(|_, _window, cx| {
                // Dispatch ToggleSearch action; StoryRoot handles it globally
                cx.dispatch_action(&crate::ToggleSearch);
            }),
        ])
    }
}
