use std::{
    collections::HashMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use log::info;
use tokio::time::{Instant, sleep};

#[derive()]
pub(crate) struct StatisticsManager {
    target_stats: Arc<RwLock<HashMap<String, TargetStatistics>>>,
    interval: Duration,
}

impl Default for StatisticsManager {
    fn default() -> Self {
        Self {
            target_stats: Default::default(),
            interval: Duration::from_millis(1000),
        }
    }
}

impl StatisticsManager {
    pub(crate) fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub(crate) fn create_stats_for_target(&self, target_name: &str) -> TargetStatistics {
        let mut guard = self
            .target_stats
            .write()
            .expect("Failed to aquire write lock");
        let stats = TargetStatistics::default();

        guard.insert(target_name.to_owned(), stats.clone());

        stats
    }

    pub(crate) async fn run_statistics(self) {
        loop {
            let iteration_start_time = Instant::now();
            sleep(self.interval).await;
            let guard = self
                .target_stats
                .read()
                .expect("Failed to aquire read lock");

            guard.iter().for_each(|(name, stats)| {
                self.print_statistic(name, stats, iteration_start_time);
            });
        }
    }

    fn print_statistic(&self, name: &str, stats: &TargetStatistics, iteration_start_time: Instant) {
        let requests_for_iteration = stats.requests.swap(0, Ordering::Relaxed);
        let request_time_acc = stats.response_time_acc.swap(0, Ordering::Relaxed);

        let seconds_elapsed = iteration_start_time.elapsed().as_secs_f64();

        let requests_per_second = requests_for_iteration as f64 / seconds_elapsed;
        let average_response_time = request_time_acc / requests_for_iteration;

        println!(
            "Stats for target: {}, clients: {}, requests/s: {}, avg response time: {}ms",
            name,
            stats.clients.load(Ordering::Relaxed),
            requests_per_second as usize,
            average_response_time,
        );
    }
}

#[derive(Default, Clone)]
pub(crate) struct TargetStatistics {
    pub requests: Arc<AtomicUsize>,
    pub response_time_acc: Arc<AtomicUsize>,
    pub clients: Arc<AtomicUsize>,
}
