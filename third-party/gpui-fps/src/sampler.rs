//! sampler.rs - 帧采样器（关键修改：适配新版 gpui 的 FrameEvent 枚举）
//!
//! 主要改动点：
//! 1. gpui 新版的 FrameTimingCollector::collect_unseen() 返回 Vec<FrameEvent> 而不是旧版的
//!    Vec<FrameTiming>。需要用 filter_map 过滤出 FrameEvent::Draw。
//! 2. FrameTiming/FrameTimingCollector 仍然存在，但需要启用 gpui 的 "profiler" feature。

use std::{collections::VecDeque, time::Duration};

// 🔧 修复点1：新增导入 FrameEvent，因为 collect_unseen() 现在返回它的枚举
use gpui::{FrameEvent, FrameTiming, FrameTimingCollector, WindowId};
use instant::Instant;

/// 超过这个时长的帧不再计入 FPS 读数
const FPS_WINDOW: Duration = Duration::from_secs(1);

/// 单帧采样数据结构
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FrameSample {
    /// 本帧在 Window::draw 中的耗时
    pub draw: Duration,
    /// 合并到本帧的无效化次数（过高说明 UI 刷新请求过于频繁）
    pub invalidations: u64,
}

/// 从 GPUI 全局帧追踪中按窗口过滤出采样数据。
///
/// GPUI 把帧时序写入进程级的环形缓冲区，因此必须按 window_id 过滤，
/// 否则会把其他窗口的帧也算进来。
pub(crate) struct FrameSampler {
    collector: FrameTimingCollector,
    window_id: WindowId,
    samples: VecDeque<FrameSample>,
    /// FPS 滚动窗口内帧的到达时间
    frame_times: VecDeque<Instant>,
    capacity: usize,
}

impl FrameSampler {
    /// 新建指定窗口的采样器
    pub(crate) fn new(window_id: WindowId, capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            collector: FrameTimingCollector::new(),
            window_id,
            samples: VecDeque::with_capacity(capacity),
            frame_times: VecDeque::new(),
            capacity,
        }
    }

    /// 取出并消费上次调用后新产生的所有帧。每渲染帧调用一次。
    pub(crate) fn tick(&mut self) {
        // 🔧 修复点2：新版 collect_unseen() 返回 Vec<FrameEvent>
        // FrameEvent::Draw(FrameTiming) 才是我们需要的绘制帧
        // FrameEvent::Present(PresentTiming) 是垂直同步相关事件，不需要
        let events = self.collector.collect_unseen();
        let timings: Vec<FrameTiming> = events
            .into_iter()
            .filter_map(|event| match event {
                FrameEvent::Draw(timing) => Some(timing),
                FrameEvent::Present(_) => None,
            })
            .collect();
        self.ingest(timings, Instant::now());
    }

    /// 调整采样保留容量
    pub(crate) fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        while self.samples.len() > self.capacity {
            self.samples.pop_front();
        }
    }

    /// 计算 FPS（滚动窗口内）。
    ///
    /// n 个帧点划定 n-1 个间隔，因此用时间跨度而不是原始数量计算，
    /// 这样在滚动窗口未满时读数也是正确的。
    pub(crate) fn fps(&self) -> f32 {
        if self.frame_times.len() < 2 {
            return 0.;
        }
        let (Some(oldest), Some(newest)) = (self.frame_times.front(), self.frame_times.back())
        else {
            return 0.;
        };
        let span = newest.duration_since(*oldest).as_secs_f32();
        if span <= 0. {
            return 0.;
        }
        (self.frame_times.len() - 1) as f32 / span
    }

    pub(crate) fn samples(&self) -> impl ExactSizeIterator<Item = &FrameSample> {
        self.samples.iter()
    }

    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }

    /// 超出预算帧的占比（0..1）
    pub(crate) fn over_budget_ratio(&self, budget: Duration) -> f32 {
        if self.samples.is_empty() {
            return 0.;
        }
        let over = self.samples.iter().filter(|sample| sample.draw > budget).count();
        over as f32 / self.samples.len() as f32
    }

    /// 保留帧的平均绘制耗时
    pub(crate) fn mean_draw(&self) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.samples.iter().map(|sample| sample.draw).sum();
        total / self.samples.len() as u32
    }

    /// 最慢帧的耗时（用于确定图表 y 轴上限）
    pub(crate) fn peak_draw(&self) -> Duration {
        self.samples.iter().map(|sample| sample.draw).max().unwrap_or_default()
    }

    /// 把一批 FrameTiming 存入采样队列
    fn ingest(&mut self, timings: Vec<FrameTiming>, now: Instant) {
        for timing in timings {
            // 只统计当前窗口的帧
            if timing.window_id != self.window_id {
                continue;
            }

            if self.samples.len() == self.capacity {
                self.samples.pop_front();
            }
            self.samples.push_back(FrameSample {
                draw: timing.draw_duration(),
                invalidations: timing.invalidations,
            });
            self.frame_times.push_back(now);
        }

        // 移除 FPS 滚动窗口之外的旧帧到达时间
        while let Some(oldest) = self.frame_times.front() {
            if now.duration_since(*oldest) > FPS_WINDOW {
                self.frame_times.pop_front();
            } else {
                break;
            }
        }
    }
}

// ------------------------------
// 资源采样：CPU & 内存（无修改）
// ------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ResourceSample {
    /// 进程 CPU 使用率（单核 100% = 100，全部核跑满 = 核心数 * 100，已除以核心数归一化）
    pub cpu_percent: f32,
    /// 进程驻留内存（字节）
    pub memory_bytes: u64,
}

/// CPU 内存采样探针（阻塞式系统调用，必须放在后台线程）
#[cfg(not(target_family = "wasm"))]
pub(crate) struct ResourceProbe {
    system: sysinfo::System,
    pid: sysinfo::Pid,
    cores: f32,
}

#[cfg(not(target_family = "wasm"))]
impl ResourceProbe {
    pub(crate) fn new() -> Option<Self> {
        let pid = sysinfo::get_current_pid().ok()?;
        let cores =
            std::thread::available_parallelism().map(|cores| cores.get() as f32).unwrap_or(1.);

        let mut probe = Self { system: sysinfo::System::new(), pid, cores };
        // 第一次刷新只是建立基线，cpu_usage 是和上次的差值，首次读为 0
        probe.refresh();
        Some(probe)
    }

    pub(crate) fn sample(&mut self) -> Option<ResourceSample> {
        self.refresh();
        let process = self.system.process(self.pid)?;
        Some(ResourceSample {
            cpu_percent: (process.cpu_usage() / self.cores).min(100.),
            memory_bytes: process.memory(),
        })
    }

    fn refresh(&mut self) {
        self.system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[self.pid]),
            false,
            sysinfo::ProcessRefreshKind::nothing().with_cpu().with_memory(),
        );
    }
}

/// CPU 采样的最小有效间隔（小于此间隔无意义）
#[cfg(not(target_family = "wasm"))]
pub(crate) fn minimum_resource_interval() -> Duration {
    sysinfo::MINIMUM_CPU_UPDATE_INTERVAL
}

#[cfg(target_family = "wasm")]
pub(crate) fn minimum_resource_interval() -> Duration {
    Duration::from_millis(200)
}

// ------------------------------
// 单元测试（适配了 FrameTiming 构造函数的新字段）
// ------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    // 构造一个测试用的 FrameTiming
    fn timing(window_id: WindowId, draw: Duration) -> FrameTiming {
        let start = std::time::Instant::now();
        FrameTiming {
            window_id,
            dirty_at: None,
            invalidations: 1,
            draw_start: start,
            draw_end: start + draw,
        }
    }

    #[test]
    fn ignores_frames_from_other_windows() {
        let ours = WindowId::from(1);
        let theirs = WindowId::from(2);
        let mut sampler = FrameSampler::new(ours, 8);
        let now = Instant::now();

        sampler.ingest(
            vec![
                timing(ours, Duration::from_millis(8)),
                timing(theirs, Duration::from_millis(40)),
                timing(ours, Duration::from_millis(9)),
            ],
            now,
        );

        assert_eq!(sampler.samples().len(), 2);
        assert_eq!(sampler.peak_draw(), Duration::from_millis(9));
    }

    #[test]
    fn drops_oldest_samples_beyond_capacity() {
        let window_id = WindowId::from(1);
        let mut sampler = FrameSampler::new(window_id, 2);
        let now = Instant::now();

        for millis in [5, 6, 7] {
            sampler.ingest(vec![timing(window_id, Duration::from_millis(millis))], now);
        }

        let draws: Vec<_> = sampler.samples().map(|sample| sample.draw).collect();
        assert_eq!(draws, vec![Duration::from_millis(6), Duration::from_millis(7)]);
    }

    fn measure_fps(count: u64, interval: Duration) -> f32 {
        let window_id = WindowId::from(1);
        let mut sampler = FrameSampler::new(window_id, 256);
        let start = Instant::now();

        for frame in 0..count {
            sampler.ingest(
                vec![timing(window_id, Duration::from_millis(1))],
                start + interval * frame as u32,
            );
        }
        sampler.fps()
    }

    #[test]
    fn fps_is_frames_divided_by_the_span_they_cover() {
        assert!((measure_fps(11, Duration::from_millis(10)) - 100.).abs() < 0.5);
        assert!((measure_fps(101, Duration::from_millis(1)) - 1000.).abs() < 5.);
    }

    #[test]
    fn fps_matches_the_common_refresh_rates() {
        for (interval_micros, expected) in
            [(16_667, 60.), (8_333, 120.), (33_333, 30.), (6_944, 144.)]
        {
            let interval = Duration::from_micros(interval_micros);
            let count = 1_000_000 / interval_micros;
            let measured = measure_fps(count, interval);
            assert!(
                (measured - expected).abs() < 1.,
                "{interval_micros}us frames measured {measured}, expected {expected}"
            );
        }
    }

    #[test]
    fn fps_needs_two_frames_to_have_a_rate_at_all() {
        assert_eq!(measure_fps(0, Duration::from_millis(10)), 0.);
        assert_eq!(measure_fps(1, Duration::from_millis(10)), 0.);
        assert!(measure_fps(2, Duration::from_millis(10)) > 0.);
    }

    #[test]
    fn simultaneous_frames_do_not_divide_by_zero() {
        let window_id = WindowId::from(1);
        let mut sampler = FrameSampler::new(window_id, 64);
        let now = Instant::now();

        sampler.ingest(
            vec![
                timing(window_id, Duration::from_millis(4)),
                timing(window_id, Duration::from_millis(4)),
                timing(window_id, Duration::from_millis(4)),
            ],
            now,
        );

        assert_eq!(sampler.fps(), 0.);
    }

    #[test]
    fn frames_outside_the_rolling_window_stop_counting() {
        let window_id = WindowId::from(1);
        let mut sampler = FrameSampler::new(window_id, 64);
        let start = Instant::now();

        for frame in 0..10 {
            sampler.ingest(
                vec![timing(window_id, Duration::from_millis(4))],
                start + Duration::from_millis(frame * 10),
            );
        }
        assert!(sampler.fps() > 0.);

        sampler.ingest(vec![], start + Duration::from_secs(2));
        assert_eq!(sampler.fps(), 0.);
        assert_eq!(sampler.samples().len(), 10);
    }
}
