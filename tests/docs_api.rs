#[path = "docs_api/support.rs"]
mod support;

use axum::http::StatusCode;
use support::{empty_plan, route_text, test_config, test_plan};
use worker_runtime_host_gen::docs::app;

#[tokio::test]
async fn docs_endpoints_serve_json_yaml_and_html() {
    let router = app(test_config(), test_plan());

    let (status, content_type, css) = route_text(router.clone(), "/docs.css").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "text/css; charset=utf-8");
    assert!(css.contains("font-family"));

    let (status, content_type, instructions) =
        route_text(router.clone(), "/instructions.json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "application/json");
    assert_contains_all(&instructions, INSTRUCTION_MARKERS);

    let (status, content_type, openapi) = route_text(router.clone(), "/openapi.yaml").await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type.contains("yaml"));
    assert_contains_all(&openapi, OPENAPI_MARKERS);

    let (status, content_type, html) = route_text(router, "/instructions.html").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type, "text/html; charset=utf-8");
    assert_contains_all(&html, HTML_MARKERS);
}

#[tokio::test]
async fn docs_endpoints_reflect_leases_only_mode_for_empty_plan() {
    let router = app(test_config(), empty_plan());

    let (status, _, instructions) = route_text(router.clone(), "/instructions.json").await;
    assert_eq!(status, StatusCode::OK);
    assert!(instructions.contains("\"bootstrap_mode\":\"leases-only\""));
    assert!(instructions.contains("leases are isolated and reusable until deleted"));

    let (status, _, projects) = route_text(router, "/projects.json").await;
    assert_eq!(status, StatusCode::OK);
    assert!(projects.contains("\"projects\":[]"));
}

fn assert_contains_all(text: &str, markers: &[&str]) {
    for marker in markers {
        assert!(text.contains(marker), "missing marker: {marker}");
    }
}

const INSTRUCTION_MARKERS: &[&str] = &[
    "bootstrap_mode",
    "food-tracker",
    "static_dir",
    "/leases",
    "/leases/{id}/logs",
    "/leases/{id}/debug",
    "/leases/{id}/failure-report",
    "/leases/{id}/probe",
    "/leases/{id}/filesystem-snapshot",
    "client_contract",
    "alpha-test",
    "wrangler.toml",
    "lease_worker_bin",
    "env_vars",
    "persist_state",
    "backend",
];

const OPENAPI_MARKERS: &[&str] = &[
    "/docs.css",
    "/instructions.json",
    "/project/{name}.json",
    "/leases/{id}/restart",
    "/leases/{id}/logs",
    "/leases/{id}/debug",
    "/leases/{id}/failure-report",
    "/leases/{id}/probe",
    "/leases/{id}/filesystem-snapshot",
    "Leases are isolated and reusable until deleted.",
    "Upload a complete prebuilt replacement bundle for one isolated lease.",
    "miniflare",
    "bad request",
    "lease not found",
    "lease worker or inspector ports unavailable",
    "lease startup unavailable",
];

const HTML_MARKERS: &[&str] = &[
    "Worker Runtime Host",
    "food-tracker",
    "/docs.css",
    "Client Contract",
    "Create a lease",
    "Bundle runtime files",
];
