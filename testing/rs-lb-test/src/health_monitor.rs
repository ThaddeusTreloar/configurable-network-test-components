use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use futures::{Stream, StreamExt, stream::FuturesUnordered};
use http::{Method, Request, StatusCode, Uri};
use http_body_util::Empty;
use hyper::body::{Body, Bytes, Incoming};
use tokio::{
    select,
    sync::RwLock,
    time::{Instant, sleep},
};

use crate::{
    config::{TargetGroupConfiguration, TargetGroupHealthCheckConfiguration},
    connection_pool::{
        TargetConnectionPool, TargetConnectionPoolCloneError, TargetGroupsConnectionPools,
    },
};

pub struct HealthMonitor {
    pub health_check_targets: Vec<TargetGroupHealthCheck>,
}

impl HealthMonitor {
    pub async fn new(
        connection_pools: HashMap<String, Arc<RwLock<Vec<TargetConnectionPool<Incoming>>>>>,
        target_group_configurations: &HashMap<String, TargetGroupConfiguration>,
    ) -> Result<Option<Self>, TargetGroupHealthCheckCreationError> {
        let mut health_check_targets = Vec::new();

        for (group_name, connection_pool) in connection_pools.iter() {
            let health_check_config = target_group_configurations
                .get(group_name)
                .map(|c| &c.health_check)
                .map(|c| (c.enabled, c));

            match health_check_config {
                None | Some((false, _)) => continue,
                Some((true, config)) => {
                    let target_group_health_check =
                        TargetGroupHealthCheck::new(connection_pool.clone(), config).await?;

                    health_check_targets.push(target_group_health_check);
                }
            }
        }

        if health_check_targets.is_empty() {
            Ok(Option::None)
        } else {
            Ok(Option::Some(Self {
                health_check_targets,
            }))
        }
    }

    pub async fn health_monitor_thread(self) {
        let mut health_check_threads = self
            .health_check_targets
            .into_iter()
            .map(TargetGroupHealthCheck::run_health_check_cycle)
            .collect::<FuturesUnordered<_>>();

        while let Some(target) = health_check_threads.next().await {
            health_check_threads.push(target.run_health_check_cycle());
        }
    }
}

pub enum HealthCheckStats {
    SuccessfulCheckCount(usize),
    UnsuccessfulCheckCount(usize),
}

impl HealthCheckStats {
    pub fn new_healthy() -> Self {
        HealthCheckStats::UnsuccessfulCheckCount(0)
    }

    pub fn check_health(&mut self, failure_threshold: usize, success_threshold: usize) -> bool {
        match self {
            Self::UnsuccessfulCheckCount(count) if *count > failure_threshold => {
                self.mark_unhealthy();
                false
            }
            Self::UnsuccessfulCheckCount(_) => true,
            Self::SuccessfulCheckCount(count) if *count > success_threshold => {
                self.mark_healthy();
                true
            }
            Self::SuccessfulCheckCount(_) => false,
        }
    }

    pub fn mark_unhealthy(&mut self) {
        std::mem::replace(self, Self::SuccessfulCheckCount(0));
    }

    pub fn mark_healthy(&mut self) {
        std::mem::replace(self, Self::UnsuccessfulCheckCount(0));
    }

    pub fn register_health_check(&mut self, is_successful: bool) {
        match (is_successful, self) {
            (true, Self::UnsuccessfulCheckCount(0)) => (),
            (true, Self::UnsuccessfulCheckCount(count)) => *count -= 1,
            (true, Self::SuccessfulCheckCount(count)) => *count += 1,
            (false, Self::SuccessfulCheckCount(0)) => (),
            (false, Self::SuccessfulCheckCount(count)) => *count -= 1,
            (false, Self::UnsuccessfulCheckCount(count)) => *count += 1,
        }
    }
}

pub enum PoolPosition {
    Healthy(usize),
    Unhealthy(usize),
}

impl PoolPosition {
    fn in_healthy_queue(&self) -> bool {
        match self {
            Self::Healthy(_) => true,
            Self::Unhealthy(_) => false,
        }
    }
}

pub struct HealthCheckTarget {
    connection_pool: TargetConnectionPool<Empty<Bytes>>,
    health_check_stats: HealthCheckStats,
    pool_position: PoolPosition,
    pub success_threshold: usize,
    pub failure_threshold: usize,
}

impl HealthCheckTarget {
    fn update_pool_position(&mut self, pool_position: PoolPosition) -> PoolPosition {
        std::mem::replace(&mut self.pool_position, pool_position)
    }

    fn is_healthy(&mut self) -> bool {
        self.health_check_stats
            .check_health(self.failure_threshold, self.success_threshold)
    }

    async fn run_check_health(&mut self, path: &str, timeout: Duration) {
        let uri = match Uri::builder().path_and_query(path).build() {
            Ok(u) => u,
            Err(e) => {
                log::error!("Failed to build uri for health check: {}", e);

                self.health_check_stats.register_health_check(false);
                return;
            }
        };

        let request = match Request::builder()
            .uri(uri)
            .method(Method::GET)
            .body(Empty::new())
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to build request for health check: {}", e);

                self.health_check_stats.register_health_check(false);
                return;
            }
        };

        let mut target = match self.connection_pool.connection_pool.get().await {
            Ok(t) => t,
            Err(e) => {
                log::error!("Failed to get pooled connection for health check: {}", e);

                self.health_check_stats.register_health_check(false);
                return;
            }
        };

        if let Err(e) = target.ready().await {
            log::error!("Failed to get ready connection during health check: {}", e);
            self.health_check_stats.register_health_check(false);
            return;
        };

        select! {
          response_result = target.send_request(request) => {
            match response_result.map(|r|r.status()) {
                Ok(StatusCode::OK) => self.health_check_stats.register_health_check(true),
                Ok(s) => {
                  log::error!("Health check failed with status: {}", e);
                  self.health_check_stats.register_health_check(false);
                },
                Err(e) => {
                  log::error!("Failed to send request during health check: {}", e);
                  self.health_check_stats.register_health_check(false);
                }
            }
          },
          _ = sleep(timeout) => {
            log::error!("Health check request timeout");
            self.health_check_stats.register_health_check(false);
          }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TargetGroupHealthCheckCreationError {
    #[error("Failed to create health check pool for target group health check, error: {0}")]
    CreateHealthCheckPool(TargetConnectionPoolCloneError),
}

pub struct TargetGroupHealthCheck {
    pub source_connection_pool: Arc<RwLock<Vec<TargetConnectionPool<Incoming>>>>,
    pub unhealthy_connection_pool: Vec<TargetConnectionPool<Incoming>>,
    pub healthy_health_check_connection_pool: Vec<HealthCheckTarget>,
    pub unhealthy_health_check_connection_pool: Vec<HealthCheckTarget>,
    pub timeout: Duration,
    pub path: String,
    pub interval: Duration,
}

impl TargetGroupHealthCheck {
    pub async fn new(
        connection_pool: Arc<RwLock<Vec<TargetConnectionPool<Incoming>>>>,
        health_check_configuration: &TargetGroupHealthCheckConfiguration,
    ) -> Result<Self, TargetGroupHealthCheckCreationError> {
        let mut health_check_connection_pool = Vec::new();

        let connection_pool_guard = connection_pool.read().await;

        for (idx, pool) in connection_pool_guard.iter().enumerate() {
            let health_check_pool = pool
                .create_health_check_pool()
                .await
                .map_err(TargetGroupHealthCheckCreationError::CreateHealthCheckPool)?;

            health_check_connection_pool.push(HealthCheckTarget {
                connection_pool: health_check_pool,
                health_check_stats: HealthCheckStats::new_healthy(),
                pool_position: PoolPosition::Healthy(idx),
                failure_threshold: health_check_configuration.success_threshold,
                success_threshold: health_check_configuration.failure_threshold,
            });
        }

        drop(connection_pool_guard);

        Ok(Self {
            source_connection_pool: connection_pool,
            healthy_health_check_connection_pool: health_check_connection_pool,
            unhealthy_health_check_connection_pool: Default::default(),
            unhealthy_connection_pool: Default::default(),
            timeout: Duration::from_millis(health_check_configuration.timeout),
            path: health_check_configuration.path.clone(),
            interval: Duration::from_millis(health_check_configuration.interval),
        })
    }

    pub async fn run_health_check_cycle(mut self) -> Self {
        log::debug!("Running health check cycle");
        let health_check_start_time = Instant::now();

        // self.check_healthy_connection_pools().await;
        // self.check_unhealthy_connection_pools().await;
        // self.filter_healthy_connection_pools().await;
        // self.filter_unhealthy_connection_pools().await;

        let health_check_duration = health_check_start_time.elapsed();

        if health_check_duration < self.interval {
            sleep(self.interval - health_check_duration).await;
        }

        self
    }

    pub async fn check_healthy_connection_pools(&mut self) -> HashSet<usize> {
        let mut unhealthy_indexes = Vec::new();

        for (idx, connection) in self
            .healthy_health_check_connection_pool
            .iter_mut()
            .enumerate()
        {
            connection.run_check_health(&self.path, self.timeout).await;

            if !connection.is_healthy() {
                unhealthy_indexes.push(idx);
            }
        }

        unhealthy_indexes.reverse();

        let mut source_connection_pool_guard = self.source_connection_pool.write().await;

        let

        for idx in unhealthy_indexes.iter() {
            let connection = source_connection_pool_guard.remove(*idx);
            self.unhealthy_connection_pool.push(connection);

            let health_check_target = self.healthy_health_check_connection_pool.remove(*idx);
            self.unhealthy_health_check_connection_pool
                .push(health_check_target);
        }

        unhealthy_indexes.into_iter().collect()
    }

    pub async fn check_unhealthy_connection_pools(&mut self, recently_unhealthy: HashSet<usize>) {
        let mut healthy_indexes = Vec::new();

        for (idx, connection) in self
            .unhealthy_health_check_connection_pool
            .iter_mut()
            .enumerate()
        {
            if

            connection.run_check_health(&self.path, self.timeout).await;

            if !connection.is_healthy() {
                healthy_indexes.push(idx);
            }
        }

        healthy_indexes.reverse();

        let mut source_connection_pool_guard = self.source_connection_pool.write().await;

        for idx in healthy_indexes.iter() {
            let connection = source_connection_pool_guard.remove(*idx);
            self.unhealthy_connection_pool.push(connection);

            let health_check_target = self.healthy_health_check_connection_pool.remove(*idx);
            self.unhealthy_health_check_connection_pool
                .push(health_check_target);
        }
    }

    // pub async fn check_unhealthy_connection_pools(&mut self) {
    //     assert!(self.unhealthy_connection_pool.len() == self.unhealthy_stats.len());

    //     for (idx, connection) in self.unhealthy_connection_pool.iter().enumerate() {
    //         if check_health(connection, &self.path, self.timeout).await {
    //             self.unhealthy_stats[idx] += 1;
    //         } else {
    //             self.unhealthy_stats[idx] == 0;
    //         }
    //     }
    // }

    // pub async fn filter_healthy_connection_pools(&mut self) {
    //     let mut failed_indexes = self
    //         .healthy_stats
    //         .iter()
    //         .filter(|c| *c >= &self.failure_threshold)
    //         .enumerate()
    //         .map(|(i, _)| i)
    //         .collect::<Vec<_>>();

    //     failed_indexes.reverse();

    //     assert!(self.health_check_connection_pool.len() == self.healthy_stats.len());

    //     for idx in failed_indexes {
    //         let pool = self.health_check_connection_pool.remove(idx);
    //         self.healthy_stats.remove(idx);

    //         self.unhealthy_connection_pool.push(pool);
    //         self.unhealthy_stats.push(0);
    //     }
    // }

    // pub async fn filter_unhealthy_connection_pools(&mut self) {
    //     let mut succeeded_indexes = self
    //         .unhealthy_stats
    //         .iter()
    //         .filter(|c| *c >= &self.success_threshold)
    //         .enumerate()
    //         .map(|(i, _)| i)
    //         .collect::<Vec<_>>();

    //     succeeded_indexes.reverse();

    //     assert!(self.unhealthy_connection_pool.len() == self.unhealthy_stats.len());

    //     for idx in succeeded_indexes {
    //         let pool = self.unhealthy_connection_pool.remove(idx);
    //         self.unhealthy_stats.remove(idx);

    //         self.health_check_connection_pool.push(pool);
    //         self.healthy_stats.push(0);
    //     }
    // }
}
