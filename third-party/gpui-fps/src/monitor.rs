//! monitor.rs - FPS HUD 主视图（逻辑无需修改，仅调整导入）

use std::time::Duration;

use gpui::{
    canvas, div, point, prelude::FluentBuilder as _, px, relative, Bounds, Context, Div, Hsla,
    InteractiveElement as _, IntoElement, ParentElement, PathBuilder, Pixels, Point, Render,
    StatefulInteractiveElement as _, Styled, Task, Window,
};
use instant::Instant;

use crate::{
    sampler::{minimum_resource_interval, FrameSampler, ResourceSample},
    style::FpsStyle,
    FrameTraceGuard,
};

/// 60Hz 的单帧预算时间
const DEFAULT_FRAME_BUDGET: Duration = Duration::from_nanos(16_666_667);
const DEFAULT_CAPACITY: usize = 120;
const DEFAULT_RESOURCE_INTERVAL: Duration = Duration::from_millis(500);

/// 图表 y 轴上限在尖峰过后的衰减系数（立即上涨，缓慢回落）
const AXIS_DECAY: f32 = 0.04;

/// HUD 固定宽度，让数字列对齐
const HUD_WIDTH: Pixels = px(172.);
const COMPACT_FIGURE_WIDTH: Pixels = px(25.);

/// 文本尺寸
const TEXT_SIZE: Pixels = px(10.);

/// 帧时间图表在背景上的不透明度
const TRACE_OPACITY: f32 = 0.35;

const HEADLINE_HEIGHT: Pixels = px(35.);

const FIGURE_SIZE: Pixels = px(28.);
const FIGURE_WIDTH: Pixels = px(70.);

const UNIT_WIDTH: Pixels = px(22.);

/// 读数刷新间隔（500ms 一次，避免数字闪烁无法阅读）
const READOUT_INTERVAL: Duration = Duration::from_millis(500);

/// 达标容错率（60Hz 显示实际读数约 58-60 都算正常）
const FPS_TOLERANCE: f32 = 0.95;

/// 默认等宽字体（不同平台使用系统自带字体）
#[cfg(target_os = "macos")]
const DEFAULT_FONT: &str = "Menlo";
#[cfg(target_os = "windows")]
const DEFAULT_FONT: &str = "Consolas";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const DEFAULT_FONT: &str = "monospace";

/// HUD 最新一次发布到屏幕上的读数快照
#[derive(Clone, Copy, Default)]
struct Readout {
    fps: f32,
    frame_millis: f32,
    dropped_percent: f32,
}

/// FPS 监视器实体视图。
///
/// 单独作为一个 view 而不是嵌入父视图，是因为它需要持续请求动画帧，
/// 作为独立 view 时每次重绘只会刷新这个 HUD 子树，不会让整个父视图一起重绘。
pub struct FpsMonitor {
    sampler: FrameSampler,
    readout: Readout,
    readout_at: Option<Instant>,
    style: FpsStyle,
    frame_budget: Duration,
    /// 是否持续请求重绘（类似游戏内 FPS 计数器）
    continuous: bool,
    show_resources: bool,
    resource_interval: Duration,
    resources: Option<ResourceSample>,
    compact: bool,
    /// 图表 y 轴上限（秒）
    axis_max: f32,
    resource_task: Option<Task<()>>,
    _frame_trace: FrameTraceGuard,
}

impl FpsMonitor {
    /// 新建监视器，绑定到指定窗口
    pub fn new(window: &Window, _cx: &mut Context<Self>) -> Self {
        let frame_budget = DEFAULT_FRAME_BUDGET;
        Self {
            sampler: FrameSampler::new(window.window_handle().window_id(), DEFAULT_CAPACITY),
            readout: Readout::default(),
            readout_at: None,
            style: FpsStyle::default(),
            frame_budget,
            continuous: true,
            show_resources: true,
            resource_interval: DEFAULT_RESOURCE_INTERVAL,
            resources: None,
            compact: false,
            axis_max: frame_budget.as_secs_f32() * 2.,
            resource_task: None,
            _frame_trace: FrameTraceGuard::acquire(),
        }
    }

    /// 设置图表保留帧数（默认 120）
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.sampler.set_capacity(capacity);
        self
    }

    /// 设置单帧预算（用于基线和颜色分级）。默认 60Hz，144Hz 屏可设为 1/144s
    pub fn frame_budget(mut self, budget: Duration) -> Self {
        self.frame_budget = budget;
        self.axis_max = budget.as_secs_f32() * 2.;
        self
    }

    /// 是否持续每帧重绘（默认 true，此时帧率读数为程序"能跑到多少"而不是实际绘制频率）
    pub fn continuous(mut self, continuous: bool) -> Self {
        self.continuous = continuous;
        self
    }

    /// 是否采样并显示 CPU/内存（默认 true，Web 上始终关闭）
    pub fn show_resources(mut self, show_resources: bool) -> Self {
        self.show_resources = show_resources;
        self
    }

    /// CPU/内存采样间隔（默认 500ms，会被向上夹到 sysinfo 最小有效间隔）
    pub fn resource_interval(mut self, interval: Duration) -> Self {
        self.resource_interval = interval;
        self
    }

    /// 启动后台资源采样任务（阻塞式系统调用必须放在后台）
    #[cfg(not(target_family = "wasm"))]
    fn start_resource_sampling(&mut self, cx: &mut Context<Self>) {
        use crate::sampler::ResourceProbe;

        if !self.show_resources || self.resource_task.is_some() {
            return;
        }

        let interval = self.resource_interval.max(minimum_resource_interval());
        self.resource_task = Some(cx.spawn(async move |this, cx| {
            let executor = cx.background_executor().clone();
            // ResourceProbe 在后台创建，避免阻塞渲染线程
            let Some(mut probe) = executor.spawn(async { ResourceProbe::new() }).await else {
                return;
            };

            loop {
                executor.timer(interval).await;

                // 采样一次并把 probe 送回来（下轮继续用）
                let (returned, sample) = executor
                    .spawn(async move {
                        let sample = probe.sample();
                        (probe, sample)
                    })
                    .await;
                probe = returned;

                let Some(sample) = sample else { continue };
                let updated = this.update(cx, |this, cx| {
                    this.resources = Some(sample);
                    cx.notify();
                });
                if updated.is_err() {
                    break;
                }
            }
        }));
    }

    #[cfg(target_family = "wasm")]
    fn start_resource_sampling(&mut self, _cx: &mut Context<Self>) {
        let _ = minimum_resource_interval();
    }

    /// 按 READOUT_INTERVAL 节拍发布新的读数到屏幕
    fn update_readout(&mut self) {
        let now = Instant::now();
        let due = self.readout_at.is_none_or(|at| now.duration_since(at) >= READOUT_INTERVAL);
        if !due {
            return;
        }

        self.readout = Readout {
            fps: self.sampler.fps(),
            // 使用窗口内平均值而不是单帧值，避免读数乱跳
            frame_millis: self.sampler.mean_draw().as_secs_f32() * 1000.,
            dropped_percent: self.sampler.over_budget_ratio(self.frame_budget) * 100.,
        };
        self.readout_at = Some(now);
    }

    /// 更新图表 y 轴上限（尖峰立即跟上，缓慢衰减回落）
    fn update_axis(&mut self) {
        let floor = self.frame_budget.as_secs_f32() * 2.;
        let target = self.sampler.peak_draw().as_secs_f32().max(floor);
        self.axis_max = if target > self.axis_max {
            target
        } else {
            self.axis_max + (target - self.axis_max) * AXIS_DECAY
        };
    }

    /// 渲染背景帧时间折线图
    fn render_chart(&self) -> impl IntoElement {
        let style = self.style;
        let budget = self.frame_budget.as_secs_f32();
        let axis_max = self.axis_max.max(f32::EPSILON);
        let capacity = self.sampler.capacity();
        let samples: Vec<(f32, Hsla)> = self
            .sampler
            .samples()
            .map(|sample| {
                let seconds = sample.draw.as_secs_f32();
                (
                    (seconds / axis_max).clamp(0., 1.),
                    style.level_color(seconds, budget).opacity(TRACE_OPACITY),
                )
            })
            .collect();

        canvas(
            |_, _, _| (),
            move |bounds: Bounds<Pixels>, _, window, _| {
                let slot = bounds.size.width / capacity as f32;
                // 新帧靠右对齐（历史向左滚动），而不是按样本数拉伸
                let leading = capacity.saturating_sub(samples.len());
                let points: Vec<(Point<Pixels>, Hsla)> = samples
                    .iter()
                    .enumerate()
                    .map(|(index, (ratio, color))| {
                        (
                            point(
                                bounds.origin.x + slot * (leading + index) as f32 + slot / 2.,
                                bounds.origin.y + bounds.size.height * (1. - *ratio),
                            ),
                            *color,
                        )
                    })
                    .collect();

                // 连续同色段合并成一条 Path（性能优化）
                let mut start = 0;
                while start + 1 < points.len() {
                    let color = points[start + 1].1;
                    let mut path = PathBuilder::stroke(px(1.));
                    path.move_to(points[start].0);

                    let mut end = start + 1;
                    while end < points.len() && points[end].1 == color {
                        path.line_to(points[end].0);
                        end += 1;
                    }

                    if let Ok(path) = path.build() {
                        window.paint_path(path, color);
                    }
                    // 边界点在下一轮继续用，保证线段不断
                    start = end - 1;
                }
            },
        )
        .absolute()
        .inset_0()
    }

    /// 渲染顶部大字 FPS 读数 + 背景折线图
    fn render_headline(&self, fps: f32, color: Hsla) -> Div {
        let style = self.style;

        div()
            .relative()
            .overflow_hidden()
            .w_full()
            .h(HEADLINE_HEIGHT)
            .child(self.render_chart())
            .child(
                div()
                    .flex()
                    .size_full()
                    .items_end()
                    .justify_center()
                    .gap_1()
                    // 左侧占位，和右侧的 FPS 单位宽度对称，保证数字居中
                    .child(div().w(UNIT_WIDTH))
                    .child(
                        div()
                            .w(FIGURE_WIDTH)
                            .text_center()
                            .text_size(FIGURE_SIZE)
                            .line_height(relative(1.))
                            .text_color(color)
                            .child(format!("{fps:.0}")),
                    )
                    .child(div().w(UNIT_WIDTH).text_color(style.muted).child("FPS")),
            )
    }
}

impl Render for FpsMonitor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 每帧：采样 → 读数 → 轴缩放 → 启动资源采样 → （可选）请求下一帧
        self.sampler.tick();
        self.update_readout();
        self.update_axis();
        self.start_resource_sampling(cx);
        if self.continuous {
            window.request_animation_frame();
        }

        let style = self.style;
        let budget = self.frame_budget;
        let Readout { fps, frame_millis, dropped_percent: dropped } = self.readout;
        let fps_color = fps_color(fps, budget, style);
        let resources = self.resources.filter(|_| self.show_resources);
        let compact = self.compact;

        div()
            .id("gpui-fps-hud")
            .flex()
            .bg(style.background)
            .font_family(DEFAULT_FONT)
            .text_size(TEXT_SIZE)
            .text_color(style.muted)
            // 点击 HUD 在完整/紧凑两种模式间切换
            .on_click(cx.listener(|this, _, _, cx| {
                this.compact = !this.compact;
                cx.notify();
            }))
            .map(|this| {
                if compact {
                    // 紧凑模式：小标签，只显示 FPS 数字
                    this.items_center()
                        .gap_1()
                        .px_1p5()
                        .py_0p5()
                        .rounded(px(3.))
                        .child(
                            div()
                                .w(COMPACT_FIGURE_WIDTH)
                                .text_right()
                                .text_color(fps_color)
                                .child(format!("{fps:.0}")),
                        )
                        .child("FPS")
                } else {
                    // 完整模式：显示 FPS 大图 + 帧时 + 掉帧率 + CPU + 内存
                    this.flex_col()
                        .w(HUD_WIDTH)
                        .px_2()
                        .py_1p5()
                        .rounded(px(4.))
                        .child(self.render_headline(fps, fps_color))
                        .child(reading(
                            "FRAME",
                            format!("{frame_millis:.1} ms"),
                            style.foreground,
                            style,
                        ))
                        .child(reading(
                            "DROP",
                            format!("{dropped:.1}%"),
                            style.level_color(if dropped > 0. { 1. } else { 0. }, 0.5),
                            style,
                        ))
                        .when_some(resources, |this, resources| {
                            this.child(
                                div()
                                    .flex()
                                    .w_full()
                                    .justify_between()
                                    .gap_2()
                                    .py(px(1.))
                                    .child(pair(
                                        "CPU",
                                        format!("{:.1}%", resources.cpu_percent),
                                        style,
                                    ))
                                    .child(pair(
                                        "MEM",
                                        format_bytes(resources.memory_bytes),
                                        style,
                                    )),
                            )
                        })
                }
            })
    }
}

/// 根据 FPS 和预算给颜色分级（考虑到 vsync 的实际读数不会完全等于理论值）
fn fps_color(fps: f32, budget: Duration, style: FpsStyle) -> Hsla {
    if fps <= 0. {
        return style.muted;
    }

    let target = 1. / budget.as_secs_f32();
    if fps >= target * FPS_TOLERANCE {
        style.good
    } else if fps >= target * 0.5 {
        style.warn
    } else {
        style.bad
    }
}

/// 标签-数值 对（用于 CPU/MEM 行）
fn pair(label: &'static str, value: String, style: FpsStyle) -> Div {
    div()
        .flex()
        .gap_1()
        .child(div().text_color(style.muted).child(label))
        .child(div().text_color(style.foreground).child(value))
}

/// LABEL ................ value 的整行读数列（右对齐让数字列整齐）
fn reading(label: &'static str, value: String, value_color: Hsla, style: FpsStyle) -> Div {
    div()
        .flex()
        .w_full()
        .justify_between()
        .gap_2()
        .py(px(1.))
        .child(div().text_color(style.muted).child(label))
        .child(div().text_color(value_color).child(value))
}

/// 按量级格式化内存字节数
fn format_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024. * 1024.;
    const GIB: f64 = MIB * 1024.;

    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.2} GB", bytes / GIB)
    } else {
        format!("{:.0} MB", bytes / MIB)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    #[gpui::test]
    fn test_fps_monitor_builder(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let budget = Duration::from_micros(6_944);
            let monitor = cx.new(|cx| {
                FpsMonitor::new(window, cx)
                    .capacity(240)
                    .frame_budget(budget)
                    .continuous(false)
                    .show_resources(false)
                    .resource_interval(Duration::from_secs(2))
            });

            let monitor = monitor.read(cx);
            assert_eq!(monitor.sampler.capacity(), 240);
            assert_eq!(monitor.frame_budget, budget);
            assert!(!monitor.continuous);
            assert!(!monitor.show_resources);
            assert_eq!(monitor.resource_interval, Duration::from_secs(2));
            assert_eq!(monitor.axis_max, budget.as_secs_f32() * 2.);
        });
    }

    #[test]
    fn a_display_keeping_up_is_never_graded_as_falling_behind() {
        let style = FpsStyle::dark();
        let budget = DEFAULT_FRAME_BUDGET;

        // 60Hz 屏健康运行时实际读数大约在 58-61 之间
        for rate in [58., 59., 59.7, 60., 61.] {
            assert_eq!(
                fps_color(rate, budget, style),
                style.good,
                "{rate} fps should read as healthy on a 60Hz display"
            );
        }

        assert_eq!(fps_color(45., budget, style), style.warn);
        assert_eq!(fps_color(20., budget, style), style.bad);
        assert_eq!(fps_color(0., budget, style), style.muted);
    }

    #[test]
    fn formats_memory_by_magnitude() {
        assert_eq!(format_bytes(184 * 1024 * 1024), "184 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.00 GB");
    }
}
