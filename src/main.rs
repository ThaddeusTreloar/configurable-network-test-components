use axum::Router;
use log::info;

use crate::{
    callback::make_callback,
    config::{AppConfig, RouteConfig},
};

mod callback;
mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let AppConfig { port, routes } = AppConfig::try_from_env().map_err(Box::new)?;

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", port))
        .await
        .map_err(Box::new)?;

    let mut app = Router::new();

    for route in routes.into_iter() {
        info!("Using route: {route}");
        let RouteConfig {
            path,
            method,
            latency,
        } = route;
        app = app.route(&path, make_callback(&method, latency).map_err(Box::new)?);
    }

    axum::serve(listener, app).await?;

    Ok(())
}
