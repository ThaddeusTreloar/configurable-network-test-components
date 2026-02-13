use std::time::Duration;

use figment::{Figment, providers::Env};
use futures::{StreamExt, stream::FuturesUnordered};
use tokio::signal::unix::{SignalKind, signal};

use crate::{config::AppConfig, stats::StatisticsManager, targets::ClientTargets};

mod config;
mod stats;
mod targets;

#[tokio::main]
async fn main() {
    // Start
    env_logger::init();

    let AppConfig {
        statistics_interval,
        targets: target_configs,
    } = match Figment::new()
        .merge(Env::prefixed("APP_").split("__"))
        .extract()
    {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error while parsing config: {e}");

            return;
        }
    };

    let stats_manager =
        StatisticsManager::default().with_interval(Duration::from_millis(statistics_interval));

    let target_futures = target_configs
        .iter()
        .map(|(n, c)| (n.as_str(), c, &stats_manager))
        .map(ClientTargets::from)
        .map(|t| t.run_client_targets())
        .collect::<FuturesUnordered<_>>();

    let _statistics_handle = tokio::spawn(stats_manager.run_statistics());

    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to get interrupt signal");
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to get terminate signal");

    tokio::select!(
      _ =  target_futures
          .for_each_concurrent(Option::None, async |r| match r {
              Ok(_) => (),
              Err(e) => eprintln!("Error for client target thread: {e}"),
          }) =>  {},
      _ = sigint.recv() => {
        println!("Recieved SIGINT, shutting down...")
      },
      _ = sigterm.recv() => {
        println!("Recieved SIGTERM, shutting down...")
      },
    );
}
