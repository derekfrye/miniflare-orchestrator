use super::LeaseBackend;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Create a reusable isolated lease.
///
/// The lease remains available for bundling and restarting until deleted.
/// `env_vars` are injected only into this lease worker process. `backend`
/// defaults to `miniflare` and may be set to `wrangler_dev` per lease.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseCreateRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub health_path: Option<String>,
    #[serde(default)]
    pub env: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub log_level: Option<String>,
    #[serde(default)]
    pub env_vars: BTreeMap<String, String>,
    #[serde(default = "default_true")]
    pub persist_state: bool,
    #[serde(default)]
    pub backend: LeaseBackend,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
pub struct LeaseRestartRequest {
    #[serde(default)]
    pub persist_state: Option<bool>,
    #[serde(default)]
    pub backend: Option<LeaseBackend>,
}

#[must_use]
pub fn default_true() -> bool {
    true
}
