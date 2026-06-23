use super::super::*;
use serde_json::Value;
use std::fs;
use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;
use tokio::process::{Child, Command};

#[derive(Debug, Default, Clone)]
pub(super) struct FailingSpawner;

impl worker_runtime_host_gen::lease_runtime::LeaseSpawner for FailingSpawner {
    fn spawn_worker<'a>(
        &'a self,
        _config: &'a worker_runtime_host_gen::lease_runtime::LeaseRuntimeConfig,
        launch: &'a worker_runtime_host_gen::lease_runtime::LeaseLaunchConfig,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Child, worker_runtime_host_gen::lease_manager::LeaseError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let stdout_log = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(launch.log_dir.join("stdout.log"))
                .map_err(worker_runtime_host_gen::lease_manager::LeaseError::from)?;
            let stderr_log = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(launch.log_dir.join("stderr.log"))
                .map_err(worker_runtime_host_gen::lease_manager::LeaseError::from)?;

            Command::new("python3")
                .arg("-c")
                .arg(
                    r#"import sys
print("boom", flush=True)
sys.exit(1)
"#,
                )
                .stdout(Stdio::from(stdout_log))
                .stderr(Stdio::from(stderr_log))
                .spawn()
                .map_err(worker_runtime_host_gen::lease_manager::LeaseError::from)
        })
    }
}

async fn wait_for_failed(router: axum::Router, lease_id: &str) -> Value {
    let mut last_response = None;
    for _ in 0..240 {
        let response = request(
            router.clone(),
            axum::http::Method::GET,
            &format!("/leases/{lease_id}"),
            serde_json::json!({}),
        )
        .await;
        last_response = Some(response.clone());
        if response["status"]["state"] == "failed" {
            return response;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    panic!("lease did not fail: {last_response:?}");
}

pub(super) async fn create_failed_lease(router: axum::Router, lease_name: &str) -> String {
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
    let failed = wait_for_failed(router.clone(), &lease_id).await;
    assert_eq!(failed["status"]["state"], "failed");

    let _ = request(
        router,
        axum::http::Method::DELETE,
        &format!("/leases/{lease_id}"),
        serde_json::json!({}),
    )
    .await;

    lease_id
}
