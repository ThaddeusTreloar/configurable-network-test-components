use std::time::Duration;

use axum::routing::{MethodRouter, connect, delete, get, head, options, patch, post, put, trace};
use tokio::time::sleep;

#[derive(Debug, thiserror::Error)]
pub(crate) enum MakeCallbackError {
    #[error("Invalid method type: {}", 0)]
    InvalidMethod(String),
}

pub fn make_callback<S>(method: &str, latency: u64) -> Result<MethodRouter<S>, MakeCallbackError>
where
    S: Clone + Send + Sync + 'static,
{
    let callback = match method {
        "OPTIONS" => options(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        "POST" => post(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        "PUT" => put(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        "DELETE" => delete(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        "HEAD" => head(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        "TRACE" => trace(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        "CONNECT" => connect(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        "PATCH" => patch(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        "GET" => get(async move || {
            sleep(Duration::from_millis(latency)).await;
            "hello"
        }),
        _ => Err(MakeCallbackError::InvalidMethod(method.to_string()))?,
    };

    Ok(callback)
}
