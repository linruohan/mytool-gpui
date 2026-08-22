use gpui::{
    AnyView, App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, SharedString, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IconName, WindowExt,
    button::Button,
    dock::{BasePanel, Panel, PanelControl, PanelEvent, PanelInfo, PanelState, TitleStyle},
    menu::PopupMenu,
    scroll::ScrollableElement,
};

use crate::{AppState, Mytool, ShowPanelInfo, StoryState};

pub struct StoryContainer {
    pub(crate) focus_handle: gpui::FocusHandle,
    pub name: SharedString,
    pub title_bg: Option<Hsla>,
    pub description: SharedString,
    story: Option<AnyView>,
    story_klass: Option<SharedString>,
    pub(crate) closable: bool,
    pub(crate) zoomable: Option<PanelControl>,
    paddings: Pixels,
    on_active: Option<fn(AnyView, bool, &mut Window, &mut App)>,
}

impl StoryContainer {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            focus_handle,
            name: "".into(),
            title_bg: None,
            description: "".into(),
            story: None,
            story_klass: None,
            closable: true,
            zoomable: Some(PanelControl::default()),
            paddings: px(16.),
            on_active: None,
        }
    }

    pub fn panel<S: Mytool>(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let name = S::title();
        let description = S::description();
        let story = S::new_view(window, cx);
        let story_klass = S::klass();

        let view = cx.new(|cx| {
            let mut story =
                Self::new(window, cx).story(story.into(), story_klass).on_active(S::on_active_any);
            story.focus_handle = cx.focus_handle();
            story.closable = S::closable();
            story.zoomable = S::zoomable();
            story.name = name.into();
            story.description = description.into();
            story.title_bg = S::title_bg();
            story.paddings = S::paddings();
            story
        });

        view
    }

    pub fn story(mut self, story: AnyView, story_klass: impl Into<SharedString>) -> Self {
        self.story = Some(story);
        self.story_klass = Some(story_klass.into());
        self
    }

    pub fn on_active(mut self, on_active: fn(AnyView, bool, &mut Window, &mut App)) -> Self {
        self.on_active = Some(on_active);
        self
    }
}

/// BasePanel 实现：面板的行为层（名称、可见性、关闭、缩放、回调等）
impl BasePanel for StoryContainer {
    /// 面板的唯一标识名称，用于持久化布局
    fn panel_name(&self) -> &'static str {
        "StoryContainer"
    }

    /// 面板是否可关闭
    fn closable(&self, _cx: &App) -> bool {
        self.closable
    }

    /// 面板是否支持缩放（返回 bool，控制缩放功能是否启用）
    fn zoomable(&self, _cx: &App) -> bool {
        self.zoomable.is_some()
    }

    /// 面板是否可见
    fn visible(&self, cx: &App) -> bool {
        !AppState::global(cx).invisible_panels.read(cx).contains(&self.name)
    }

    /// 面板缩放状态变更回调
    fn set_zoomed(&mut self, zoomed: bool, _window: &mut Window, _cx: &mut Context<Self>) {
        println!("panel: {} zoomed: {}", self.name, zoomed);
    }

    /// 面板激活状态变更回调
    fn set_active(&mut self, active: bool, _window: &mut Window, cx: &mut Context<Self>) {
        println!("panel: {} active: {}", self.name, active);
        if let Some(on_active) = self.on_active {
            if let Some(story) = self.story.clone() {
                on_active(story, active, _window, cx);
            }
        }
    }

    /// 序列化面板状态，用于持久化布局
    fn dump(&self, _cx: &App) -> PanelState {
        let mut state = PanelState::new(self.panel_name());
        let story_state = StoryState { story_klass: self.story_klass.clone().unwrap() };
        state.info = PanelInfo::panel(story_state.to_value());
        state
    }
}

/// Panel 实现：面板的展示层（标题、样式、工具栏、菜单等）
impl Panel for StoryContainer {
    /// 面板标题元素
    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.name.clone().into_any_element()
    }

    /// 面板标题样式（背景色、前景色）
    fn title_style(&self, cx: &App) -> Option<TitleStyle> {
        if let Some(bg) = self.title_bg {
            Some(TitleStyle { background: bg, foreground: cx.theme().foreground })
        } else {
            None
        }
    }

    /// 缩放控件的显示位置（返回 PanelControl，控制缩放按钮显示在哪里）
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
            Button::new("info").icon(IconName::Info).on_click(|_, window, cx| {
                window.push_notification("You have clicked info button", cx);
            }),
            Button::new("search").icon(IconName::Search).on_click(|_, window, cx| {
                window.push_notification("You have clicked search button", cx);
            }),
        ])
    }
}

impl EventEmitter<PanelEvent> for StoryContainer {}
impl Focusable for StoryContainer {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for StoryContainer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("story-container")
            .size_full()
            .overflow_y_scrollbar()
            .track_focus(&self.focus_handle)
            .when_some(self.story.clone(), |this, story| {
                this.child(div().size_full().p(self.paddings).child(story))
            })
    }
}
