use crate::docs_model::InstructionsDocument;
use crate::docs_openapi_paths::build_paths;
use crate::docs_openapi_schemas::schema_components;
use crate::plan::Plan;
use crate::service::DocsServiceConfig;
use serde_json::{Value, json};

#[must_use]
pub fn openapi_json(
    config: &DocsServiceConfig,
    _plan: &Plan,
    _instructions: &InstructionsDocument,
) -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Worker Runtime Host",
            "version": "1.0.0",
            "description": "Local runtime host control plane for isolated Worker test leases. Leases are isolated and reusable until deleted. The host allocates Worker request ports and inspector ports under the lease-state lock and prevents same-kind and cross-kind port collisions across active leases. Lease launches use the requested backend, miniflare by default or wrangler_dev, and uploaded runtime bundles must already contain built artifacts."
        },
        "servers": [
            {"url": format!("http://{}:{}", config.bind, config.port)}
        ],
        "paths": build_paths(),
        "components": {
            "schemas": schema_components()
        }
    })
}

/// Serializes the `OpenAPI` document as YAML.
///
/// # Errors
///
/// Returns an error if YAML serialization fails.
pub fn openapi_yaml(doc: &Value) -> Result<String, yaml_serde::Error> {
    yaml_serde::to_string(doc)
}
