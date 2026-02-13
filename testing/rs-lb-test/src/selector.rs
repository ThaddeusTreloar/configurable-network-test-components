use std::sync::atomic::{AtomicUsize, Ordering};

pub struct RoundRobin(AtomicUsize);

impl RoundRobin {
    pub fn new() -> Self {
        Self(AtomicUsize::new(0))
    }

    pub fn next_wrapping(&self, limit: usize) -> usize {
        self.0.fetch_add(1, Ordering::Relaxed) % limit
    }
}
