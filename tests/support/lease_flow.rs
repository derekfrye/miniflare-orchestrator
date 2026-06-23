use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::Value;

use super::request_helpers::{request, wait_for_ready};

pub struct LeaseOutcome {
    pub id: String,
    pub response: Value,
}

/// Exercises the create/bundle/restart flow for a lease.
///
/// # Panics
///
/// Panics if the API does not return the expected state transitions.
#[must_use]
pub async fn lease_flow(
    router: axum::Router,
    lease_name: &str,
    runtime_body: &str,
    static_body: &str,
) -> LeaseOutcome {
    let created = request(
        router.clone(),
        axum::http::Method::POST,
        "/leases",
        serde_json::json!({
            "name": lease_name,
            "health_path": "/health",
            "env": "dev",
            "protocol": "http",
            "log_level": "warn"
        }),
    )
    .await;
    let lease_id = created["id"].as_str().expect("id").to_string();

    let bundle = request(
        router.clone(),
        axum::http::Method::POST,
        &format!("/leases/{lease_id}/bundle"),
        serde_json::json!({
            "runtime_files": [
                {
                    "path": "wrangler.toml",
                    "content_b64": STANDARD.encode(format!("name = \"{lease_name}\"\n"))
                },
                {
                    "path": "worker_entry.mjs",
                    "content_b64": STANDARD.encode(runtime_body)
                }
            ],
            "static_files": [
                {
                    "path": "index.html",
                    "content_b64": STANDARD.encode(static_body)
                }
            ]
        }),
    )
    .await;
    assert_eq!(bundle["status"]["state"], "bundled");

    let restart = request(
        router.clone(),
        axum::http::Method::POST,
        &format!("/leases/{lease_id}/restart"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(restart["status"]["state"], "starting");

    let response = wait_for_ready(router, &lease_id).await;
    LeaseOutcome {
        id: lease_id,
        response,
    }
}
