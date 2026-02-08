//! Performance metrics and monitoring

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::RwLock;

/// Performance metrics for database operations
#[derive(Debug, Clone)]
pub struct OperationMetrics {
    pub count: u64,
    pub total_duration: Duration,
    pub avg_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl OperationMetrics {
    pub fn new() -> Self {
        Self {
            count: 0,
            total_duration: Duration::ZERO,
            avg_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    pub fn record(&mut self, duration: Duration, cache_hit: bool) {
        self.count += 1;
        self.total_duration += duration;
        self.avg_duration = self.total_duration / self.count as u32;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);

        if cache_hit {
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }
    }

    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 { 0.0 } else { self.cache_hits as f64 / total as f64 }
    }
}

impl Default for OperationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics collector for performance monitoring
#[derive(Clone, Debug)]
pub struct MetricsCollector {
    metrics: Arc<RwLock<HashMap<String, OperationMetrics>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self { metrics: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Record an operation with its duration and cache hit status
    pub async fn record(&self, operation: &str, duration: Duration, cache_hit: bool) {
        let mut metrics = self.metrics.write().await;
        let op_metrics = metrics.entry(operation.to_string()).or_default();
        op_metrics.record(duration, cache_hit);
    }

    /// Get metrics for a specific operation
    pub async fn get_metrics(&self, operation: &str) -> Option<OperationMetrics> {
        let metrics = self.metrics.read().await;
        metrics.get(operation).cloned()
    }

    /// Get all metrics
    pub async fn get_all_metrics(&self) -> HashMap<String, OperationMetrics> {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Clear all metrics
    pub async fn clear(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.clear();
    }

    /// Start a timer for an operation
    pub fn start_timer(&self, operation: &str) -> Timer {
        Timer::new(operation.to_string(), self.clone())
    }

    /// Record an operation with count
    pub async fn record_operation(&self, operation: &str, count: usize) {
        // This is a simplified version, just record a zero duration
        self.record(operation, Duration::ZERO, false).await;
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer for measuring operation duration
pub struct Timer {
    start: Instant,
    operation: String,
    collector: MetricsCollector,
}

impl Timer {
    pub fn new(operation: String, collector: MetricsCollector) -> Self {
        Self { start: Instant::now(), operation, collector }
    }

    pub async fn stop(self, cache_hit: bool) {
        let duration = self.start.elapsed();
        self.collector.record(&self.operation, duration, cache_hit).await;
    }
}

/// Macro to easily time operations
#[macro_export]
macro_rules! time_operation {
    ($collector:expr, $operation:expr, $async_block:expr) => {{
        use $crate::services::metrics::Timer;
        let timer = Timer::new($operation.to_string(), $collector.clone());
        let result = $async_block.await;
        let cache_hit = result.is_ok(); // Simple heuristic, customize as needed
        timer.stop(cache_hit).await;
        result
    }};
}
