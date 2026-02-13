use std::time::Duration;

use tokio::time::Instant;

#[derive(Debug)]
pub(crate) struct SlidingFailureWindow {
    failures: Vec<Instant>,
    window_size: Duration,
    failure_threshold: usize,
}

impl Default for SlidingFailureWindow {
    fn default() -> Self {
        Self {
            failures: Default::default(),
            window_size: Duration::from_millis(1000),
            failure_threshold: 10,
        }
    }
}

impl SlidingFailureWindow {
    pub(crate) fn new(window_size: Duration, failure_threshold: usize) -> Self {
        Self {
            window_size,
            failure_threshold,
            ..Default::default()
        }
    }

    pub(crate) fn append_failure(&mut self) {
        self.failures.push(Instant::now());
    }

    fn update_window(&mut self) {
        let instant = Instant::now()
            .checked_sub(self.window_size)
            .expect("Failed to calculate failure window start time.");

        self.failures.retain(|i| i > &instant);
    }

    pub(crate) fn threshold_exceeded(&mut self) -> bool {
        self.update_window();

        self.failures.len() >= self.failure_threshold
    }

    pub(crate) fn current_failure_count(&self) -> usize {
        self.failures.len()
    }
}
