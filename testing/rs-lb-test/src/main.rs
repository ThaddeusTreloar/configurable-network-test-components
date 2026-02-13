use std::collections::HashMap;
use std::time::Duration;
use std::{error::Error, sync::Arc};

use figment::Figment;
use figment::providers::Env;
use hyper_util::rt::TokioIo;
use tokio::select;
use tokio::signal::unix::{SignalKind, signal};
use tokio::{net::TcpListener, spawn};

use crate::config::LoadBalancerConfiguration;
use crate::connection_pool::TargetGroupsConnectionPools;
use crate::health_monitor::HealthMonitor;
use crate::listener::ListenerRule;
use crate::load_balancer::LoadBalancer;
use crate::target::{TargetGroup, TargetGroupCreationError};

mod cache;
mod config;
mod connection_manager;
mod connection_pool;
mod health_monitor;
mod listener;
mod load_balancer;
mod selector;
mod target;

async fn listen(listener: TcpListener, balancer: Arc<LoadBalancer>) {
    while let Ok((stream, _)) = listener.accept().await {
        let balancer_ref = balancer.clone();
        let conn = TokioIo::new(stream);
        let handler_fut = balancer_ref.serve_connection(conn);

        spawn(handler_fut);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let load_balancer_configuration = match Figment::new()
        .merge(Env::prefixed("APP_").split("__"))
        .extract()
    {
        Ok(cfg) => cfg,
        Err(e) => {
            log::error!("Error while parsing config: {e}");

            return Err(Box::new(e));
        }
    };

    log::info!("Using configuation:\n{}", load_balancer_configuration);

    let LoadBalancerConfiguration {
        listener_port,
        connection_timout,
        connection_pool_size,
        listener_rules: raw_listener_rules,
        target_groups: raw_target_groups,
        cache_enabled,
        cache_ttl: cache_ttl_ms,
        ..
    } = load_balancer_configuration;

    let listener = TcpListener::bind(format!("0.0.0.0:{}", listener_port))
        .await
        .expect("Failed to create listener");

    let listener_rules: Vec<ListenerRule> = raw_listener_rules
        .into_values()
        .map(ListenerRule::from)
        .collect();

    let target_groups = raw_target_groups
        .iter()
        .map(|(k, v)| TargetGroup::try_from(v).map(|tg| (k.clone(), tg)))
        .collect::<Result<HashMap<String, TargetGroup>, TargetGroupCreationError>>()
        .map_err(Box::new)?;

    let connection_pools =
        TargetGroupsConnectionPools::try_from_target_groups(&target_groups, connection_pool_size)
            .await
            .map_err(Box::new)?;

    let health_check_connection_pools =
        TargetGroupsConnectionPools::try_from_target_groups(&target_groups, 1)
            .await
            .map_err(Box::new)?;

    if let Some(health_monitor) =
        HealthMonitor::new(health_check_connection_pools, &raw_target_groups)
    {
        spawn(health_monitor.health_monitor_thread());
    }

    let mut balancer = LoadBalancer::new(
        listener_rules,
        &connection_pools,
        Duration::from_millis(connection_timout),
    )
    .await;

    if cache_enabled {
        balancer = balancer.with_cache(Duration::from_millis(cache_ttl_ms));
    }

    let balancer_arc = Arc::new(balancer);

    log::info!("Serving connections at: 0.0.0.0:{}", listener_port);

    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    select!(
      _ = listen(listener, balancer_arc) => {},
      _ = sigint.recv() => {
        log::info!("Recieved SIGINT, shutting down...")
      },
      _ = sigterm.recv() => {
        log::info!("Recieved SIGTERM, shutting down...")
      },
    );

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    if let Err(e) = run().await {
        log::error!("Encountered fatal error: {}", e)
    }
}
