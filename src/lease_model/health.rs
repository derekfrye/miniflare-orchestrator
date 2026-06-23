use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeaseHealthProbeOutcome {
    Healthy,
    Unhealthy,
    RedirectedToHttps,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseHealthProbeReport {
    pub request_url: String,
    pub request_method: String,
    pub protocol: String,
    pub health_path: String,
    pub outcome: LeaseHealthProbeOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
