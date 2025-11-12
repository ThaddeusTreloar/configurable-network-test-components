use std::{env::Vars, fmt::Display};

use itertools::Itertools;
use log::{info, warn};

#[derive(Debug, thiserror::Error)]
pub(crate) enum AppConfigParseError {
    #[error("Failed to parse server port due to error: {}", 0)]
    InvalidLatencyValue(#[from] std::num::ParseIntError),
    #[error(transparent)]
    InvalidRoute(#[from] RouteConfigParseError),
    #[error("No routes for server")]
    EmptyRoutes,
}

pub(crate) struct AppConfig {
    pub port: u16,
    pub routes: Vec<RouteConfig>,
}

impl AppConfig {
    pub fn try_from_env() -> Result<Self, AppConfigParseError> {
        std::env::vars().try_into()
    }
}

impl TryFrom<Vars> for AppConfig {
    type Error = AppConfigParseError;

    fn try_from(vars: Vars) -> Result<Self, Self::Error> {
        let owned_vars = vars.collect::<Vec<_>>();

        let port = owned_vars
            .iter()
            .find(|(k, _)| *k == "PORT")
            .map(|(_, v)| v.parse())
            .unwrap_or(Ok(8080))?;

        info!("Getting route configurations...");

        let routes: Vec<RouteConfig> = owned_vars
            .into_iter()
            .sorted()
            .filter(|(k, _)| k.starts_with("ROUTE_"))
            .map(|(k, v)| (k.trim_start_matches("ROUTE_").to_owned(), v))
            .map(|(k, v)| (k.split("_").map(String::from).collect::<Vec<_>>(), v))
            .filter(|(k, _)| k.len() == 2)
            .map(|(k, v)| (k[0].clone(), k[1].clone(), v))
            .chunk_by(|(r, _, _)| r.to_owned())
            .into_iter()
            .map(|(r, c)| (r, c.collect::<Vec<_>>()))
            .map(|(_, configs)| RouteConfig::try_from(configs))
            .collect::<Result<Vec<_>, RouteConfigParseError>>()?;

        if routes.is_empty() {
            Err(AppConfigParseError::EmptyRoutes)?;
        }

        Ok(Self { port, routes })
    }
}

pub(crate) struct RouteConfig {
    pub path: String,
    pub method: String,
    pub latency: u64,
}

impl Display for RouteConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "{{path: \"{}\", method: \"{}\", latency: \"{}\"}}",
                self.path, self.method, self.latency,
            )
            .as_str(),
        )
    }
}

impl RouteConfig {
    pub fn new(path: &str, method: &str, latency: u64) -> Self {
        Self {
            path: path.to_string(),
            method: method.to_string(),
            latency,
        }
    }
}

impl Default for RouteConfig {
    fn default() -> Self {
        Self::new("/", "GET", 0)
    }
}

impl TryFrom<Vec<(String, String, String)>> for RouteConfig {
    type Error = RouteConfigParseError;

    fn try_from(vars: Vec<(String, String, String)>) -> Result<Self, Self::Error> {
        let mut route = String::new();
        let mut path: Option<String> = Option::None;
        let mut method: String = "GET".to_owned();
        let mut latency: u64 = 0;

        for (r, k, v) in vars.into_iter() {
            route = r;
            match k.as_str() {
                "PATH" => path = Some(v),
                "METHOD" => method = v.to_ascii_uppercase(),
                "LATENCY" => {
                    latency = v.parse().map_err(|e| {
                        RouteConfigParseError::InvalidLatencyValue((route.clone(), e))
                    })?
                }
                unknown => warn!(
                    "Unrecognised route configuration: {unknown}, for route: {route}, skipping..."
                ),
            }
        }

        match method.as_str() {
            "OPTIONS" | "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "TRACE" | "CONNECT"
            | "PATCH" => Ok(Self {
                path: path.ok_or(RouteConfigParseError::MissingPath(route))?,
                method,
                latency,
            }),
            _ => Err(RouteConfigParseError::InvalidMethod((method, route))),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RouteConfigParseError {
    #[error("Failed to parse latency for route: {}, due to error: {}", 0, 1)]
    InvalidLatencyValue((String, std::num::ParseIntError)),
    #[error("Missing path for route: {}", 0)]
    MissingPath(String),
    #[error("Invalid method: {} for route: {}", 0, 1)]
    InvalidMethod((String, String)),
}
