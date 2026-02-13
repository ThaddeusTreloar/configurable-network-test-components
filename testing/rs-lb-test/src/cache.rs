use std::{sync::Arc, time::Duration};

use dashmap::DashMap;
use http::Response;
use http_body_util::Full;
use hyper::body::Bytes;
use tokio::{
    spawn,
    time::{Instant, sleep},
};

#[derive()]
pub struct RequestCache {
    inner: DashMap<String, CachedResponse>,
    ttl: Duration,
}

impl RequestCache {
    pub fn new(ttl: Duration) -> Arc<Self> {
        let self_arc = Arc::new(Self {
            inner: Default::default(),
            ttl,
        });

        spawn(self_arc.clone().cleanup_thread());

        self_arc
    }

    pub fn get(&self, key: &str) -> Option<Response<Full<Bytes>>> {
        self.inner.get(key).map(|e| e.inner.clone())
    }

    pub fn set(&self, key: &str, request: &Response<Full<Bytes>>) {
        self.inner
            .insert(key.to_owned(), CachedResponse::new(request.clone()));
    }

    async fn cleanup_thread(self: Arc<Self>) {
        loop {
            sleep(self.ttl).await;

            self.inner.retain(|_, v| !v.is_expired(self.ttl));
        }
    }
}

pub struct CachedResponse {
    pub inner: Response<Full<Bytes>>,
    pub set_time: Instant,
}

impl CachedResponse {
    pub fn new(response: Response<Full<Bytes>>) -> Self {
        Self {
            inner: response,
            set_time: Instant::now(),
        }
    }

    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.set_time.elapsed() > ttl
    }
}
