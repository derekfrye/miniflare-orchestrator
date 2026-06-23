use super::*;

#[tokio::test]
async fn lease_create_with_https_reports_https_urls() {
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
        router,
        axum::http::Method::POST,
        "/leases",
        serde_json::json!({
            "name": "https-test",
            "health_path": "/health",
            "env": "dev",
            "protocol": "https",
            "log_level": "warn"
        }),
    )
    .await;
    assert_eq!(
        created["base_url"],
        format!("https://127.0.0.1:{lease_port}")
    );
    assert_eq!(
        created["health_url"],
        format!("https://127.0.0.1:{lease_port}/health")
    );
    assert_eq!(created["protocol"], "https");
}
