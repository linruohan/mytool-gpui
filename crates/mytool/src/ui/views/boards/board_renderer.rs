//! 通用 Board 渲染组件
//!
//! 这个模块提供了可复用的 Board 渲染逻辑，减少各 Board 组件的重复代码。
//! 由于 GPUI 的生命周期限制，这些函数只在 Board 内部使用。

use std::sync::Arc;

use gpui::{
    App, AppContext, BorrowAppContext, ClickEvent, Context, Entity, Hsla, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, StatefulInteractiveElement, Styled, Window,
    div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    h_flex,
    menu::{DropdownMenu, PopupMenu, PopupMenuItem},
    v_flex,
};
use gpui_kit::assets::IconName;
use rust_i18n::t;
use todos::entity::ItemModel;

use super::{
    board_base::{BoardView, drop_reorder_items},
    board_common::BoardSectionActions,
};
use crate::{
    ItemRow, ItemRowState, ScheduleButtonState, board_section,
    core::state::{ItemSelection, TodoStore},
};

/// 任务行拖拽载荷，同时作为拖影预览。
#[derive(Clone)]
pub struct ItemDragPayload {
    pub item_id: String,
    pub content: String,
}

impl Render for ItemDragPayload {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let label =
            if self.content.is_empty() { self.item_id.clone() } else { self.content.clone() };
        div()
            .px_3()
            .py_1()
            .rounded_md()
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_md()
            .text_sm()
            .max_w(px(280.))
            .child(label)
    }
}

fn click_is_multi(event: &ClickEvent) -> bool {
    event.modifiers().secondary()
}

/// Board 空状态：居中大图标 + 标题 + 提示，无虚线框。
pub fn render_empty_placeholder(
    cx: &App,
    icon: IconName,
    title: impl Into<gpui::SharedString>,
    description: impl Into<gpui::SharedString>,
) -> impl IntoElement {
    v_flex()
        .w_full()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_2()
        .py_8()
        .child(
            Icon::new(icon)
                .with_size(px(48.))
                .text_color(cx.theme().muted_foreground.opacity(0.45)),
        )
        .child(div().text_lg().font_semibold().child(title.into()))
        .child(div().text_sm().text_color(cx.theme().muted_foreground).child(description.into()))
}

// ==================== 通用渲染辅助 ====================
/// 渲染单行任务项（可点击选中、高亮、展示 ItemRow）
pub fn render_item_row<V>(
    i: usize,
    item_row: Option<Entity<ItemRowState>>,
    is_active: bool,
    is_selected: bool,
    item_id: String,
    active_border: gpui::Hsla,
    view: Entity<V>,
) -> impl IntoElement
where
    V: BoardView + Render,
{
    let drag_id = item_id.clone();
    let drop_id = item_id.clone();
    let can_drop_id = item_id.clone();
    div()
        .id(("item", i))
        .rounded_md()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .when(!item_id.is_empty(), |this| {
            this.on_drag(
                ItemDragPayload { item_id: drag_id, content: String::new() },
                |drag, _, _, cx| {
                    let mut preview = drag.clone();
                    if preview.content.is_empty() {
                        preview.content = cx
                            .global::<TodoStore>()
                            .get_item(&preview.item_id)
                            .map(|item| item.content.clone())
                            .unwrap_or_default();
                    }
                    cx.new(|_| preview)
                },
            )
            .drag_over::<ItemDragPayload>(|style, _, _, cx| {
                style.bg(cx.theme().accent.opacity(0.18))
            })
            .can_drop(move |drag, _, _| {
                drag.downcast_ref::<ItemDragPayload>()
                    .is_some_and(|payload| payload.item_id != can_drop_id)
            })
            .on_drop(move |drag: &ItemDragPayload, _, cx| {
                drop_reorder_items(&drag.item_id, &drop_id, cx);
            })
        })
        .on_click(move |event, _, cx| {
            cx.stop_propagation();
            let multi = click_is_multi(event);
            let id = item_id.clone();
            if !id.is_empty() {
                cx.update_global::<ItemSelection, _>(|sel, _| sel.apply_click(id, multi));
            }
            view.update(cx, |this, cx| {
                this.set_active_index(Some(i));
                cx.notify();
            });
        })
        .when(is_selected, |this| this.bg(active_border.opacity(0.22)).rounded_md())
        .when(is_active && !is_selected, |this| this.bg(active_border.opacity(0.12)).rounded_md())
        .children(item_row.map(|row| ItemRow::new(&row)))
}

/// 仅渲染任务列表（v_flex 行），不包 section；用于已有 section 标题的区块（如 No
/// Section、动态分区）
pub fn render_item_list<V>(
    items: &[(usize, Arc<ItemModel>)],
    item_rows: &[Entity<ItemRowState>],
    active_index: Option<usize>,
    active_border: gpui::Hsla,
    view: Entity<V>,
    cx: &App,
) -> impl IntoElement
where
    V: BoardView + Render,
{
    let selected = cx.global::<ItemSelection>().ids().clone();
    v_flex().gap(px(2.)).w_full().children(flatten_with_indent(items).into_iter().map(
        |(i, indent)| {
            let item_row = item_rows.get(i).cloned();
            let item_id =
                item_row.as_ref().map(|row| row.read(cx).item.id.clone()).unwrap_or_default();
            let is_active = active_index == Some(i);
            let is_selected = selected.contains(&item_id);
            let row = render_item_row(
                i,
                item_row,
                is_active,
                is_selected,
                item_id,
                active_border,
                view.clone(),
            );
            div().when(indent, |this| this.pl(px(22.))).child(row)
        },
    ))
}

fn flatten_with_indent(items: &[(usize, Arc<ItemModel>)]) -> Vec<(usize, bool)> {
    let ids: std::collections::HashSet<&str> =
        items.iter().map(|(_, item)| item.id.as_str()).collect();
    let mut nested_pos = std::collections::HashSet::new();
    let mut by_parent: std::collections::HashMap<&str, Vec<usize>> =
        std::collections::HashMap::new();
    for (pos, (_, item)) in items.iter().enumerate() {
        if let Some(pid) = item.parent_id.as_deref().filter(|id| !id.is_empty())
            && ids.contains(pid)
        {
            by_parent.entry(pid).or_default().push(pos);
            nested_pos.insert(pos);
        }
    }
    let mut out = Vec::with_capacity(items.len());
    for (pos, (_, item)) in items.iter().enumerate() {
        if nested_pos.contains(&pos) {
            continue;
        }
        out.push((items[pos].0, false));
        if let Some(child_pos) = by_parent.get(item.id.as_str()) {
            for &cpos in child_pos {
                out.push((items[cpos].0, true));
            }
        }
    }
    out
}

/// 构建 Section 更多操作下拉菜单（编辑 / 复制 / 归档 / 删除）
pub fn build_section_more_menu<V: BoardSectionActions>(
    view: Entity<V>,
    section_id: String,
) -> impl Fn(PopupMenu, &mut Window, &mut gpui::Context<PopupMenu>) -> PopupMenu + 'static {
    move |this, window, _cx| {
        let view = view.clone();
        let section_id2 = section_id.clone();
        let section_id3 = section_id.clone();
        let section_id4 = section_id.clone();
        let section_id5 = section_id.clone();
        this.item(PopupMenuItem::new(t!("todo.section.edit").to_string()).on_click(
            window.listener_for(&view, move |this, _, window, cx| {
                this.show_section_dialog(window, cx, Some(section_id2.clone()), true);
                cx.notify();
            }),
        ))
        .separator()
        .item(PopupMenuItem::new(t!("todo.section.copy").to_string()).on_click(
            window.listener_for(&view, move |this, _, window, cx| {
                this.duplicate_section(window, cx, section_id3.clone());
                cx.notify();
            }),
        ))
        .separator()
        .item(PopupMenuItem::new(t!("todo.section.archive").to_string()).on_click(
            window.listener_for(&view, move |this, _, window, cx| {
                this.archive_section(window, cx, section_id4.clone());
                cx.notify();
            }),
        ))
        .separator()
        .item(PopupMenuItem::new(t!("todo.section.delete").to_string()).on_click(
            window.listener_for(&view, move |this, _, window, cx| {
                this.show_section_delete_dialog(window, cx, section_id5.clone());
                cx.notify();
            }),
        ))
    }
}

/// 渲染带分区的 Section 区块（标题 + 工具栏 + 任务列表）
///
/// 参数较多是 GPUI 渲染函数的固有特点：需同时聚合视图实体、任务数据、
/// 高亮样式与交互回调等不同关注点，强行合并为参数对象反而降低可读性。
#[allow(clippy::too_many_arguments, reason = "渲染函数需聚合视图/数据/样式/交互等多类上下文")]
pub fn render_section_block<V: BoardSectionActions>(
    section_name: String,
    section_id: String,
    items: &[(usize, Arc<ItemModel>)],
    item_rows: &[Entity<ItemRowState>],
    active_index: Option<usize>,
    active_border: Hsla,
    view: Entity<V>,
    cx: &App,
) -> impl IntoElement {
    let view_clone = view.clone();
    let add_button = Button::new(format!("add-item-to-section-{}", section_id))
        .small()
        .ghost()
        .compact()
        .icon(IconName::PlusLargeSymbolic)
        .tooltip(t!("todo.item.add").to_string())
        .on_click({
            let view = view_clone.clone();
            let section_id = section_id.clone();
            move |_, window, cx| {
                view.update(cx, |this, cx| {
                    this.show_item_dialog(window, cx, false, Some(section_id.clone()));
                    cx.notify();
                })
            }
        });

    let more_button = Button::new(format!("more-section-{}", section_id))
        .small()
        .ghost()
        .compact()
        .icon(IconName::EllipsisVertical)
        .tooltip(t!("todo.more").to_string())
        .dropdown_menu(build_section_more_menu(view_clone.clone(), section_id.clone()));

    let mut block = board_section(section_name);
    block = block.sub_title(h_flex().gap_1().child(add_button).child(more_button));

    block.child(render_item_list(items, item_rows, active_index, active_border, view_clone, cx))
}

/// 渲染「No Section」区块
#[allow(clippy::too_many_arguments, reason = "渲染函数需聚合视图/数据/样式/交互等多类上下文")]
pub fn render_no_section_block<V: BoardSectionActions>(
    items: &[(usize, Arc<ItemModel>)],
    item_rows: &[Entity<ItemRowState>],
    active_index: Option<usize>,
    active_border: Hsla,
    view: Entity<V>,
    _compact_toolbar: bool,
    cx: &App,
) -> impl IntoElement {
    let view_clone = view.clone();

    let add_button = Button::new("add-item-to-no-section")
        .small()
        .ghost()
        .compact()
        .icon(IconName::PlusLargeSymbolic)
        .tooltip(t!("todo.item.add").to_string())
        .on_click({
            let view = view_clone.clone();
            move |_, window, cx| {
                view.update(cx, |this, cx| {
                    this.show_item_dialog(window, cx, false, None);
                    cx.notify();
                })
            }
        });

    board_section(t!("todo.section.ungrouped").to_string())
        .sub_title(h_flex().gap_1().child(add_button))
        .child(render_item_list(items, item_rows, active_index, active_border, view_clone, cx))
}

/// 渲染简单分组（标题 + 可选更多菜单 + 任务列表），用于 Pinned / Today 等虚拟分组
#[allow(clippy::too_many_arguments, reason = "渲染函数需聚合视图/数据/样式/交互等多类上下文")]
pub fn render_simple_group_block<V: BoardView + Render>(
    title: impl AsRef<str>,
    items: &[(usize, Arc<ItemModel>)],
    item_rows: &[Entity<ItemRowState>],
    active_index: Option<usize>,
    active_border: Hsla,
    view: Entity<V>,
    _show_more_menu: bool,
    cx: &App,
) -> impl IntoElement {
    board_section(title.as_ref()).child(render_item_list(
        items,
        item_rows,
        active_index,
        active_border,
        view,
        cx,
    ))
}

/// 渲染带 Schedule 按钮的简单分组，用于 Past Due 等
#[allow(clippy::too_many_arguments, reason = "渲染函数需聚合视图/数据/样式/交互等多类上下文")]
pub fn render_group_with_schedule_button<V: BoardView + Render>(
    title: impl AsRef<str>,
    items: &[(usize, Arc<ItemModel>)],
    item_rows: &[Entity<ItemRowState>],
    active_index: Option<usize>,
    active_border: Hsla,
    view: Entity<V>,
    schedule_button: &Entity<ScheduleButtonState>,
    cx: &App,
) -> impl IntoElement {
    let view_clone = view.clone();

    board_section(title.as_ref())
        .sub_title(crate::ui::components::ScheduleButton::new(schedule_button))
        .child(render_item_list(items, item_rows, active_index, active_border, view_clone, cx))
}

/// 渲染带前置工具栏元素的 Section 区块（如 Calendar Schedule 按钮 + Add + More）
#[allow(clippy::too_many_arguments, reason = "渲染函数需聚合视图/数据/样式/交互等多类上下文")]
pub fn render_section_block_with_leading<V: BoardSectionActions>(
    section_name: String,
    section_id: String,
    items: &[(usize, Arc<ItemModel>)],
    item_rows: &[Entity<ItemRowState>],
    active_index: Option<usize>,
    active_border: Hsla,
    view: Entity<V>,
    leading: impl IntoElement,
    cx: &App,
) -> impl IntoElement {
    let view_clone = view.clone();

    let add_button = Button::new(format!("add-item-to-section-{}", section_id))
        .small()
        .ghost()
        .compact()
        .icon(IconName::PlusLargeSymbolic)
        .tooltip(t!("todo.item.add").to_string())
        .on_click({
            let view = view_clone.clone();
            let section_id = section_id.clone();
            move |_, window, cx| {
                view.update(cx, |this, cx| {
                    this.show_item_dialog(window, cx, false, Some(section_id.clone()));
                    cx.notify();
                })
            }
        });

    let more_button = Button::new(format!("more-section-{}", section_id))
        .small()
        .ghost()
        .compact()
        .icon(IconName::EllipsisVertical)
        .tooltip(t!("todo.more").to_string())
        .dropdown_menu(build_section_more_menu(view_clone.clone(), section_id.clone()));

    board_section(section_name)
        .sub_title(h_flex().gap_1().child(leading).child(add_button).child(more_button))
        .child(render_item_list(items, item_rows, active_index, active_border, view_clone, cx))
}
