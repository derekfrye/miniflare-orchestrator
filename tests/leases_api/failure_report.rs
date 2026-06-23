#[path = "failure_report/support.rs"]
mod support;

use super::*;
use std::sync::Arc;
use support::{FailingSpawner, create_failed_lease};
use worker_runtime_host_gen::lease_manager::unknown_lease_message;

#[tokio::test]
async fn failed_lease_failure_report_survives_deletion() {
    let temp = make_temp_dir();
    let worker_bin = bin("worker-runtime-host-worker");
    let wrangler_bin = temp.path().join("bin/wrangler");
    make_executable(&wrangler_bin, &fake_wrangler_script());

    let config = test_config_with_retention(
        &temp,
        &worker_bin.display().to_string(),
        &wrangler_bin.display().to_string(),
        1,
        86_400,
        100,
    );
    let router = worker_runtime_host_gen::docs_routes::app_with_spawner(
        config,
        test_plan(),
        Arc::new(FailingSpawner),
    );

    let lease_id = create_failed_lease(router.clone(), "failed-lease").await;

    let report = request(
        router,
        axum::http::Method::GET,
        &format!("/leases/{lease_id}/failure-report"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(report["lease"]["id"], lease_id);
    assert_eq!(report["lease"]["status"]["state"], "failed");
    assert!(
        report["lease"]["status"]["message"]
            .as_str()
            .expect("failure message")
            .contains("worker exited")
    );
    assert_eq!(
        report["startup"]["worker_bin"],
        worker_bin.display().to_string()
    );
    assert!(
        report["log_tail"]
            .as_str()
            .expect("log tail")
            .contains("boom")
    );
}

#[tokio::test]
async fn retained_failure_reports_are_pruned_by_ttl_and_max_size() {
    let temp = make_temp_dir();
    let worker_bin = bin("worker-runtime-host-worker");
    let wrangler_bin = temp.path().join("bin/wrangler");
    make_executable(&wrangler_bin, &fake_wrangler_script());

    let config = test_config_with_retention(
        &temp,
        &worker_bin.display().to_string(),
        &wrangler_bin.display().to_string(),
        3,
        2,
        2,
    );
    let router = worker_runtime_host_gen::docs_routes::app_with_spawner(
        config,
        test_plan(),
        Arc::new(FailingSpawner),
    );

    let alpha_id = create_failed_lease(router.clone(), "alpha-failed").await;
    let beta_id = create_failed_lease(router.clone(), "beta-failed").await;
    let gamma_id = create_failed_lease(router.clone(), "gamma-failed").await;

    let alpha_report = request(
        router.clone(),
        axum::http::Method::GET,
        &format!("/leases/{alpha_id}/failure-report"),
        serde_json::json!({}),
    )
    .await;
    let beta_report = request(
        router.clone(),
        axum::http::Method::GET,
        &format!("/leases/{beta_id}/failure-report"),
        serde_json::json!({}),
    )
    .await;
    let gamma_report = request(
        router.clone(),
        axum::http::Method::GET,
        &format!("/leases/{gamma_id}/failure-report"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(beta_report["lease"]["status"]["state"], "failed");
    assert_eq!(gamma_report["lease"]["status"]["state"], "failed");
    assert_eq!(alpha_report["error"], unknown_lease_message(&alpha_id));

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let alpha_pruned = request(
        router.clone(),
        axum::http::Method::GET,
        &format!("/leases/{alpha_id}/failure-report"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(alpha_pruned["error"], unknown_lease_message(&alpha_id));

    let beta_still_available = request(
        router.clone(),
        axum::http::Method::GET,
        &format!("/leases/{beta_id}/failure-report"),
        serde_json::json!({}),
    )
    .await;
    let gamma_still_available = request(
        router,
        axum::http::Method::GET,
        &format!("/leases/{gamma_id}/failure-report"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(
        beta_still_available["error"],
        unknown_lease_message(&beta_id)
    );
    assert_eq!(
        gamma_still_available["error"],
        unknown_lease_message(&gamma_id)
    );
}
