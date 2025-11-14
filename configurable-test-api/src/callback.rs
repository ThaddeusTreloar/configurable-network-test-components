use std::time::Duration;

use axum::routing::{MethodRouter, connect, delete, get, head, options, patch, post, put, trace};
use shared::Method;
use tokio::time::sleep;

#[derive(Debug, thiserror::Error)]
pub(crate) enum MakeCallbackError {}

pub fn make_callback<S>(method: &Method, latency: u64) -> Result<MethodRouter<S>, MakeCallbackError>
where
    S: Clone + Send + Sync + 'static,
{
    let callback = match method {
        Method::Options => options(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        Method::Post => post(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        Method::Put => put(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        Method::Delete => delete(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        Method::Head => head(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        Method::Trace => trace(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        Method::Connect => connect(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        Method::Patch => patch(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        Method::Get => get(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
    };

    Ok(callback)
}
