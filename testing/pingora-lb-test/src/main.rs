use std::{collections::HashMap, sync::Arc};

use http::{uri::PathAndQuery, Uri};
use pingora::{
    http::RequestHeader,
    lb::{selection::SelectionAlgorithm, LoadBalancer},
    prelude::{HttpPeer, RoundRobin},
    proxy::{http_proxy_service, ProxyHttp, Session},
    server::Server,
};

pub struct Selector {}

impl SelectionAlgorithm for Selector {
    fn new() -> Self {
        Self {}
    }

    fn next(&self, _key: &[u8]) -> u64 {
        0
    }
}

pub struct Router {
    // routes: Arc<HashMap<String, LoadBalancer<RoundRobin>>>,
}

pub struct TestLoadBalancer {
    routes: Arc<HashMap<String, LoadBalancer<RoundRobin>>>,
}

#[async_trait::async_trait]
impl ProxyHttp for TestLoadBalancer {
    type CTX = Option<String>;

    fn new_ctx(&self) -> Self::CTX {
        Option::None
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<(), Box<pingora::Error>> {
        let mut path = upstream_request.uri.path().to_owned();

        if let Some(query) = upstream_request.uri.query() {
            path = format!("{path}?{query}")
        }

        match ctx {
            None => panic!("No matched path"),
            Some(matched_patch) => {
                let rewritten_path = path.strip_prefix(matched_patch.as_str());

                upstream_request.set_uri(
                    Uri::builder()
                        .path_and_query(PathAndQuery::try_from(rewritten_path.unwrap()).unwrap())
                        .build()
                        .unwrap(),
                );

                Ok(())
            }
        }
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let path = session.req_header().uri.path();

        let matched_route = self.routes.keys().find_map(|k| {
            if path.starts_with(k) {
                Some((k, self.routes.get(k).unwrap()))
            } else {
                None
            }
        });

        match matched_route {
            None => Err(pingora::Error::new(pingora::ErrorType::HTTPStatus(404))),
            Some((path, selector)) => {
                let selection = selector
                    .select(b"", 256)
                    .expect("Failed to select downstream");

                ctx.replace(path.clone());

                let peer = Box::new(HttpPeer::new(selection, false, "none".to_string()));

                Ok(peer)
            }
        }
    }
}

fn main() {
    let mut server = Server::new(None).unwrap();

    let mut routes = HashMap::default();

    routes.insert(
        "/s".to_owned(),
        LoadBalancer::try_from_iter(["api:8081"]).unwrap(),
    );

    let lb_inst = TestLoadBalancer {
        routes: Arc::new(routes),
    };

    let mut lb = http_proxy_service(&server.configuration, lb_inst);
    lb.add_tcp("0.0.0.0:8080");

    server.add_service(lb);
    server.bootstrap();
    server.run_forever();
}
