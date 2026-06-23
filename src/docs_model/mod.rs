use crate::plan::Plan;
use crate::service::DocsServiceConfig;
use schemars::JsonSchema;
use serde::Serialize;

#[path = "contract.rs"]
mod contract;
#[path = "endpoints.rs"]
mod endpoints;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct InstructionsDocument {
    pub version: u32,
    pub bootstrap_mode: String,
    pub host_root: String,
    pub docs_port: u16,
    pub lease_root: String,
    pub lease_port_range: String,
    pub lease_inspector_port_range: String,
    pub lease_worker_bin: String,
    pub lease_wrangler_bin: String,
    pub project_root_pattern: String,
    pub deploy_flow: Vec<String>,
    pub lease_flow: Vec<String>,
    pub endpoints: Vec<EndpointDocument>,
    pub client_contract: Vec<ClientContractExample>,
    pub projects: Vec<InstructionProject>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct InstructionProject {
    pub name: String,
    pub runtime_dir: String,
    pub static_dir: String,
    pub state_dir: String,
    pub log_dir: String,
    pub config_file: String,
    pub reload_token: String,
    pub port: u16,
    pub inspector_port: u16,
    pub health_url: String,
    pub env: String,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct EndpointDocument {
    pub method: String,
    pub path: String,
    pub description: String,
    pub content_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ClientContractExample {
    pub title: String,
    pub purpose: String,
    pub method: String,
    pub path: String,
    pub request_example: serde_json::Value,
    pub response_example: serde_json::Value,
}

pub fn build_instructions(config: &DocsServiceConfig, plan: &Plan) -> InstructionsDocument {
    let bootstrap_mode = if plan.projects.is_empty() {
        "leases-only"
    } else {
        "manifest_and_leases"
    };
    InstructionsDocument {
        version: 1,
        bootstrap_mode: bootstrap_mode.to_string(),
        host_root: config.host_root.display().to_string(),
        docs_port: config.port,
        lease_root: config.lease_root.display().to_string(),
        lease_port_range: format!("{}..={}", config.lease_port_start, config.lease_port_end),
        lease_inspector_port_range: format!(
            "{}..={}",
            config.lease_inspector_port_start, config.lease_inspector_port_end
        ),
        lease_worker_bin: config.worker_bin.display().to_string(),
        lease_wrangler_bin: config.wrangler_bin.display().to_string(),
        project_root_pattern: format!("{}/projects/<project>", config.host_root.display()),
        deploy_flow: vec![
            "build the Worker repo".to_string(),
            "copy wrangler.toml, worker_entry.mjs, built artifacts such as build/, and sibling static/ into the project root".to_string(),
            "atomically replace runtime/.reload-token".to_string(),
            "poll the project health URL".to_string(),
        ],
        lease_flow: vec![
            "POST /leases to allocate an isolated Worker request port, Wrangler inspector port, and project root".to_string(),
            "POST /leases/{id}/bundle to upload the prebuilt runtime bundle and static assets".to_string(),
            "POST /leases/{id}/restart to launch the Worker and wait for health".to_string(),
            "GET /leases/{id}/debug to inspect the launch metadata and last probe".to_string(),
            "GET /leases/{id}/probe to run the current health check and capture its response".to_string(),
            "GET /leases/{id}/filesystem-snapshot to inspect the lease runtime, static, state, and log directories".to_string(),
            "GET /leases/{id}/logs to fetch the current lease logs".to_string(),
            "GET /leases/{id}/logs/tail?lines=N to tail the current lease logs".to_string(),
            "poll GET /leases/{id} until status.state becomes ready".to_string(),
            "DELETE /leases/{id} to stop and release the lease".to_string(),
        ],
        endpoints: endpoints::build_endpoints(),
        client_contract: contract::build_client_contract(),
        projects: plan.projects.iter().map(endpoints::build_project).collect(),
        notes: vec![
            if plan.projects.is_empty() {
                "bootstrap mode is leases-only; fixed projects are not prewired".to_string()
            } else {
                "bootstrap mode is manifest_and_leases; fixed projects were loaded from the manifest".to_string()
            },
            "leases are isolated and reusable until deleted".to_string(),
            "lease Worker request ports and Wrangler inspector ports are allocated under the same control-plane lock so concurrent lease creation does not assign an active port twice".to_string(),
            "lease env_vars are injected only into the matching lease worker process and are not shared across leases".to_string(),
            "lease Worker processes receive HOME and TMPDIR scoped to their own state_dir so Wrangler caches and temporary files do not share /tmp across concurrent leases".to_string(),
            "lease protocol may be http or https; https uses the worker runtime's self-signed certificate and the lease URLs are returned with an https scheme".to_string(),
            "POST /leases/{id}/bundle replaces the lease runtime and static directory contents with a prebuilt bundle and may include bundle metadata describing the source root, build command, built artifacts, and source files".to_string(),
            "worker launches serve prebuilt artifacts; Wrangler build hooks in uploaded wrangler.toml files are skipped and callers must build before syncing or bundling".to_string(),
            "lease responses include prebuilt_bundle_notice, bundle_metadata, bundle_uploaded_at, and bundle_diagnostics so test harnesses can surface stale Worker bundle mistakes directly".to_string(),
            "POST /leases/{id}/restart replaces the current worker process for the lease and keeps the lease id, Worker request port, and Wrangler inspector port stable".to_string(),
            "POST /leases/{id}/restart accepts persist_state and backend so callers can choose state reuse and either miniflare or wrangler_dev execution per lease; leases default to miniflare".to_string(),
            "GET /leases/{id}/debug exposes the effective startup command, injected env, and last probe details for one lease".to_string(),
            "GET /leases/{id}/probe reruns the health probe and returns the full request/response details".to_string(),
            "GET /leases/{id}/filesystem-snapshot returns a recursive snapshot of the lease filesystem tree".to_string(),
            "GET /leases/{id}/logs and GET /leases/{id}/logs/tail only read files from that lease's log_dir".to_string(),
            "the manifest JSON schema is documented in the OpenAPI components as Manifest".to_string(),
            "lease bundles are expected to contain runtime_files for the prebuilt worker bundle and static_files for the test assets".to_string(),
            "runtime_files should include wrangler.toml, worker_entry.mjs, and any built JS/Wasm files imported by the entrypoint".to_string(),
            "bundle file paths must be relative and cannot contain traversal segments".to_string(),
            "runtime and static are separate sibling directories so assets.directory = \"../static\" stays isolated per project".to_string(),
            "the docs service is read-only and reflects the validated plan file".to_string(),
            "lease bundles are written under /work/host/leases/<id> and launched directly by the docs control plane".to_string(),
        ],
    }
}
