//! overlay.rs - FPS HUD 的覆盖层组件（无需修改，兼容新旧版 gpui）

use gpui::{
    div, prelude::FluentBuilder as _, px, Anchor, App, Entity, IntoElement, ParentElement, Pixels,
    RenderOnce, Styled, Window,
};

use crate::monitor::FpsMonitor;

/// HUD 距离父容器边缘的间距
const MARGIN: Pixels = px(12.);

/// 把 FpsMonitor 固定到父容器的某个角落或边缘的覆盖层组件。
///
/// 大多数情况直接用 `fps_monitor()` 即可，不需要手动构造此结构体。
/// 注意：父元素必须设置为 `relative()`，否则定位会出错。
#[derive(IntoElement)]
pub struct FpsOverlay {
    monitor: Entity<FpsMonitor>,
    anchor: Anchor,
}

impl FpsOverlay {
    /// 创建新的 FPS 覆盖层，默认在右上角
    pub fn new(monitor: &Entity<FpsMonitor>) -> Self {
        Self { monitor: monitor.clone(), anchor: Anchor::TopRight }
    }

    /// 设置 HUD 停靠位置，默认 TopRight（右上角）
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }
}

impl RenderOnce for FpsOverlay {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let margin = MARGIN;

        // 根据不同的 anchor 枚举值设置对应的定位样式
        div()
            .absolute()
            .flex()
            .map(|this| match self.anchor {
                Anchor::TopLeft => this.top(margin).left(margin),
                Anchor::TopRight => this.top(margin).right(margin),
                Anchor::BottomLeft => this.bottom(margin).left(margin),
                Anchor::BottomRight => this.bottom(margin).right(margin),
                Anchor::TopCenter => this.top(margin).left_0().right_0().justify_center(),
                Anchor::BottomCenter => this.bottom(margin).left_0().right_0().justify_center(),
                Anchor::LeftCenter => this.left(margin).top_0().bottom_0().items_center(),
                Anchor::RightCenter => this.right(margin).top_0().bottom_0().items_center(),
            })
            .child(self.monitor)
    }
}
