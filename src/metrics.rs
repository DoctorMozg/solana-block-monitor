use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, trace, warn};

const SLOW_OPERATION_THRESHOLD_MS: u64 = 1000;

pub trait Metrics {
    fn record_latest_slot(&self, slot: u64);
    fn record_get_blocks_elapsed(&self, elapsed: std::time::Duration);
    fn record_is_slot_confirmed_elapsed(&self, elapsed: std::time::Duration);
    fn record_cache_hit(&self, hit: bool);
}

#[derive(Default, Clone)]
pub struct TracingMetrics;

impl TracingMetrics {
    pub fn new() -> Self {
        Self
    }

    fn get_timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn log_performance(&self, operation: &str, elapsed: Duration) {
        let elapsed_ms = elapsed.as_millis() as u64;
        let elapsed_micros = elapsed.as_micros() as u64;

        let log_level = if elapsed_ms > SLOW_OPERATION_THRESHOLD_MS {
            "slow"
        } else if elapsed_ms > SLOW_OPERATION_THRESHOLD_MS / 2 {
            "moderate"
        } else {
            "fast"
        };

        if elapsed_ms > SLOW_OPERATION_THRESHOLD_MS {
            warn!(
                target: "metrics::performance::slow",
                operation = operation,
                elapsed_ms = elapsed_ms,
                elapsed_micros = elapsed_micros,
                threshold_ms = SLOW_OPERATION_THRESHOLD_MS,
                performance = log_level,
                "Slow operation detected"
            );
        } else {
            debug!(
                target: "metrics::performance",
                operation = operation,
                elapsed_ms = elapsed_ms,
                elapsed_micros = elapsed_micros,
                performance = log_level,
                "Operation completed"
            );
        }

        trace!(
            target: "metrics::timing",
            operation = operation,
            elapsed_ns = elapsed.as_nanos() as u64,
            elapsed_micros = elapsed_micros,
            elapsed_ms = elapsed_ms,
            timestamp = Self::get_timestamp_ms(),
            "Detailed timing information"
        );
    }
}

impl Metrics for TracingMetrics {
    fn record_latest_slot(&self, slot: u64) {
        info!(
            target: "metrics::blockchain",
            slot = slot,
            metric_type = "latest_slot",
            timestamp = Self::get_timestamp_ms(),
            "Latest slot recorded"
        );

        trace!(
            target: "metrics::slot_tracking",
            slot = slot,
            event = "slot_update",
            timestamp = Self::get_timestamp_ms(),
            "Slot tracking update"
        );
    }

    fn record_get_blocks_elapsed(&self, elapsed: Duration) {
        let elapsed_ms = elapsed.as_millis() as u64;

        info!(
            target: "metrics::rpc",
            operation = "get_blocks",
            elapsed_ms = elapsed_ms,
            metric_type = "operation_duration",
            "RPC get_blocks operation completed"
        );

        self.log_performance("get_blocks", elapsed);
    }

    fn record_is_slot_confirmed_elapsed(&self, elapsed: Duration) {
        let elapsed_ms = elapsed.as_millis() as u64;

        info!(
            target: "metrics::rpc",
            operation = "is_slot_confirmed",
            elapsed_ms = elapsed_ms,
            metric_type = "operation_duration",
            "RPC is_slot_confirmed operation completed"
        );

        self.log_performance("is_slot_confirmed", elapsed);
    }

    fn record_cache_hit(&self, hit: bool) {
        let cache_result = if hit { "hit" } else { "miss" };

        info!(
            target: "metrics::cache",
            cache_result = cache_result,
            hit = hit,
            metric_type = "cache_performance",
            timestamp = Self::get_timestamp_ms(),
            "Cache operation recorded"
        );

        trace!(
            target: "metrics::cache_tracking",
            hit = hit,
            result = cache_result,
            event = "cache_access",
            timestamp = Self::get_timestamp_ms(),
            "Cache access tracking"
        );
    }
}

#[derive(Default)]
pub struct NoOpMetrics;

impl Metrics for NoOpMetrics {
    fn record_latest_slot(&self, _slot: u64) {}
    fn record_get_blocks_elapsed(&self, _elapsed: Duration) {}
    fn record_is_slot_confirmed_elapsed(&self, _elapsed: Duration) {}
    fn record_cache_hit(&self, _hit: bool) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_metrics_trait_implementation() {
        let metrics = TracingMetrics::new();

        metrics.record_latest_slot(12345);

        metrics.record_get_blocks_elapsed(Duration::from_millis(100));

        metrics.record_is_slot_confirmed_elapsed(Duration::from_millis(50));

        metrics.record_cache_hit(true);
        metrics.record_cache_hit(false);
    }

    #[test]
    fn test_noop_metrics() {
        let metrics = NoOpMetrics::default();

        metrics.record_latest_slot(12345);
        metrics.record_get_blocks_elapsed(Duration::from_millis(100));
        metrics.record_is_slot_confirmed_elapsed(Duration::from_millis(50));
        metrics.record_cache_hit(true);
        metrics.record_cache_hit(false);
    }

    #[test]
    fn test_cache_hit_miss_tracking() {
        let metrics = TracingMetrics::new();

        metrics.record_cache_hit(true);
        metrics.record_cache_hit(false);
        metrics.record_cache_hit(true);
    }
}
