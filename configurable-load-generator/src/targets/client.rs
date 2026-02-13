use std::{
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};

use futures::TryFutureExt;
use log::trace;
use reqwest::Client;
use shared::Method;
use tokio::time::{Instant, sleep};

use crate::{
    config::{RampStrategy, TargetConfig},
    stats::TargetStatistics,
    targets::error::ClientTargetError,
};

#[derive()]
pub(crate) struct ClientTarget {
    pub client: Client,
    pub target: String,
    pub method: Method,
    pub current_wait: Duration,
    pub _wait_decay: usize,
    pub _wait_decay_interval: Duration,
    pub _wait_decay_strategy: RampStrategy,
    pub _wait_jitter: Duration,
    pub request_statistics: Arc<AtomicUsize>,
    pub response_time_acc: Arc<AtomicUsize>,
}

impl ClientTarget {
    pub(crate) fn new(target_config: &TargetConfig, statistics: &TargetStatistics) -> Self {
        trace!(
            "Creating client with target: {}, method: {}",
            target_config.target, target_config.method
        );

        Self {
            client: Client::builder()
                .timeout(Duration::from_millis(target_config.client_timeout))
                .tcp_keepalive(Some(Duration::from_millis(10000)))
                .build()
                .expect("Failed to create client"),
            target: target_config.target.clone(),
            method: target_config.method,
            current_wait: Duration::from_millis(target_config.client_wait_start),
            _wait_decay: target_config.client_wait_decay,
            _wait_decay_interval: Duration::from_millis(target_config.client_wait_decay_interval),
            _wait_decay_strategy: target_config.client_wait_decay_strategy.clone(),
            _wait_jitter: Duration::from_millis(target_config.client_wait_jitter),
            request_statistics: statistics.requests.clone(),
            response_time_acc: statistics.response_time_acc.clone(),
        }
    }
}

impl ClientTarget {
    pub(crate) async fn run_client(self) -> Result<(), ClientTargetError> {
        loop {
            let request_start_time = Instant::now();

            match self.method {
                Method::Options => self.handle_options(request_start_time).await?,
                Method::Get => self.handle_get(request_start_time).await?,
                Method::Post => self.handle_post(request_start_time).await?,
                Method::Put => self.handle_put(request_start_time).await?,
                Method::Delete => self.handle_delete(request_start_time).await?,
                Method::Head => self.handle_head(request_start_time).await?,
                Method::Trace => self.handle_trace(request_start_time).await?,
                Method::Connect => self.handle_connect(request_start_time).await?,
                Method::Patch => self.handle_patch(request_start_time).await?,
            }

            sleep(self.current_wait).await
        }
    }

    async fn handle_options(&self, _request_start_time: Instant) -> Result<(), ClientTargetError> {
        unimplemented!("handle_options");
    }

    async fn handle_get(&self, request_start_time: Instant) -> Result<(), ClientTargetError> {
        match self.client.get(self.target.as_str()).send().await {
            Ok(r) => {
                let status = r.status();
                let bytes = r
                    .bytes()
                    .map_err(|e| ClientTargetError::RequestFailure {
                        status: e
                            .status()
                            .map(|s| s.as_str().to_owned())
                            .unwrap_or("None".to_owned()),
                        timeout: e.is_timeout(),
                        request: e.is_request(),
                        connection: e.is_connect(),
                    })
                    .await?;
                trace!("Recieved response code: {status}");
                self.request_statistics
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.response_time_acc.fetch_add(
                    request_start_time.elapsed().as_millis() as usize,
                    std::sync::atomic::Ordering::Relaxed,
                );
            }
            Err(e) => {
                eprintln!("{}", e);

                return Err(ClientTargetError::RequestFailure {
                    status: e
                        .status()
                        .map(|s| s.as_str().to_owned())
                        .unwrap_or("None".to_owned()),
                    timeout: e.is_timeout(),
                    request: e.is_request(),
                    connection: e.is_connect(),
                });
            }
        };

        Ok(())
    }

    async fn handle_post(&self, _request_start_time: Instant) -> Result<(), ClientTargetError> {
        unimplemented!("handle_post");
    }

    async fn handle_put(&self, _request_start_time: Instant) -> Result<(), ClientTargetError> {
        unimplemented!("handle_put");
    }

    async fn handle_delete(&self, _request_start_time: Instant) -> Result<(), ClientTargetError> {
        unimplemented!("handle_delete");
    }

    async fn handle_head(&self, _request_start_time: Instant) -> Result<(), ClientTargetError> {
        unimplemented!("handle_head");
    }

    async fn handle_trace(&self, _request_start_time: Instant) -> Result<(), ClientTargetError> {
        unimplemented!("handle_trace");
    }

    async fn handle_connect(&self, _request_start_time: Instant) -> Result<(), ClientTargetError> {
        unimplemented!("handle_connect");
    }

    async fn handle_patch(&self, _request_start_time: Instant) -> Result<(), ClientTargetError> {
        unimplemented!("handle_patch");
    }
}
