use std::{pin::Pin, time::Duration};

use futures::{StreamExt, stream::FuturesUnordered};
use log::info;
use tokio::{select, task::JoinHandle, time::sleep};

use crate::{
    config::{RampStrategy, TargetConfig},
    stats::{StatisticsManager, TargetStatistics},
    targets::{
        client::ClientTarget,
        error::{ClientTargetError, ClientTargetsError},
        window::SlidingFailureWindow,
    },
};

mod client;
mod error;
mod window;

type ClientThreadFutures = FuturesUnordered<Pin<Box<JoinHandle<Result<(), ClientTargetError>>>>>;

#[derive()]
pub(crate) struct ClientTargets {
    name: String,
    statistics: TargetStatistics,
    target_config: TargetConfig,
    initial_ramp_time: Duration,
    inital_ramp: usize,
    client_count_ramp: usize,
    client_target_threads: ClientThreadFutures,
    failure_window: SlidingFailureWindow,
}

impl From<(&str, &TargetConfig, &StatisticsManager)> for ClientTargets {
    fn from((name, value, stats): (&str, &TargetConfig, &StatisticsManager)) -> Self {
        Self {
            name: name.to_owned(),
            statistics: stats.create_stats_for_target(name),
            target_config: value.clone(),
            initial_ramp_time: Duration::from_millis(value.client_count_ramp_interval),
            inital_ramp: value.client_count_start,
            client_count_ramp: value.client_count_ramp,
            client_target_threads: Default::default(),
            failure_window: SlidingFailureWindow::new(
                Duration::from_millis(value.client_error_threshold_window),
                value.client_error_threshold,
            ),
        }
    }
}

impl ClientTargets {
    fn next_ramp_time(&self, current_ramp_time: Duration) -> Duration {
        match self.target_config.client_count_ramp_strategy {
            RampStrategy::Step => current_ramp_time,
        }
    }

    fn next_ramp(&self, _current_ramp: usize) -> usize {
        match self.target_config.client_count_ramp_strategy {
            RampStrategy::Step => self.client_count_ramp,
        }
    }

    fn create_client_targets(&mut self, current_ramp: usize) {
        println!("Creating {current_ramp} client targets");

        for _ in 0..current_ramp {
            let client = ClientTarget::new(&self.target_config, &self.statistics);

            self.client_target_threads
                .push(Box::pin(tokio::spawn(client.run_client())));
        }

        self.statistics
            .clients
            .fetch_add(current_ramp, std::sync::atomic::Ordering::Relaxed);
    }

    pub(crate) async fn run_client_targets(mut self) -> Result<(), ClientTargetsError> {
        let mut current_ramp = self.inital_ramp;
        let mut current_ramp_time = self.initial_ramp_time;

        loop {
            select! {
              () = sleep(current_ramp_time) => {
                  self.create_client_targets(current_ramp);
                  current_ramp = self.next_ramp(current_ramp);

                  current_ramp_time = self.next_ramp_time(current_ramp_time);
              }
              result = self.client_target_threads.next() => {
                  match result {
                      Some(Err(e)) => {
                        eprintln!("Encountered client failure with error: {e}");
                        self.failure_window.append_failure();

                        if self.failure_window.threshold_exceeded() {
                          // Cancel all exiting client threads
                          self.client_target_threads.clear();

                          return Err(ClientTargetsError::FailureThresholdExceeded(
                              self.failure_window.current_failure_count(),
                          ));
                        } else {
                          self.create_client_targets(1);
                        }
                      },
                      Some(Ok(Err(e))) => {
                        eprintln!("Encountered client failure with error: {e}");
                        self.failure_window.append_failure();

                        if self.failure_window.threshold_exceeded() {
                          // Cancel all exiting client threads
                          self.client_target_threads.clear();

                          eprintln!("Client error threshold for target: {}, exceeded treshold", self.name);

                          return Err(ClientTargetsError::FailureThresholdExceeded(
                              self.failure_window.current_failure_count(),
                          ));
                        } else {
                          self.create_client_targets(1);
                        }
                      },
                      Some(Ok(_)) => {
                        eprintln!("Client thread exited for unknown reason");
                        self.create_client_targets(1);
                      }
                      None => {
                          self.create_client_targets(current_ramp);
                          current_ramp = self.next_ramp(current_ramp);

                          current_ramp_time = self.next_ramp_time(current_ramp_time);
                      }
                  }
              },
            }
        }
    }
}
