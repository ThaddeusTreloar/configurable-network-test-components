use std::{collections::HashMap, fmt::Display};

use serde::Deserialize;
use shared::Method;

fn default_statistics_interval() -> u64 {
    1000
}

#[derive(Debug, Deserialize)]
pub(crate) struct AppConfig {
    #[serde(default = "default_statistics_interval")]
    pub statistics_interval: u64,
    pub targets: HashMap<String, TargetConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) enum RampStrategy {
    Step,
}

impl Display for RampStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Step => "STEP",
        })
    }
}

fn default_method() -> Method {
    Method::Get
}

fn default_client_timeout() -> u64 {
    5000
}

fn default_client_error_threshold() -> usize {
    10
}

fn default_client_error_threshold_window() -> u64 {
    1000
}

fn default_client_count_start() -> usize {
    10
}
fn default_client_count_ramp() -> usize {
    10
}
fn default_client_count_ramp_interval() -> u64 {
    1000
}
fn default_client_count_ramp_strategy() -> RampStrategy {
    RampStrategy::Step
}
fn default_client_wait_start() -> u64 {
    250
}
fn default_client_wait_decay() -> usize {
    10
}
fn default_client_wait_decay_interval() -> u64 {
    1000
}
fn default_client_wait_jitter() -> u64 {
    50
}
fn default_client_wait_decay_strategy() -> RampStrategy {
    RampStrategy::Step
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TargetConfig {
    pub target: String,
    #[serde(default = "default_method")]
    pub method: Method,
    #[serde(default = "default_client_timeout")]
    pub client_timeout: u64,
    #[serde(default = "default_client_error_threshold")]
    pub client_error_threshold: usize,
    #[serde(default = "default_client_error_threshold_window")]
    pub client_error_threshold_window: u64,
    #[serde(default = "default_client_count_start")]
    pub client_count_start: usize,
    #[serde(default = "default_client_count_ramp")]
    pub client_count_ramp: usize,
    #[serde(default = "default_client_count_ramp_interval")]
    pub client_count_ramp_interval: u64,
    #[serde(default = "default_client_count_ramp_strategy")]
    pub client_count_ramp_strategy: RampStrategy,
    #[serde(default = "default_client_wait_start")]
    pub client_wait_start: u64,
    #[serde(default = "default_client_wait_decay")]
    pub client_wait_decay: usize,
    #[serde(default = "default_client_wait_decay_interval")]
    pub client_wait_decay_interval: u64,
    #[serde(default = "default_client_wait_jitter")]
    pub client_wait_jitter: u64,
    #[serde(default = "default_client_wait_decay_strategy")]
    pub client_wait_decay_strategy: RampStrategy,
}

impl Display for TargetConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "{{target: \"{}\", method: \"{}\", client_count_start: \"{}\", client_count_ramp: \"{}\", client_count_ramp_interval: \"{}\", client_count_ramp_strategy: \"{}\", client_wait: \"{}\", client_wait_decay: \"{}\", client_wait_decay_interval: \"{}\", client_wait_decay_strategy: \"{}\"}}",
                self.target, self.method,
                self.client_count_start,
                self.client_count_ramp,
                self.client_count_ramp_interval,
                self.client_count_ramp_strategy,
                self.client_wait_start,
                self.client_wait_decay,
                self.client_wait_decay_interval,
                self.client_wait_decay_strategy,
            )
            .as_str(),
        )
    }
}
