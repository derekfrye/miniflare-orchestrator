use super::{
    LeaseBackend, LeaseBundleDiagnostic, LeaseBundleMetadata, LeaseHealthProbeReport,
    LeaseLaunchDetails, LeasePrebuiltBundleNotice, LeaseStartupDiagnostics, LeaseStatus,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseResponse {
    pub id: String,
    pub name: String,
    pub port: u16,
    pub inspector_port: u16,
    pub base_url: String,
    pub health_url: String,
    pub status: LeaseStatus,
    pub runtime_dir: String,
    pub static_dir: String,
    pub state_dir: String,
    pub log_dir: String,
    pub health_path: String,
    pub env: String,
    pub protocol: String,
    pub log_level: String,
    pub env_vars: BTreeMap<String, String>,
    pub persist_state: bool,
    pub backend: LeaseBackend,
    pub prebuilt_bundle_notice: LeasePrebuiltBundleNotice,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_metadata: Option<LeaseBundleMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_uploaded_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bundle_diagnostics: Vec<LeaseBundleDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_diagnostics: Option<LeaseStartupDiagnostics>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseDebugResponse {
    pub lease: LeaseResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup: Option<LeaseLaunchDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_probe: Option<LeaseHealthProbeReport>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseFailureReport {
    pub lease: LeaseResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup: Option<LeaseLaunchDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_probe: Option<LeaseHealthProbeReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_tail: Option<String>,
}
