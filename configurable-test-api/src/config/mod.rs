use std::{collections::HashMap, fmt::Display};

use serde::Deserialize;
use shared::Method;

fn default_port() -> u16 {
    8080
}

#[derive(Debug, Deserialize)]
pub(crate) struct AppConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    pub routes: HashMap<String, RouteConfig>,
}

fn default_method() -> Method {
    Method::Get
}

#[derive(Debug, Deserialize)]
pub(crate) struct RouteConfig {
    pub path: String,
    #[serde(default = "default_method")]
    pub method: Method,
    #[serde(default)]
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
