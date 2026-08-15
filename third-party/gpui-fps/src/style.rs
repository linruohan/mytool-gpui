//! style.rs - FPS HUD 的调色板定义（无需修改，兼容新旧版 gpui）

use gpui::{hsla, Hsla};

/// FPS HUD 使用的颜色样式。
/// 调色板不对外暴露，因为其对比度是固定的，不能随意修改。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FpsStyle {
    /// HUD 背景色
    pub background: Hsla,
    /// 主要读数（FPS 数字）
    pub foreground: Hsla,
    /// 次要读数（单位、标签、资源行）
    pub muted: Hsla,
    /// 在帧预算内完成的帧颜色
    pub good: Hsla,
    /// 超出预算但未超过两倍的帧颜色
    pub warn: Hsla,
    /// 超出两倍预算的帧颜色
    pub bad: Hsla,
}

impl Default for FpsStyle {
    fn default() -> Self {
        Self::dark()
    }
}

impl FpsStyle {
    /// 暗色 HUD 样式，在任何窗口背景上都清晰可读。
    pub(crate) fn dark() -> Self {
        Self {
            background: hsla(0., 0., 0.04, 0.92),
            foreground: hsla(0., 0., 0.98, 1.),
            muted: hsla(0., 0., 0.62, 1.),
            good: hsla(0.41, 0.95, 0.56, 1.),
            warn: hsla(0.11, 0.95, 0.6, 1.),
            bad: hsla(0.99, 0.9, 0.62, 1.),
        }
    }

    /// 根据帧耗时和预算返回对应的颜色等级
    pub(crate) fn level_color(&self, frame_secs: f32, budget_secs: f32) -> Hsla {
        if frame_secs <= budget_secs {
            self.good
        } else if frame_secs <= budget_secs * 2. {
            self.warn
        } else {
            self.bad
        }
    }
}
