use scc::Queue;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{debug, error, info};

use crate::logic::SyndicaAppLogic;

const WORKERS_COUNT: usize = 5;
const INTERVAL_SIZE: u64 = 100;
const MIN_INTERVAL_SIZE: u64 = 5;
const POLL_DIVIDER: u64 = 10;

#[derive(Debug, Clone)]
struct SlotInterval {
    start: u64,
    end: u64,
}

impl SlotInterval {
    fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    fn size(&self) -> u64 {
        if self.end >= self.start {
            self.end - self.start + 1
        } else {
            0
        }
    }
}

/// The Synchronizer is designed to efficiently monitor Solana blockchain
/// blocks while minimizing RPC traffic.
///
/// Key Design Decisions:
/// 1. Interval-based Processing:
///    - Instead of checking every slot individually, we process slots in intervals
///    - This significantly reduces RPC calls by batching slot checks
///    - Intervals are dynamically sized based on monitoring depth
///
/// 2. Queue-based Architecture:
///    - Uses a concurrent queue to manage slot intervals
///    - Enables parallel processing of different slot ranges
///    - Provides backpressure when processing falls behind
///
/// 3. Dual Task System:
///    - Slot Updater: Continuously monitors new slots
///    - History Updater: Processes historical slots in parallel
///    - Separation allows independent scaling of real-time vs historical processing
///
/// Future Optimizations:
/// 1. Adaptive Interval Sizing:
///    - Dynamically adjust interval size based on network conditions
///    - Implement exponential backoff for failed intervals
///    - Add interval merging for sparse regions
///
/// 2. Performance Enhancements:
///    - Add batch processing for multiple intervals
///    - Implement priority queue for newer slots
///    - Add circuit breaker for RPC rate limiting
pub struct Synchronizer {
    logic: Arc<SyndicaAppLogic>,
    monitor_interval_ms: u64,
    monitoring_depth: usize,
    interval_queue: Arc<Queue<SlotInterval>>,
}

impl Synchronizer {
    pub fn new(
        logic: Arc<SyndicaAppLogic>,
        monitor_interval_ms: u64,
        monitoring_depth: usize,
    ) -> Self {
        Self {
            logic,
            monitor_interval_ms,
            monitoring_depth,
            interval_queue: Arc::new(Queue::<SlotInterval>::default()),
        }
    }

    pub async fn run(&mut self) {
        info!("Starting block synchronizer");
        let slot_updater_handle = self.spawn_slot_updater().await;
        let history_updater_handle = self.spawn_history_updater().await;

        tokio::select! {
            _ = slot_updater_handle => {
                error!("Slot updater task ended unexpectedly");
            }
            _ = history_updater_handle => {
                error!("History updater task ended unexpectedly");
            }
        }
    }

    async fn spawn_slot_updater(&mut self) -> JoinHandle<()> {
        let logic = Arc::clone(&self.logic);
        let monitor_interval_ms = self.monitor_interval_ms;
        let interval_queue = Arc::clone(&self.interval_queue);
        let monitoring_depth = self.monitoring_depth;

        tokio::spawn(async move {
            let mut interval_timer = interval(Duration::from_millis(monitor_interval_ms));
            info!(
                "Slot updater started - updating every {}ms",
                monitor_interval_ms
            );
            let mut last_tracked_slot: u64 = 0;

            loop {
                match logic.update_latest_slot().await {
                    Ok(start_slot) => {
                        info!(start_slot, "Updated latest slot");
                        let begin_slot = std::cmp::max(
                            last_tracked_slot + 1,
                            start_slot - monitoring_depth as u64,
                        );
                        if begin_slot <= start_slot {
                            let interval = SlotInterval::new(begin_slot, start_slot);
                            info!(
                                start = interval.start,
                                end = interval.end,
                                size = interval.size(),
                                "Added interval to queue"
                            );
                            interval_queue.push(interval);
                        }
                        last_tracked_slot = start_slot;
                    }
                    Err(e) => {
                        error!("Failed to update starting slot: {}", e);
                    }
                }
                interval_timer.tick().await;
            }
        })
    }

    async fn spawn_history_updater(&mut self) -> JoinHandle<()> {
        let logic = Arc::clone(&self.logic);
        let monitoring_depth = self.monitoring_depth;
        let monitor_interval_ms = self.monitor_interval_ms;
        let interval_queue = Arc::clone(&self.interval_queue);

        tokio::spawn(async move {
            info!("History updater started with {} workers", WORKERS_COUNT);

            let mut worker_handles = Vec::new();
            for worker_id in 0..WORKERS_COUNT {
                let worker_logic = Arc::clone(&logic);
                let worker_queue = Arc::clone(&interval_queue);

                let handle = tokio::spawn(async move {
                    Self::interval_worker(
                        worker_id,
                        worker_logic,
                        worker_queue,
                        monitoring_depth,
                        monitor_interval_ms,
                    )
                    .await;
                });
                worker_handles.push(handle);
            }

            for handle in worker_handles {
                if let Err(e) = handle.await {
                    error!("Worker task ended unexpectedly: {}", e);
                }
            }
        })
    }

    async fn interval_worker(
        worker_id: usize,
        logic: Arc<SyndicaAppLogic>,
        queue: Arc<Queue<SlotInterval>>,
        monitoring_depth: usize,
        monitor_interval_ms: u64,
    ) {
        info!(worker_id, "History worker started");

        loop {
            if let Some(interval) = queue.pop() {
                info!(
                    worker_id,
                    start = interval.start,
                    end = interval.end,
                    size = interval.size(),
                    "Worker got interval from queue"
                );

                match Self::process_interval(&logic, &interval).await {
                    Ok(sub_intervals) => {
                        for sub_interval in sub_intervals {
                            let interval_size_ok = sub_interval.size() >= MIN_INTERVAL_SIZE;
                            let interval_end_ok = sub_interval.end
                                > logic.state().last_processed_slot() - monitoring_depth as u64;
                            if interval_size_ok && interval_end_ok {
                                queue.push(sub_interval.clone());
                                debug!(
                                    worker_id,
                                    start = sub_interval.start,
                                    end = sub_interval.end,
                                    size = sub_interval.size(),
                                    "Added sub-interval to queue"
                                );
                            } else if !interval_size_ok {
                                info!(
                                    worker_id,
                                    start = sub_interval.start,
                                    end = sub_interval.end,
                                    size = sub_interval.size(),
                                    "Sub-interval size is too small"
                                );
                            } else if !interval_end_ok {
                                info!(
                                    worker_id,
                                    start = sub_interval.start,
                                    end = sub_interval.end,
                                    size = sub_interval.size(),
                                    "Sub-interval end is too far behind"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            worker_id,
                            start = interval.start,
                            end = interval.end,
                            error = %e,
                            "Failed to process interval"
                        );
                        queue.push(SlotInterval::new(interval.start, interval.end));
                    }
                }
                debug!(worker_id, "No interval to process - sleeping briefly");
                tokio::time::sleep(Duration::from_millis(monitor_interval_ms / POLL_DIVIDER)).await;
            } else {
                info!(worker_id, "No interval to process - sleeping");
                tokio::time::sleep(Duration::from_millis(monitor_interval_ms)).await;
            }
        }
    }

    async fn process_interval(
        logic: &Arc<SyndicaAppLogic>,
        interval: &SlotInterval,
    ) -> Result<Vec<SlotInterval>, Box<dyn std::error::Error + Send + Sync>> {
        let confirmed_blocks = logic.get_blocks(interval.start, interval.end).await?;
        logic.query_slot_range(interval.start, interval.end).await?;
        let mut sub_intervals = Vec::new();
        let mut current_pos = interval.start;

        for &confirmed_slot in &confirmed_blocks {
            if confirmed_slot > current_pos {
                let gap_start = current_pos;
                let gap_end = confirmed_slot - 1;
                let desired_end = std::cmp::min(
                    std::cmp::max(gap_end, gap_start + INTERVAL_SIZE - 1),
                    interval.end,
                );
                sub_intervals.push(SlotInterval::new(gap_start, desired_end));
                current_pos = desired_end + 1;
            } else {
                current_pos = confirmed_slot + 1;
            }
        }

        if current_pos <= interval.end {
            sub_intervals.push(SlotInterval::new(current_pos, interval.end));
        }

        info!(
            start = interval.start,
            end = interval.end,
            confirmed_count = confirmed_blocks.len(),
            sub_intervals_count = sub_intervals.len(),
            "Processed interval"
        );

        Ok(sub_intervals)
    }
}
