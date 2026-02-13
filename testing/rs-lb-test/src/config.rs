use std::{collections::HashMap, fmt::Display};

use serde::Deserialize;

fn default_listener_port() -> u16 {
    8080
}

fn default_connection_timeout() -> u64 {
    60000
}

fn default_load_balancing_algorithm() -> LoadBalancingAlgorithm {
    LoadBalancingAlgorithm::RoundRobin
}

fn default_connection_pool_size() -> u32 {
    1024
}

fn default_cache_enabled() -> bool {
    false
}

fn default_cache_ttl() -> u64 {
    10000
}

#[derive(Debug, Deserialize)]
pub(crate) struct LoadBalancerConfiguration {
    #[serde(default = "default_listener_port")]
    pub listener_port: u16,
    #[serde(default = "default_connection_timeout")]
    pub connection_timout: u64,
    #[serde(default = "default_load_balancing_algorithm")]
    pub load_balancing_algorithm: LoadBalancingAlgorithm,
    #[serde(default = "default_connection_pool_size")]
    pub connection_pool_size: u32,
    #[serde(default = "default_cache_enabled")]
    pub cache_enabled: bool,
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl: u64,
    pub listener_rules: HashMap<String, ListenerRuleConfiguration>,
    pub target_groups: HashMap<String, TargetGroupConfiguration>,
}

impl Display for LoadBalancerConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "LoadBalancerConfiguration{{\n\tlistener_port={},\n\tconnection_timout={},\n\tload_balancing_algorithm={},\n\tconnection_pool_size={},\n\tcache_enabled={},\n",
                self.listener_port,
                self.connection_timout,
                self.load_balancing_algorithm,
                self.connection_pool_size,
                self.cache_enabled
            )
            .as_ref(),
        )?;

        self.listener_rules.iter().try_for_each(|(name, rule)| {
            f.write_str(format!("\tlistener_rules.{}={}\n", name, rule).as_str())
        })?;

        self.target_groups.iter().try_for_each(|(name, group)| {
            f.write_str(format!("\ttarget_groups.{}={}\n", name, group).as_str())
        })?;

        f.write_str("}}")
    }
}

#[derive(Debug, Deserialize)]
pub enum LoadBalancingAlgorithm {
    #[serde(alias = "ROUND_ROBIN")]
    RoundRobin,
}

impl Display for LoadBalancingAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variant = match self {
            Self::RoundRobin => "ROUND_ROBIN",
        };

        f.write_str(format!("LoadBalancingAlgorithm::{}", variant).as_ref())
    }
}

fn default_path_rewrite() -> String {
    "".to_owned()
}

#[derive(Debug, Deserialize)]
pub struct ListenerRuleConfiguration {
    pub target_group: String,
    pub path_prefix: String,
    #[serde(default = "default_path_rewrite")]
    pub path_rewrite: String,
}

impl Display for ListenerRuleConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "ListenerRuleConfiguration{{target_group={}, path_prefix={}, path_rewrite={}}}",
                self.target_group, self.path_prefix, self.path_rewrite
            )
            .as_ref(),
        )
    }
}

fn default_health_check() -> TargetGroupHealthCheckConfiguration {
    Default::default()
}

#[derive(Debug, Deserialize)]
pub struct TargetGroupConfiguration {
    pub targets: String,
    #[serde(default = "default_health_check")]
    pub health_check: TargetGroupHealthCheckConfiguration,
}

impl Display for TargetGroupConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("TargetGroupConfiguration{{targets={}}}", self.targets).as_ref())
    }
}

fn default_enable() -> bool {
    false
}
fn default_timeout() -> u64 {
    10000
}
fn default_interval() -> u64 {
    60000
}
fn default_success_threshold() -> usize {
    5
}
fn default_failure_threshold() -> usize {
    3
}

#[derive(Debug, Deserialize, Clone)]
pub struct TargetGroupHealthCheckConfiguration {
    pub path: String,
    #[serde(default = "default_enable")]
    pub enabled: bool,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_interval")]
    pub interval: u64,
    #[serde(default = "default_success_threshold")]
    pub success_threshold: usize,
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: usize,
}

impl Default for TargetGroupHealthCheckConfiguration {
    fn default() -> Self {
        Self {
            path: String::new(),
            enabled: default_enable(),
            timeout: default_timeout(),
            interval: default_interval(),
            success_threshold: default_success_threshold(),
            failure_threshold: default_failure_threshold(),
        }
    }
}
