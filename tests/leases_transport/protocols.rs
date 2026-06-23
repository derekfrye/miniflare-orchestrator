use super::support::{
    TestBackend, TransportHarness, delete_lease, provision_lease, restart_lease, wait_for_ready,
};

#[tokio::test]
async fn lease_http_and_https_work_against_the_real_worker_process() {
    for backend in TestBackend::all() {
        lease_http_and_https_work_against_the_real_worker_process_for_backend(backend).await;
    }
}

async fn lease_http_and_https_work_against_the_real_worker_process_for_backend(
    backend: TestBackend,
) {
    let harness = TransportHarness::with_capacity(2);
    let lease_port_start = harness.lease_port_start;
    let router = harness.router;

    let (plain_lease_id, plain_created) = provision_lease(
        router.clone(),
        "http-lease",
        "http",
        backend,
        serde_json::json!({}),
    )
    .await;
    let (tls_lease_id, tls_created) = provision_lease(
        router.clone(),
        "https-lease",
        "https",
        backend,
        serde_json::json!({}),
    )
    .await;

    assert_eq!(
        plain_created["base_url"],
        format!("http://127.0.0.1:{lease_port_start}")
    );
    assert_eq!(
        tls_created["base_url"],
        format!("https://127.0.0.1:{}", lease_port_start + 1)
    );
    assert_eq!(plain_created["backend"], backend.name());
    assert_eq!(tls_created["backend"], backend.name());

    let plain_restart = restart_lease(router.clone(), &plain_lease_id).await;
    let tls_restart = restart_lease(router.clone(), &tls_lease_id).await;
    assert_eq!(plain_restart["status"]["state"], "starting");
    assert_eq!(tls_restart["status"]["state"], "starting");

    let plain_ready = wait_for_ready(router.clone(), &plain_lease_id).await;
    let tls_ready = wait_for_ready(router.clone(), &tls_lease_id).await;
    assert_eq!(plain_ready["backend"], backend.name());
    assert_eq!(tls_ready["backend"], backend.name());

    assert_plain_health(&plain_ready).await;
    assert_tls_health(&tls_ready).await;

    let plain_deleted = delete_lease(router.clone(), &plain_lease_id).await;
    let tls_deleted = delete_lease(router, &tls_lease_id).await;

    assert_eq!(plain_deleted["status"]["state"], "stopped");
    assert_eq!(tls_deleted["status"]["state"], "stopped");
}

async fn assert_plain_health(lease: &serde_json::Value) {
    let response = reqwest::get(format!(
        "{}/health",
        lease["base_url"].as_str().expect("base url")
    ))
    .await
    .expect("plain health");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

async fn assert_tls_health(lease: &serde_json::Value) {
    let https_client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("https client");
    let response = https_client
        .get(format!(
            "{}/health",
            lease["base_url"].as_str().expect("base url")
        ))
        .send()
        .await
        .expect("tls health");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
}
