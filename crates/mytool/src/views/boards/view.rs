use crate::{
    BoardContainer, CompletedBoard, InboxBoard, LabelsBoard, PinBoard, ScheduledBoard, TodayBoard,
};
use gpui::prelude::FluentBuilder;
use gpui::{
    App, AppContext, ClickEvent, Context, Entity, IntoElement, IsZero, ParentElement, Render,
    Styled, Subscription, Window, div, px,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::sidebar::{SidebarBoard, SidebarBoardItem};
use gpui_component::{ActiveTheme, h_flex, v_flex};

pub struct BoardPanel {
    search_input: Entity<InputState>,
    pub boards: Vec<Entity<BoardContainer>>,
    pub(crate) active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl BoardPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        let boards = vec![
            BoardContainer::panel::<InboxBoard>(window, cx),
            BoardContainer::panel::<TodayBoard>(window, cx),
            BoardContainer::panel::<ScheduledBoard>(window, cx),
            BoardContainer::panel::<PinBoard>(window, cx),
            BoardContainer::panel::<LabelsBoard>(window, cx),
            BoardContainer::panel::<CompletedBoard>(window, cx),
        ];
        let _subscriptions = vec![cx.subscribe(&search_input, |this, _, e, cx| match e {
            InputEvent::Change => {
                this.active_index = Some(0);
                cx.notify()
            }
            _ => {}
        })];
        Self {
            search_input,
            boards,
            active_index: None,
            _subscriptions,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
    pub fn update_active_index(&mut self, value: Option<usize>) {
        self.active_index = value;
    }
}

impl Render for BoardPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search_input.read(cx).value().trim().to_lowercase();
        let boards: Vec<_> = self
            .boards
            .iter()
            .filter(|story| story.read(cx).name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        //项目分类：
        v_flex()
            .w_full()
            .gap_4()
            .child(
                div()
                    .bg(cx.theme().sidebar_accent)
                    .rounded_full()
                    .when(cx.theme().radius.is_zero(), |this| this.rounded(px(0.)))
                    .flex_1()
                    .mx_1()
                    .child(Input::new(&self.search_input).appearance(false).cleanable()),
            )
            .child(
                SidebarBoard::new().children(
                    boards
                        .iter()
                        .enumerate()
                        .map(|(ix, item)| {
                            let board = item.read(cx);
                            SidebarBoardItem::new(
                                board.name.clone(),
                                board.colors.clone(),
                                board.count,
                                board.icon.clone(),
                            )
                            .size(gpui::Length::Definite(gpui::DefiniteLength::Fraction(0.5)))
                            .active(self.active_index == Some(ix))
                            .on_click(cx.listener(
                                move |this, _: &ClickEvent, _, cx| {
                                    this.active_index = Some(ix);
                                    println!("board:view {:?}", this.active_index);
                                    cx.notify();
                                },
                            ))
                        })
                        .collect::<Vec<_>>(),
                ),
            )
            .child(
                h_flex()
                    .bg(cx.theme().sidebar_border)
                    .px_1()
                    .flex_1()
                    .justify_between()
                    .mt(px(35.0)),
            )
    }
}
