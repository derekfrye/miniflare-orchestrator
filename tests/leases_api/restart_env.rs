use super::*;

use std::sync::Arc;

#[tokio::test]
async fn lease_restart_can_override_persist_state_and_carry_env_vars() {
    let temp = make_temp_dir();
    let ((port, _), (inspector_port, _)) = available_port_ranges(1);
    let worker_bin = bin("worker-runtime-host-worker");
    let wrangler_bin = temp.path().join("bin/wrangler");
    make_executable(&wrangler_bin, &fake_wrangler_script());

    let recording_spawner = RecordingSpawner::new();
    let config = worker_runtime_host_gen::service::DocsServiceConfig {
        bind: "127.0.0.1".to_string(),
        port: 8786,
        plan_file: temp.path().join("work/host/config/projects.plan.json"),
        host_root: temp.path().join("work/host"),
        lease_root: temp.path().join("work/host/leases"),
        lease_port_start: port,
        lease_port_end: port,
        lease_inspector_port_start: inspector_port,
        lease_inspector_port_end: inspector_port,
        worker_bin: worker_bin.clone(),
        wrangler_bin: wrangler_bin.clone(),
        failure_report_ttl_secs: 86_400,
        failure_report_max_entries: 100,
    };
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
            "name": "env-test",
            "health_path": "/health",
            "env": "dev",
            "protocol": "http",
            "log_level": "warn",
            "env_vars": {
                "API_URL": "https://example.invalid"
            },
            "persist_state": true
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

    let restart = request(
        router.clone(),
        axum::http::Method::POST,
        &format!("/leases/{lease_id}/restart"),
        serde_json::json!({
            "persist_state": false
        }),
    )
    .await;
    assert_eq!(restart["status"]["state"], "starting");

    let ready = wait_for_ready(router.clone(), &lease_id).await;
    assert_eq!(ready["status"]["state"], "ready");

    let _ = request(
        router.clone(),
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
    assert!(!calls[0].persist_state);
}
