//! 通用 Board 渲染组件
//!
//! 这个模块提供了可复用的 Board 渲染逻辑，减少各 Board 组件的重复代码。
//! 由于 GPUI 的生命周期限制，这些函数只在 Board 内部使用。

use std::sync::Arc;

use gpui::{
    Entity, Hsla, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder,
};
use gpui_component::{
    IconName, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    menu::{DropdownMenu, PopupMenu, PopupMenuItem},
    v_flex,
};
use todos::entity::ItemModel;

use super::{board_base::BoardView, board_common::BoardSectionActions};
use crate::{ItemRow, ItemRowState, ScheduleButtonState, VisualHierarchy, section};

// ==================== 通用渲染辅助 ====================
/// 渲染单行任务项（可点击选中、高亮、展示 ItemRow）
pub fn render_item_row<V>(
    i: usize,
    item_row: Option<Entity<ItemRowState>>,
    is_active: bool,
    active_border: gpui::Hsla,
    view: Entity<V>,
) -> impl IntoElement
where
    V: BoardView + Render,
{
    div()
        .id(("item", i))
        .on_click(move |_, _, cx| {
            view.update(cx, |this, cx| {
                this.set_active_index(Some(i));
                cx.notify();
            });
        })
        .when(is_active, |this| this.border_color(active_border))
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
) -> impl IntoElement
where
    V: BoardView + Render,
{
    v_flex().gap(VisualHierarchy::spacing(2.0)).w_full().children(items.iter().map(|(i, _)| {
        let item_row = item_rows.get(*i).cloned();
        let is_active = active_index == Some(*i);
        render_item_row(*i, item_row, is_active, active_border, view.clone())
    }))
}

/// Section 区块渲染选项
pub struct SectionBlockOptions {
    /// 是否在工具栏显示独立的 Edit / Delete 按钮（Inbox 为 true，Pin 为 false）
    pub show_inline_edit_delete: bool,
}

/// 构建 Section 更多操作下拉菜单（Add / Edit / Duplicate / Archive / Delete）
pub fn build_section_more_menu<V: BoardSectionActions>(
    view: Entity<V>,
    section_id: String,
) -> impl Fn(PopupMenu, &mut Window, &mut gpui::Context<PopupMenu>) -> PopupMenu + 'static {
    move |this, window, _cx| {
        let view = view.clone();
        let section_id1 = section_id.clone();
        let section_id2 = section_id.clone();
        let section_id3 = section_id.clone();
        let section_id4 = section_id.clone();
        let section_id5 = section_id.clone();
        this.item(PopupMenuItem::new("+ Add Task").on_click(window.listener_for(
            &view,
            move |this, _, window, cx| {
                this.show_item_dialog(window, cx, false, Some(section_id1.clone()));
                cx.notify();
            },
        )))
        .separator()
        .item(PopupMenuItem::new("Edit Section").on_click(window.listener_for(
            &view,
            move |this, _, window, cx| {
                this.show_section_dialog(window, cx, Some(section_id2.clone()), true);
                cx.notify();
            },
        )))
        .separator()
        .item(PopupMenuItem::new("Duplicate").on_click(window.listener_for(
            &view,
            move |this, _, window, cx| {
                this.duplicate_section(window, cx, section_id3.clone());
                cx.notify();
            },
        )))
        .separator()
        .item(PopupMenuItem::new("Archive").on_click(window.listener_for(
            &view,
            move |this, _, window, cx| {
                this.archive_section(window, cx, section_id4.clone());
                cx.notify();
            },
        )))
        .separator()
        .item(PopupMenuItem::new("Delete Section").on_click(window.listener_for(
            &view,
            move |this, _, window, cx| {
                this.show_section_delete_dialog(window, cx, section_id5.clone());
                cx.notify();
            },
        )))
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
    options: SectionBlockOptions,
) -> impl IntoElement {
    let view_clone = view.clone();
    let add_button = Button::new(format!("add-item-to-section-{}", section_id))
        .small()
        .ghost()
        .compact()
        .icon(IconName::PlusLargeSymbolic)
        .label("Add Task")
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
        .dropdown_menu(build_section_more_menu(view_clone.clone(), section_id.clone()));

    let mut block = section(section_name);

    if options.show_inline_edit_delete {
        block = block.sub_title(h_flex().gap_1().child(add_button));
        block = block.sub_title(
            h_flex()
                .gap_1()
                .child(
                    Button::new(format!("edit-section-{}", section_id))
                        .small()
                        .ghost()
                        .compact()
                        .icon(IconName::EditSymbolic)
                        .on_click({
                            let view = view_clone.clone();
                            let section_id = section_id.clone();
                            move |_, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.show_section_dialog(
                                        window,
                                        cx,
                                        Some(section_id.clone()),
                                        true,
                                    );
                                    cx.notify();
                                })
                            }
                        }),
                )
                .child(
                    Button::new(format!("delete-section-{}", section_id))
                        .small()
                        .ghost()
                        .compact()
                        .icon(IconName::UserTrashSymbolic)
                        .on_click({
                            let view = view_clone.clone();
                            let section_id = section_id.clone();
                            move |_, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.show_section_delete_dialog(window, cx, section_id.clone());
                                    cx.notify();
                                })
                            }
                        }),
                )
                .child(more_button),
        );
    } else {
        block = block.sub_title(h_flex().gap_1().child(add_button).child(more_button));
    }

    block.child(render_item_list(items, item_rows, active_index, active_border, view_clone))
}

/// 渲染「No Section」区块
pub fn render_no_section_block<V: BoardSectionActions>(
    items: &[(usize, Arc<ItemModel>)],
    item_rows: &[Entity<ItemRowState>],
    active_index: Option<usize>,
    active_border: Hsla,
    view: Entity<V>,
    compact_toolbar: bool,
) -> impl IntoElement {
    let view_clone = view.clone();

    let add_button = Button::new("add-item-to-no-section")
        .small()
        .ghost()
        .compact()
        .icon(IconName::PlusLargeSymbolic)
        .label("Add Task")
        .on_click({
            let view = view_clone.clone();
            move |_, window, cx| {
                view.update(cx, |this, cx| {
                    this.show_item_dialog(window, cx, false, None);
                    cx.notify();
                })
            }
        });

    let more_button = Button::new("more-no-section")
        .small()
        .ghost()
        .compact()
        .icon(IconName::EllipsisVertical)
        .dropdown_menu({
            let view = view_clone.clone();
            move |this, window, _cx| {
                this.item(PopupMenuItem::new("+ Add Task").on_click(window.listener_for(
                    &view,
                    |this, _, window, cx| {
                        this.show_item_dialog(window, cx, false, None);
                        cx.notify();
                    },
                )))
                .separator()
                .item(PopupMenuItem::new("Show Completed Tasks").on_click(
                    window.listener_for(&view, |_this, _, _window, cx| {
                        cx.notify();
                    }),
                ))
            }
        });

    let mut block = section("No Section");

    if compact_toolbar {
        block = block.sub_title(h_flex().gap_1().child(add_button).child(more_button));
    } else {
        block = block
            .sub_title(h_flex().gap_1().child(add_button))
            .sub_title(h_flex().gap_1().child(more_button));
    }

    block.child(render_item_list(items, item_rows, active_index, active_border, view_clone))
}

/// 渲染简单分组（标题 + 可选更多菜单 + 任务列表），用于 Pinned / Today 等虚拟分组
pub fn render_simple_group_block<V: BoardView + Render>(
    title: &str,
    items: &[(usize, Arc<ItemModel>)],
    item_rows: &[Entity<ItemRowState>],
    active_index: Option<usize>,
    active_border: Hsla,
    view: Entity<V>,
    show_more_menu: bool,
) -> impl IntoElement {
    let view_clone = view.clone();
    let mut block = section(title);

    if show_more_menu {
        block = block.sub_title(
            h_flex().gap_1().child(
                Button::new(format!("more-{}", title.to_lowercase().replace(' ', "-")))
                    .small()
                    .ghost()
                    .compact()
                    .icon(IconName::EllipsisVertical)
                    .dropdown_menu({
                        let view = view_clone.clone();
                        move |this, window, _cx| {
                            this.item(PopupMenuItem::new("Show Completed Tasks").on_click(
                                window.listener_for(&view, |_this, _, _window, cx| {
                                    cx.notify();
                                }),
                            ))
                        }
                    }),
            ),
        );
    }

    block.child(render_item_list(items, item_rows, active_index, active_border, view_clone))
}

/// 渲染带 Schedule 按钮的简单分组，用于 Past Due 等
pub fn render_group_with_schedule_button<V: BoardView + Render>(
    title: &str,
    items: &[(usize, Arc<ItemModel>)],
    item_rows: &[Entity<ItemRowState>],
    active_index: Option<usize>,
    active_border: Hsla,
    view: Entity<V>,
    schedule_button: &Entity<ScheduleButtonState>,
) -> impl IntoElement {
    let view_clone = view.clone();

    section(title)
        .sub_title(
            h_flex()
                .gap_1()
                .child(crate::ui::components::ScheduleButton::new(schedule_button))
                .child(
                    Button::new(format!("more-{}", title.to_lowercase().replace(' ', "-")))
                        .small()
                        .ghost()
                        .compact()
                        .icon(IconName::EllipsisVertical)
                        .dropdown_menu({
                            let view = view_clone.clone();
                            move |this, window, _cx| {
                                this.item(PopupMenuItem::new("Show Completed Tasks").on_click(
                                    window.listener_for(&view, |_this, _, _window, cx| {
                                        cx.notify();
                                    }),
                                ))
                            }
                        }),
                ),
        )
        .child(render_item_list(items, item_rows, active_index, active_border, view_clone))
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
) -> impl IntoElement {
    let view_clone = view.clone();

    let add_button = Button::new(format!("add-item-to-section-{}", section_id))
        .small()
        .ghost()
        .compact()
        .icon(IconName::PlusLargeSymbolic)
        .label("Add Task")
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
        .dropdown_menu(build_section_more_menu(view_clone.clone(), section_id.clone()));

    section(section_name)
        .sub_title(h_flex().gap_1().child(leading).child(add_button).child(more_button))
        .child(render_item_list(items, item_rows, active_index, active_border, view_clone))
}
