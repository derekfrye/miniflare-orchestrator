use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PREBUILT_BUNDLE_NOTICE: &str = "Lease backends serve the uploaded prebuilt bundle only. Wrangler build hooks are skipped because workers run with prebuilt artifacts; rebuild worker artifacts and POST /leases/{id}/bundle again after changing Worker source.";

/// Upload a complete prebuilt replacement bundle for one isolated lease.
///
/// `runtime_files` are written beneath the runtime directory and
/// `static_files` beneath the static directory. File paths must be relative
/// and path-traversal segments are rejected. Runtime host backends consume
/// prebuilt bundles, so Wrangler build hooks such as `[build].command` are
/// skipped and callers must upload already-built artifacts. `runtime_files`
/// should include at least `wrangler.toml`, `worker_entry.mjs` or another
/// configured main module, and any built JS/Wasm files imported by the
/// entrypoint.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseBundleRequest {
    pub runtime_files: Vec<LeaseFile>,
    #[serde(default)]
    pub static_files: Vec<LeaseFile>,
    /// Optional caller-provided provenance for the prebuilt bundle.
    ///
    /// This metadata is returned by lease inspection and debug endpoints so
    /// test harnesses and LLM agents can see exactly which already-built
    /// artifact the lease is serving. When `source_root` and `source_paths`
    /// are provided, the host can warn if those source files are newer than
    /// the last uploaded bundle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<LeaseBundleMetadata>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseFile {
    pub path: String,
    pub content_b64: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseBundleMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeasePrebuiltBundleNotice {
    pub message: String,
    pub required_action: String,
    pub applies_to: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeaseBundleDiagnosticKind {
    PossiblyStaleBundle,
    ArtifactMissing,
    MetadataIncomplete,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseBundleDiagnostic {
    pub kind: LeaseBundleDiagnosticKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
}
