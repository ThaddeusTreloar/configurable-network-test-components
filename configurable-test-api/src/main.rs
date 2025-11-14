use axum::Router;
use figment::{Figment, providers::Env};
use log::info;

use crate::{
    callback::make_callback,
    config::{AppConfig, RouteConfig},
};

mod callback;
mod config;

#[tokio::main]
async fn main() {
    env_logger::init();

    let AppConfig { port, routes } = match Figment::new()
        .merge(Env::prefixed("APP_").split("_"))
        .extract()
    {
        Ok(cfg) => cfg,
        Err(e) => {
            log::error!("Error while parsing config: {e}");

            return;
        }
    };

    let listener = match tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", port))
        .await
        .map_err(Box::new)
    {
        Ok(listener) => listener,
        Err(e) => {
            log::error!("Error while creating listener: {e}");
            return;
        }
    };

    let mut app = Router::new();

    for (_, route) in routes.into_iter() {
        info!("Using route: {route}");
        let RouteConfig {
            path,
            method,
            latency,
        } = route;

        let callback = match make_callback(&method, latency).map_err(Box::new) {
            Ok(callback) => callback,
            Err(e) => {
                log::error!("Error while building route callback: {e}");
                return;
            }
        };

        app = app.route(&path, callback);
    }

    match axum::serve(listener, app).await {
        Ok(_) => (),
        Err(e) => {
            log::error!("Error while parsing config: {e}");
            return;
        }
    };
}
