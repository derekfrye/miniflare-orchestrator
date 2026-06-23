use super::LeaseBackend;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeaseStartupDiagnosticKind {
    RedirectedToHttps,
    WorkerExitedEarly,
    HealthCheckTimedOut,
    ProbeError,
    PossiblyStaleBundle,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseStartupDiagnostics {
    pub kind: LeaseStartupDiagnosticKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_protocol: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct LeaseEffectiveBindings {
    pub backend: LeaseBackend,
    pub env: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vars: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kv_namespaces: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub r2_buckets: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub durable_objects: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseLaunchDetails {
    pub backend: LeaseBackend,
    pub worker_bin: String,
    pub wrangler_bin: String,
    pub config_file: String,
    pub runtime_dir: String,
    pub static_dir: String,
    pub state_dir: String,
    pub log_dir: String,
    pub port: u16,
    pub inspector_port: u16,
    pub env: String,
    pub protocol: String,
    pub log_level: String,
    pub persist_state: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env_vars: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub injected_env: BTreeMap<String, String>,
    #[serde(default)]
    pub effective_bindings: LeaseEffectiveBindings,
}
