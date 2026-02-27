//! 保存状态指示器
//!
//! 在标题栏显示当前保存状态：
//! - 空闲：不显示
//! - 保存中：显示 "正在保存..."
//! - 错误：显示 "保存失败" + 错误图标

use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div, px};
use gpui_component::{
    ActiveTheme, IconName, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    label::Label,
};

use crate::core::state::{PendingTasksState, SaveStatus};

pub struct SaveStatusIndicator {
    last_status: SaveStatus,
}

impl SaveStatusIndicator {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { last_status: SaveStatus::Idle }
    }

    fn refresh_status(&mut self, cx: &mut Context<Self>) {
        let status = cx.global::<PendingTasksState>().save_status();
        self.last_status = status;
    }
}

impl Render for SaveStatusIndicator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        self.refresh_status(cx);

        let theme = cx.theme();
        let status = self.last_status;

        match status {
            SaveStatus::Idle => div().into_any_element(),
            SaveStatus::Saving => h_flex()
                .gap_1()
                .items_center()
                .px_1()
                .py_0p5()
                .rounded(px(4.))
                .bg(theme.accent.opacity(0.2))
                .child(Label::new("正在保存...").text_xs().text_color(theme.foreground))
                .into_any_element(),
            SaveStatus::HasError => h_flex()
                .gap_1()
                .items_center()
                .px_1()
                .py_0p5()
                .rounded(px(4.))
                .bg(theme.red.opacity(0.2))
                .child(Button::new("error-icon").icon(IconName::TriangleAlert).small().ghost())
                .child(Label::new("保存失败").text_xs().text_color(theme.foreground))
                .into_any_element(),
        }
    }
}
