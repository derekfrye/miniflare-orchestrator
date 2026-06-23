use super::*;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

mod bundle;

#[tokio::test]
async fn leases_can_be_created_bundled_restarted_and_deleted() {
    let temp = make_temp_dir();
    let worker_bin = bin("worker-runtime-host-worker");
    let wrangler_bin = temp.path().join("bin/wrangler");
    make_executable(&wrangler_bin, &fake_wrangler_script());

    let config = test_config(
        &temp,
        &worker_bin.display().to_string(),
        &wrangler_bin.display().to_string(),
        1,
    );
    let lease_port = config.lease_port_start;
    let router = worker_runtime_host_gen::docs::app(config, test_plan());

    let created = request(
        router.clone(),
        axum::http::Method::POST,
        "/leases",
        serde_json::json!({
            "name": "food-tracker-test",
            "health_path": "/health",
            "env": "dev",
            "protocol": "http",
            "log_level": "warn"
        }),
    )
    .await;
    let lease_id = created["id"].as_str().expect("id").to_string();
    assert_eq!(created["port"], lease_port);
    assert_eq!(created["status"]["state"], "created");
    assert_eq!(created["backend"], "miniflare");
    assert!(
        created["prebuilt_bundle_notice"]["message"]
            .as_str()
            .expect("prebuilt notice")
            .contains("prebuilt bundle")
    );

    let bundle = request(
        router.clone(),
        axum::http::Method::POST,
        &format!("/leases/{lease_id}/bundle"),
        lease_bundle(),
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
    assert_eq!(
        restart["health_url"],
        format!("http://127.0.0.1:{lease_port}/health")
    );

    let deleted = request(
        router.clone(),
        axum::http::Method::DELETE,
        &format!("/leases/{lease_id}"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(deleted["status"]["state"], "stopped");

    let response = router
        .oneshot(
            Request::builder()
                .uri(format!("/leases/{lease_id}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
