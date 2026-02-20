//! 通用 Board 渲染组件
//!
//! 这个模块提供了可复用的 Board 渲染逻辑，减少各 Board 组件的重复代码。
//! 由于 GPUI 的生命周期限制，这些函数只在 Board 内部使用。

use std::sync::Arc;

use gpui::{
    Entity, Hsla, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder,
};
use gpui_component::{IconName, v_flex};
use todos::entity::ItemModel;

use super::board_base::BoardView;
use crate::{ItemRow, ItemRowState, VisualHierarchy, section};

/// Board 配置结构
///
/// 用于定义 Board 的标题、描述、图标和颜色
pub struct BoardConfig {
    pub title: &'static str,
    pub description: &'static str,
    pub icon: IconName,
    pub colors: Vec<Hsla>,
}

/// 预定义的 Board 配置
pub mod configs {
    use gpui::rgb;

    use super::*;

    /// Today Board 配置
    pub fn today() -> BoardConfig {
        BoardConfig {
            title: "Today",
            description: "今天需要完成的任务",
            icon: IconName::StarOutlineThickSymbolic,
            colors: vec![rgb(0x33d17a).into()],
        }
    }

    /// Inbox Board 配置
    pub fn inbox() -> BoardConfig {
        BoardConfig {
            title: "Inbox",
            description: "收件箱",
            icon: IconName::MailSymbolic,
            colors: vec![rgb(0x3584e4).into()],
        }
    }

    /// Scheduled Board 配置
    pub fn scheduled() -> BoardConfig {
        BoardConfig {
            title: "Scheduled",
            description: "已安排的任务",
            icon: IconName::ClockSymbolic,
            colors: vec![rgb(0xff7800).into()],
        }
    }

    /// Pinned Board 配置
    pub fn pinned() -> BoardConfig {
        BoardConfig {
            title: "Pinned",
            description: "置顶的任务",
            icon: IconName::PinSymbolic,
            colors: vec![rgb(0xe5a50a).into()],
        }
    }

    /// Completed Board 配置
    pub fn completed() -> BoardConfig {
        BoardConfig {
            title: "Completed",
            description: "已完成的任务",
            icon: IconName::CheckmarkSmallSymbolic,
            colors: vec![rgb(0x77767b).into()],
        }
    }
}

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

/// 渲染带标题的任务区块（Pinned / Overdue 等）
pub fn render_item_section<V>(
    title: impl ToString,
    items: &[(usize, Arc<ItemModel>)],
    item_rows: &[Entity<ItemRowState>],
    active_index: Option<usize>,
    active_border: gpui::Hsla,
    view: Entity<V>,
) -> impl IntoElement
where
    V: BoardView + Render,
{
    section(title.to_string()).child(render_item_list(
        items,
        item_rows,
        active_index,
        active_border,
        view,
    ))
}
