#[path = "debug_isolation/assertions.rs"]
mod assertions;

use super::*;
use assertions::{assert_debug_isolated, assert_snapshots_isolated};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Map, json};
use std::sync::Arc;

struct LeaseRoutePaths {
    bundle: String,
    restart: String,
    debug: String,
    snapshot: String,
}

impl LeaseRoutePaths {
    fn new(id: &str) -> Self {
        Self {
            bundle: format!("/leases/{id}/bundle"),
            restart: format!("/leases/{id}/restart"),
            debug: format!("/leases/{id}/debug"),
            snapshot: format!("/leases/{id}/filesystem-snapshot"),
        }
    }
}

fn custom_bundle(label: &str) -> serde_json::Value {
    json!({
        "runtime_files": [
            {
                "path": "wrangler.toml",
                "content_b64": STANDARD.encode(format!("name = \"{label}\"\n"))
            },
            {
                "path": "worker_entry.mjs",
                "content_b64": STANDARD.encode("export default {}\n")
            },
            {
                "path": format!("{label}.txt"),
                "content_b64": STANDARD.encode(format!("{label}-only\n"))
            }
        ],
        "static_files": [
            {
                "path": "index.html",
                "content_b64": STANDARD.encode(format!("<h1>{label}</h1>\n"))
            }
        ]
    })
}

async fn create_lease(router: axum::Router, name: &str, env_key: &str) -> serde_json::Value {
    let mut env_vars = Map::new();
    env_vars.insert(env_key.to_string(), json!(format!("{name}-secret")));
    request(
        router,
        axum::http::Method::POST,
        "/leases",
        json!({
            "name": name,
            "health_path": "/health",
            "env": "dev",
            "protocol": "http",
            "log_level": "warn",
            "env_vars": env_vars
        }),
    )
    .await
}

#[tokio::test]
async fn concurrent_leases_keep_debug_and_snapshot_data_isolated() {
    let recording_spawner = RecordingSpawner::new();
    let router = test_router(recording_spawner.clone());

    let (alpha_created, beta_created) = tokio::join!(
        create_lease(router.clone(), "alpha", "ALPHA_TOKEN"),
        create_lease(router.clone(), "beta", "BETA_TOKEN"),
    );
    let alpha_id = alpha_created["id"].as_str().expect("alpha id").to_string();
    let beta_id = beta_created["id"].as_str().expect("beta id").to_string();
    let alpha_paths = LeaseRoutePaths::new(&alpha_id);
    let beta_paths = LeaseRoutePaths::new(&beta_id);

    let (alpha_bundle, beta_bundle) =
        bundle_leases(router.clone(), &alpha_paths, &beta_paths).await;
    assert_eq!(alpha_bundle["status"]["state"], "bundled");
    assert_eq!(beta_bundle["status"]["state"], "bundled");

    let (alpha_restart, beta_restart) =
        restart_leases(router.clone(), &alpha_paths, &beta_paths).await;
    assert_eq!(alpha_restart["status"]["state"], "starting");
    assert_eq!(beta_restart["status"]["state"], "starting");

    let (alpha_ready, beta_ready) = tokio::join!(
        wait_for_ready(router.clone(), &alpha_id),
        wait_for_ready(router.clone(), &beta_id),
    );
    assert_eq!(alpha_ready["status"]["state"], "ready");
    assert_eq!(beta_ready["status"]["state"], "ready");

    let (alpha_debug, beta_debug) =
        fetch_pair(router.clone(), &alpha_paths.debug, &beta_paths.debug).await;
    assert_debug_isolated(&alpha_debug, &beta_debug, &alpha_id, &beta_id);

    let (alpha_snapshot, beta_snapshot) =
        fetch_pair(router.clone(), &alpha_paths.snapshot, &beta_paths.snapshot).await;
    assert_snapshots_isolated(&alpha_snapshot, &beta_snapshot, &alpha_id, &beta_id);

    delete_leases(router, &alpha_id, &beta_id).await;

    let calls = recording_spawner.calls.lock().await;
    assert_eq!(calls.len(), 2);
}

fn test_router(recording_spawner: RecordingSpawner) -> axum::Router {
    let temp = make_temp_dir();
    let worker_bin = bin("worker-runtime-host-worker");
    let wrangler_bin = temp.path().join("bin/wrangler");
    make_executable(&wrangler_bin, &fake_wrangler_script());

    let config = test_config(
        &temp,
        &worker_bin.display().to_string(),
        &wrangler_bin.display().to_string(),
        2,
    );
    worker_runtime_host_gen::docs_routes::app_with_spawner(
        config,
        test_plan(),
        Arc::new(recording_spawner),
    )
}

async fn bundle_leases(
    router: axum::Router,
    alpha_paths: &LeaseRoutePaths,
    beta_paths: &LeaseRoutePaths,
) -> (serde_json::Value, serde_json::Value) {
    tokio::join!(
        request(
            router.clone(),
            axum::http::Method::POST,
            &alpha_paths.bundle,
            custom_bundle("alpha"),
        ),
        request(
            router,
            axum::http::Method::POST,
            &beta_paths.bundle,
            custom_bundle("beta"),
        ),
    )
}

async fn restart_leases(
    router: axum::Router,
    alpha_paths: &LeaseRoutePaths,
    beta_paths: &LeaseRoutePaths,
) -> (serde_json::Value, serde_json::Value) {
    tokio::join!(
        request(
            router.clone(),
            axum::http::Method::POST,
            &alpha_paths.restart,
            json!({}),
        ),
        request(
            router,
            axum::http::Method::POST,
            &beta_paths.restart,
            json!({}),
        ),
    )
}

async fn fetch_pair(
    router: axum::Router,
    alpha_path: &str,
    beta_path: &str,
) -> (serde_json::Value, serde_json::Value) {
    tokio::join!(
        request(
            router.clone(),
            axum::http::Method::GET,
            alpha_path,
            json!({})
        ),
        request(router, axum::http::Method::GET, beta_path, json!({})),
    )
}

async fn delete_leases(router: axum::Router, alpha_id: &str, beta_id: &str) {
    let alpha_path = format!("/leases/{alpha_id}");
    let beta_path = format!("/leases/{beta_id}");
    let _ = tokio::join!(
        request(
            router.clone(),
            axum::http::Method::DELETE,
            &alpha_path,
            json!({}),
        ),
        request(router, axum::http::Method::DELETE, &beta_path, json!({}),),
    );
}
