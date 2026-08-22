//! ScheduledBoard - 计划任务视图
//!
//! 显示计划中任务，在其他时间去执行的任务。
//! 使用 TodoStore 作为数据源，通过内存过滤获取数据。

use std::{collections::HashMap, sync::Arc};

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, Hsla, InteractiveElement,
    ParentElement, Render, Styled, Window, div,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable,
    button::{Button, ButtonVariants},
    dock::PanelControl,
    h_flex,
    menu::{DropdownMenu, PopupMenuItem},
    scroll::ScrollableElement,
    v_flex,
};

use crate::{
    BoardBase, VisualHierarchy, section_with_title,
    todo_state::TodoStore,
    ui::views::boards::{
        BoardView,
        board_common::{
            BoardItemClickEvent, FinishItemDialogStyle, render_board_header,
            show_finish_item_dialog, show_item_delete_dialog, show_pin_item_dialog,
            with_selected_item,
        },
        board_renderer,
        container_board::Board,
    },
};

impl EventEmitter<BoardItemClickEvent> for ScheduledBoard {}

pub struct ScheduledBoard {
    base: BoardBase,
    /// 按日期分组的缓存（在 refresh 时构建，render 只读）
    grouped_by_date: Vec<(String, Vec<(usize, Arc<todos::entity::ItemModel>)>)>,
}

impl ScheduledBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { base: BoardBase::new(window, cx), grouped_by_date: Vec::new() }
    }

    fn apply_pending_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(state_items) = self.base.apply_store_refresh(
            window,
            cx,
            crate::core::state::ChangeMask::affects_scheduled,
            |this| &this.base.pending_refresh,
            |cx| {
                let cache = cx.global::<crate::core::state::QueryCache>();
                cx.global::<TodoStore>().scheduled_items_cached(cache)
            },
        ) {
            self.grouped_by_date = group_scheduled_by_date(state_items.as_slice());
        }
    }

    pub fn show_item_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        is_edit: bool,
        section_id: Option<String>,
    ) {
        self.base.show_item_dialog(window, cx, is_edit, section_id);
    }

    pub fn show_item_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_item_delete_dialog(window, cx, item);
        });
    }

    pub fn show_pin_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_pin_item_dialog(window, cx, item);
        });
    }

    pub fn show_finish_item_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        with_selected_item(self.base.active_index, &self.base, cx, |item, cx| {
            show_finish_item_dialog(window, cx, item, FinishItemDialogStyle::Standard);
        });
    }

    pub fn show_section_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: Option<String>,
        is_edit: bool,
    ) {
        self.base.show_section_dialog(window, cx, section_id, is_edit);
    }

    pub fn show_section_delete_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        section_id: String,
    ) {
        BoardBase::show_section_delete_dialog(window, cx, section_id);
    }
}

fn group_scheduled_by_date(
    items: &[Arc<todos::entity::ItemModel>],
) -> Vec<(String, Vec<(usize, Arc<todos::entity::ItemModel>)>)> {
    let mut items_by_date: HashMap<String, Vec<(usize, Arc<todos::entity::ItemModel>)>> =
        HashMap::new();
    for (i, item) in items.iter().enumerate() {
        let date_key = item.due_date_ymd().unwrap_or_else(|| "无日期".to_string());
        items_by_date.entry(date_key).or_default().push((i, item.clone()));
    }
    let mut grouped: Vec<_> = items_by_date.into_iter().collect();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));
    grouped
}

impl BoardView for ScheduledBoard {
    fn set_active_index(&mut self, index: Option<usize>) {
        self.base.set_active_index(index);
    }
}

impl Board for ScheduledBoard {
    fn icon() -> IconName {
        IconName::MonthSymbolic
    }

    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0xdc8add).into(), gpui::rgb(0x9141ac).into()]
    }

    fn count(cx: &mut App) -> usize {
        let store = cx.global::<TodoStore>();
        let cache = cx.global::<crate::core::state::QueryCache>();
        store.scheduled_items_cached(cache).len()
    }

    fn title() -> &'static str {
        "Scheduled"
    }

    fn description() -> &'static str {
        "计划中任务，在其他时间去执行的任务"
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ScheduledBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.base.focus_handle.clone()
    }
}

impl Render for ScheduledBoard {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        // 在 render 开头处理待执行的刷新操作
        self.apply_pending_refresh(window, cx);

        let view = cx.entity().clone();
        let active_border = cx.theme().list_active_border;
        let item_rows = &self.base.item_rows;
        let active_index = self.base.active_index;

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let grouped_by_date = &self.grouped_by_date;
        let orange_color = gpui::hsla(38.0, 1.0, 0.53, 1.0);

        v_flex()
            .track_focus(&self.base.focus_handle)
            .size_full()
            .gap(VisualHierarchy::spacing(4.0))
            .child(render_board_header(
                cx,
                <ScheduledBoard as Board>::icon(),
                <ScheduledBoard as Board>::title(),
                <ScheduledBoard as Board>::description(),
                Button::new("add-section")
                    .small()
                    .ghost()
                    .compact()
                    .icon(IconName::PlusLargeSymbolic)
                    .label("Add Section")
                    .on_click({
                        let view = view.clone();
                        move |_event, window, cx| {
                            view.update(cx, |this, cx| {
                                this.show_section_dialog(window, cx, None, false);
                                cx.notify();
                            })
                        }
                    }),
            ))
            .child(
                v_flex().flex_1().overflow_y_scrollbar().child(
                    v_flex()
                        .gap(VisualHierarchy::spacing(4.0))
                        .p(VisualHierarchy::spacing(3.0))
                        .children(grouped_by_date.iter().filter_map(|(date, items)| {
                            if items.is_empty() {
                                return None;
                            }

                            let view_clone = view.clone();
                            let is_today = date.as_str() == today;

                            let title_color =
                                if is_today { orange_color } else { cx.theme().foreground };

                            Some(
                                section_with_title(div().flex().items_center().gap_2().child(
                                    div().text_base().text_color(title_color).child(date.clone()),
                                ))
                                .sub_title(
                                    h_flex().gap_1().child(
                                        Button::new(format!("more-date-{}", date))
                                            .small()
                                            .ghost()
                                            .compact()
                                            .icon(IconName::EllipsisVertical)
                                            .dropdown_menu({
                                                let view = view_clone.clone();
                                                move |this, window, _cx| {
                                                    this.item(
                                                        PopupMenuItem::new("Show Completed Tasks")
                                                            .on_click(window.listener_for(
                                                                &view,
                                                                |_this, _, _window, cx| {
                                                                    cx.notify();
                                                                },
                                                            )),
                                                    )
                                                }
                                            }),
                                    ),
                                )
                                .child(
                                    board_renderer::render_item_list(
                                        items,
                                        item_rows,
                                        active_index,
                                        active_border,
                                        view_clone,
                                    ),
                                ),
                            )
                        })),
                ),
            )
    }
}
