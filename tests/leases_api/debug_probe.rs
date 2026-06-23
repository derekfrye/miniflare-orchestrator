use super::*;

use std::sync::Arc;

#[tokio::test]
async fn lease_debug_and_probe_expose_launch_and_health_details() {
    let temp = make_temp_dir();
    let worker_bin = bin("worker-runtime-host-worker");
    let wrangler_bin = temp.path().join("bin/wrangler");
    make_executable(&wrangler_bin, &fake_wrangler_script());
    let recording_spawner = RecordingSpawner::new();

    let config = test_config(
        &temp,
        &worker_bin.display().to_string(),
        &wrangler_bin.display().to_string(),
        1,
    );
    let router = worker_runtime_host_gen::docs_routes::app_with_spawner(
        config,
        test_plan(),
        Arc::new(recording_spawner.clone()),
    );

    let created = request(
        router.clone(),
        axum::http::Method::POST,
        "/leases",
        serde_json::json!({
            "name": "debug-test",
            "health_path": "/health",
            "env": "dev",
            "protocol": "http",
            "log_level": "warn",
            "env_vars": {
                "API_URL": "https://example.invalid"
            }
        }),
    )
    .await;
    let lease_id = created["id"].as_str().expect("id").to_string();

    let _ = request(
        router.clone(),
        axum::http::Method::POST,
        &format!("/leases/{lease_id}/bundle"),
        lease_bundle(),
    )
    .await;
    let _ = request(
        router.clone(),
        axum::http::Method::POST,
        &format!("/leases/{lease_id}/restart"),
        serde_json::json!({}),
    )
    .await;

    let ready = wait_for_ready(router.clone(), &lease_id).await;
    assert_eq!(ready["status"]["state"], "ready");

    let debug = request(
        router.clone(),
        axum::http::Method::GET,
        &format!("/leases/{lease_id}/debug"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(debug["lease"]["status"]["state"], "ready");
    assert_eq!(
        debug["startup"]["worker_bin"],
        worker_bin.display().to_string()
    );
    assert!(
        debug["startup"]["static_dir"]
            .as_str()
            .expect("static dir")
            .contains(&lease_id)
    );
    assert_eq!(
        debug["startup"]["injected_env"]["API_URL"],
        "https://example.invalid"
    );
    assert_eq!(debug["last_probe"]["outcome"], "healthy");
    assert_eq!(debug["last_probe"]["status_code"], 200);

    let probe = request(
        router.clone(),
        axum::http::Method::GET,
        &format!("/leases/{lease_id}/probe"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(probe["outcome"], "healthy");
    assert_eq!(probe["request_method"], "GET");
    assert_eq!(probe["status_code"], 200);

    let _ = request(
        router,
        axum::http::Method::DELETE,
        &format!("/leases/{lease_id}"),
        serde_json::json!({}),
    )
    .await;

    let calls = recording_spawner.calls.lock().await;
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].env_vars.get("API_URL").map(String::as_str),
        Some("https://example.invalid")
    );
}
