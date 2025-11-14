use gpui::{
    App, AppContext, ClickEvent, Context, Entity, EventEmitter, InteractiveElement, IntoElement,
    IsZero, MouseButton, ParentElement, Render, Styled, Subscription, Window, div,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    sidebar::{SidebarBoard, SidebarBoardItem},
    v_flex,
};

use crate::{
    BoardContainer, CompletedBoard, InboxBoard, ItemEvent, LabelEvent, LabelsBoard, PinBoard,
    ScheduledBoard, TodayBoard,
};

pub struct BoardPanel {
    search_input: Entity<InputState>,
    pub boards: Vec<Entity<BoardContainer>>,
    pub(crate) active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<LabelEvent> for BoardPanel {}
impl EventEmitter<ItemEvent> for BoardPanel {}

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
            },
            _ => {},
        })];
        Self { search_input, boards, active_index: None, _subscriptions }
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
        // 项目分类：
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
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(Input::new(&self.search_input).appearance(false).cleanable(true))
                            .child(
                                Button::new("add-label")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::EditFindSymbolic),
                            )
                            .child(
                                Button::new("edit-item")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::MenuLargeSymbolic),
                            ),
                    ),
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
