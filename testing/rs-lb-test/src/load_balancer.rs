use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};

use http::StatusCode;
use http::{Request, Response, Uri, uri::PathAndQuery};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{body::Bytes, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use log::{debug, error};
use tokio::sync::RwLock;
use tokio::{net::TcpStream, select, time::sleep};

use crate::cache::RequestCache;
use crate::{
    connection_pool::{TargetConnectionPool, TargetGroupsConnectionPools},
    listener::ListenerRule,
    selector::RoundRobin,
};

pub struct LoadBalancer {
    pub listener_targets: HashMap<String, ListenerRuleHandler>,
    pub prefixes: Vec<String>,
    pub cache: Option<Arc<RequestCache>>,
}

impl LoadBalancer {
    pub async fn new(
        listener_rules: Vec<ListenerRule>,
        connection_pools: &TargetGroupsConnectionPools<Incoming>,
        connection_timeout: Duration,
    ) -> Self {
        let mut prefixes: Vec<String> = listener_rules
            .iter()
            .map(|r| format!("{}/", r.path_prefix.trim_end_matches("/")))
            .collect();

        prefixes.sort();
        prefixes.reverse();

        let listener_targets = listener_rules
            .into_iter()
            .map(|r| {
                (
                    format!("{}/", r.path_prefix.trim_end_matches("/")),
                    ListenerRuleHandler {
                        selector: RoundRobin::new(),
                        connection_pool: connection_pools
                            .get_pool_for_group(&r.target_group)
                            .expect("Missing target group"),
                        path_rewrite: r.path_rewrite,
                        connection_timeout,
                    },
                )
            })
            .collect();

        Self {
            listener_targets,
            prefixes,
            cache: Option::None,
        }
    }

    pub fn with_cache(self, ttl: Duration) -> Self {
        let Self {
            listener_targets,
            prefixes,
            ..
        } = self;
        Self {
            listener_targets,
            prefixes,
            cache: Some(RequestCache::new(ttl)),
        }
    }

    pub async fn serve_connection(self: Arc<Self>, conn: TokioIo<TcpStream>) {
        if let Err(err) = http1::Builder::new()
            .keep_alive(true)
            .serve_connection(conn, service_fn(|request| self.handle_connection(request)))
            .await
        {
            error!("Error serving connection: {:?}", err);
        }
    }

    pub fn match_uri(&self, uri: &str) -> Option<&str> {
        self.prefixes
            .iter()
            .find(|r| uri.starts_with(*r))
            .map(String::as_ref)
    }

    pub async fn handle_connection(
        &self,
        request: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>, Infallible> {
        debug!("Handling Connection: {:?}", request);

        let uri = request.uri().to_string();

        if let Some(response) = self.cache.as_ref().and_then(|c| c.get(&uri)) {
            return Ok(response);
        }

        let response = match self.match_uri(request.uri().path()) {
            None => Response::builder()
                .status(http::StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::new()))
                .unwrap(),
            Some(prefix) => {
                self.listener_targets
                    .get(prefix)
                    .expect("Failed to get listener target")
                    .handle_connection(request)
                    .await?
            }
        };

        if let Some(cache) = &self.cache {
            cache.set(&uri, &response);
        }

        Ok(response)
    }
}

pub struct ListenerRuleHandler {
    pub selector: RoundRobin,
    pub connection_pool: Arc<RwLock<Vec<TargetConnectionPool<Incoming>>>>,
    pub path_rewrite: String,
    pub connection_timeout: Duration,
}

impl ListenerRuleHandler {
    pub async fn handle_connection(
        &self,
        request: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>, Infallible> {
        let connection_pool_guard = self.connection_pool.read().await;

        if connection_pool_guard.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Full::new(Bytes::new()))
                .unwrap());
        }

        let selection = self.selector.next_wrapping(connection_pool_guard.len());

        let (mut target, uri) = match connection_pool_guard.get(selection) {
            None => panic!("Cannot find connection"),
            Some(c) => match c.connection_pool.get().await {
                Ok(p) => (p, c.uri.clone()),
                Err(e) => {
                    log::error!("Failed to get pooled connection: {}", e);

                    return Ok(Response::builder()
                        .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Full::new(Bytes::new()))
                        .unwrap());
                }
            },
        };

        let path_and_query = request.uri().path_and_query().unwrap();

        let sanitised_path_and_query = path_and_query
            .path()
            .strip_prefix(&self.path_rewrite)
            .expect("Failed to strip prefix for matched path. This should not happen.")
            .trim_start_matches("/");

        let rewritten_path = if uri.is_empty() {
            format!("/{}", sanitised_path_and_query)
        } else {
            format!("/{}/{}", uri, sanitised_path_and_query)
        };

        let rewritten_path_and_query = match path_and_query.query() {
            None => PathAndQuery::try_from(rewritten_path).unwrap(),
            Some(query) => PathAndQuery::try_from(format!("{}?{}", rewritten_path, query)).unwrap(),
        };

        let mut uri_builder = Uri::builder().path_and_query(rewritten_path_and_query);

        if let Some(authority) = request.uri().authority() {
            uri_builder = uri_builder.authority(authority.as_str());
        }

        if let Some(scheme) = request.uri().scheme() {
            uri_builder = uri_builder.scheme(scheme.as_str());
        }

        let uri = uri_builder.build().expect("Failed to build uri");

        let client_request = request
            .headers()
            .iter()
            .fold(
                Request::builder()
                    .uri(uri)
                    .method(request.method())
                    .version(request.version()),
                |b, (k, v)| b.header(k, v),
            )
            .body(request.into_body())
            .unwrap();

        target
            .ready()
            .await
            .expect("Failed to wait for ready connection");

        select! {
          response_result = target.send_request(client_request) => {
            let response = response_result.expect("Failed to send request");

            let (parts, incoming_body) = response.into_parts();

            let body = incoming_body
                .collect()
                .await
                .expect("Failed to get body")
                .to_bytes();

            let response = Response::from_parts(parts, Full::new(body));

            Ok(response)
          },
          _ = sleep(self.connection_timeout) => {
            Ok(Response::builder()
                .status(http::StatusCode::GATEWAY_TIMEOUT)
                .body(Full::new(Bytes::new()))
                .unwrap())
          }
        }
    }
}
