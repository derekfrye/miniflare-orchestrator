use super::*;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use std::path::Path;
use tower::ServiceExt;

#[tokio::test]
async fn lease_log_endpoints_are_scoped_to_the_requested_lease() {
    let temp = make_temp_dir();
    let worker_bin = bin("worker-runtime-host-worker");
    let wrangler_bin = temp.path().join("bin/wrangler");
    make_executable(&wrangler_bin, &fake_wrangler_script());
    let ((lease_port_start, lease_port_end), (inspector_port_start, inspector_port_end)) =
        available_port_ranges(2);

    let config = worker_runtime_host_gen::service::DocsServiceConfig {
        bind: "127.0.0.1".to_string(),
        port: 8786,
        plan_file: temp.path().join("work/host/config/projects.plan.json"),
        host_root: temp.path().join("work/host"),
        lease_root: temp.path().join("work/host/leases"),
        lease_port_start,
        lease_port_end,
        lease_inspector_port_start: inspector_port_start,
        lease_inspector_port_end: inspector_port_end,
        worker_bin: worker_bin.clone(),
        wrangler_bin: wrangler_bin.clone(),
        failure_report_ttl_secs: 86_400,
        failure_report_max_entries: 100,
    };
    let router = worker_runtime_host_gen::docs::app(config, test_plan());

    let alpha = request(
        router.clone(),
        axum::http::Method::POST,
        "/leases",
        serde_json::json!({
            "name": "alpha",
            "health_path": "/health",
            "env": "dev",
            "protocol": "http",
            "log_level": "warn"
        }),
    )
    .await;
    let beta = request(
        router.clone(),
        axum::http::Method::POST,
        "/leases",
        serde_json::json!({
            "name": "beta",
            "health_path": "/health",
            "env": "dev",
            "protocol": "http",
            "log_level": "warn"
        }),
    )
    .await;

    let alpha_id = alpha["id"].as_str().expect("alpha id").to_string();
    let beta_id = beta["id"].as_str().expect("beta id").to_string();

    let alpha_log_dir = Path::new(alpha["log_dir"].as_str().expect("alpha log dir"));
    let beta_log_dir = Path::new(beta["log_dir"].as_str().expect("beta log dir"));
    std::fs::write(alpha_log_dir.join("stdout.log"), "alpha stdout\n").expect("alpha stdout");
    std::fs::write(alpha_log_dir.join("stderr.log"), "alpha stderr\n").expect("alpha stderr");
    std::fs::write(beta_log_dir.join("stdout.log"), "beta stdout\n").expect("beta stdout");
    std::fs::write(beta_log_dir.join("stderr.log"), "beta stderr\n").expect("beta stderr");

    let alpha_logs = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/leases/{alpha_id}/logs"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(alpha_logs.status(), StatusCode::OK);
    let alpha_body = to_bytes(alpha_logs.into_body(), usize::MAX)
        .await
        .expect("body");
    let alpha_text = String::from_utf8(alpha_body.to_vec()).expect("alpha text");
    assert!(alpha_text.contains("alpha stdout"));
    assert!(alpha_text.contains("alpha stderr"));
    assert!(!alpha_text.contains("beta stdout"));

    let beta_tail = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/leases/{beta_id}/logs/tail?lines=2"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(beta_tail.status(), StatusCode::OK);
    let beta_body = to_bytes(beta_tail.into_body(), usize::MAX)
        .await
        .expect("body");
    let beta_text = String::from_utf8(beta_body.to_vec()).expect("beta text");
    assert!(beta_text.starts_with('['));
    assert!(beta_text.contains("] [stdout.log] beta stdout"));
    assert!(beta_text.contains("] [stderr.log] beta stderr"));
    assert!(!beta_text.contains("alpha stdout"));
}
