use gpui::{
    App, AppContext, ClickEvent, Context, Entity, EventEmitter, Hsla, InteractiveElement,
    IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled, Subscription, Window,
    div, prelude::FluentBuilder, px,
};
use gpui_component::{
    Icon, h_flex,
    input::{InputEvent, InputState},
    v_flex,
};
use gpui_kit::assets::IconName;
use rust_i18n::t;

use crate::{
    Board, BoardContainer, CompletedBoard, InboxBoard, ItemEvent, LabelEvent, LabelsBoard,
    PinBoard, ScheduledBoard, TodayBoard, todo_state::TodoStore,
};

pub struct BoardPanel {
    search_input: Entity<InputState>,
    pub boards: Vec<Entity<BoardContainer>>,
    pub(crate) active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
    /// 🚀 6.9修复：缓存上次各 board 的 count 值<br/>用于避免不必要的 notify，打破观察者循环
    cached_counts: Vec<usize>,
}
impl EventEmitter<LabelEvent> for BoardPanel {}
impl EventEmitter<ItemEvent> for BoardPanel {}

impl BoardPanel {
    fn board_count_for_klass(klass: &str, cx: &mut App) -> Option<usize> {
        let map: [(&str, fn(&mut App) -> usize); 6] = [
            (InboxBoard::klass(), InboxBoard::count),
            (TodayBoard::klass(), TodayBoard::count),
            (ScheduledBoard::klass(), ScheduledBoard::count),
            (PinBoard::klass(), PinBoard::count),
            (LabelsBoard::klass(), LabelsBoard::count),
            (CompletedBoard::klass(), CompletedBoard::count),
        ];
        map.iter().find(|(k, _)| *k == klass).map(|(_, f)| f(cx))
    }

    /// 🚀 6.9修复：安全地刷新 count 并检查是否有实际变化
    ///
    /// 只有在至少一个 board 的 count 发生变化时才返回 true，
    /// 调用方据此决定是否需要 notify。
    fn refresh_counts_if_changed(&mut self, cx: &mut Context<Self>) -> bool {
        let mut new_counts = Vec::with_capacity(self.boards.len());
        let mut changed = false;

        for (ix, board) in self.boards.iter().enumerate() {
            // 先读取 board_klass，避免与后续 cx 使用冲突
            let board_klass = board.read(cx).board_klass.clone();
            let new_count = board_klass
                .as_deref()
                .and_then(|klass| Self::board_count_for_klass(klass, cx))
                .unwrap_or(0);

            if ix >= self.cached_counts.len() || self.cached_counts[ix] != new_count {
                changed = true;
            }
            new_counts.push(new_count);
        }

        if new_counts.len() != self.cached_counts.len() {
            changed = true;
        }

        if changed {
            for (ix, board) in self.boards.iter().enumerate() {
                let nc = new_counts[ix];
                board.update(cx, |b, _| {
                    b.count = nc;
                });
            }
            self.cached_counts = new_counts;
        }

        changed
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx
            .new(|cx| InputState::new(window, cx).placeholder(t!("todo.board.filter").to_string()));
        // 🚀 7.0修复后：恢复所有 6 个 Board（InboxBoard 已使用延迟注册）
        let boards = vec![
            BoardContainer::panel::<InboxBoard>(window, cx),
            BoardContainer::panel::<TodayBoard>(window, cx),
            BoardContainer::panel::<ScheduledBoard>(window, cx),
            BoardContainer::panel::<LabelsBoard>(window, cx),
            BoardContainer::panel::<PinBoard>(window, cx),
            BoardContainer::panel::<CompletedBoard>(window, cx),
        ];

        // 初始化缓存的 count 值（全为0，第一次回调时会更新）
        let cached_counts = vec![0; boards.len()];

        let _subscriptions = vec![
            cx.subscribe(&search_input, |_, _, e, cx| {
                if let InputEvent::Change = e {
                    cx.notify()
                }
            }),
            // 刷新各 Board 的 count 显示
            cx.observe_global::<TodoStore>(move |this, cx| {
                if this.refresh_counts_if_changed(cx) {
                    cx.notify();
                }
            }),
        ];
        Self { search_input, boards, active_index: Some(0), _subscriptions, cached_counts }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn update_active_index(&mut self, value: Option<usize>) {
        self.active_index = value;
    }

    pub fn labels_board(&self, cx: &App) -> Option<Entity<LabelsBoard>> {
        self.boards
            .iter()
            .find_map(|board| board.read(cx).inner_board()?.downcast::<LabelsBoard>().ok())
    }
}

fn board_tile_colors(colors: &[Hsla]) -> (Hsla, Hsla) {
    let bg = colors.first().copied().unwrap_or_else(|| gpui::rgb(0xe8e8e8).into());
    let accent = colors.get(1).copied().unwrap_or(bg);
    (bg, accent)
}

fn render_board_tile(
    ix: usize,
    name: impl Into<gpui::SharedString>,
    icon: IconName,
    colors: &[Hsla],
    count: usize,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let (bg, accent) = board_tile_colors(colors);
    v_flex()
        .id(("board-tile", ix))
        .flex_1()
        .h(px(54.))
        .px_3()
        .py_1p5()
        .gap_1()
        .justify_between()
        .rounded(px(14.))
        .bg(bg)
        .border_1()
        .border_color(if active { accent } else { bg })
        .hover(|this| this.opacity(0.9))
        .on_click(on_click)
        .child(
            h_flex()
                .w_full()
                .justify_between()
                .items_center()
                .child(Icon::new(icon).text_color(accent))
                .when(count > 0, |this| {
                    this.child(div().text_xs().text_color(accent).child(count.to_string()))
                }),
        )
        .child(div().text_sm().text_color(accent).child(name.into()))
}

impl Render for BoardPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search_input.read(cx).value().trim().to_lowercase();
        let boards: Vec<_> = self
            .boards
            .iter()
            .enumerate()
            .filter(|(_, story)| story.read(cx).name.to_lowercase().contains(&query))
            .map(|(ix, story)| (ix, story.clone()))
            .collect();

        v_flex().w_full().gap_1().children(boards.chunks(2).map(|row| {
            h_flex().w_full().gap_1().children(row.iter().map(|(ix, item)| {
                let board = item.read(cx);
                render_board_tile(
                    *ix,
                    board.name.clone(),
                    board.icon,
                    &board.colors,
                    board.count,
                    self.active_index == Some(*ix),
                    cx.listener({
                        let ix = *ix;
                        move |this, _: &ClickEvent, _, cx| {
                            this.active_index = Some(ix);
                            cx.notify();
                        }
                    }),
                )
            }))
        }))
    }
}
