use crate::basic::{available_port_ranges, bin, make_executable, make_temp_dir};
use crate::lease_shared_fixtures::{fake_wrangler_script, lease_bundle};
use crate::plan_fixture::test_plan;
use crate::request_json::request;
use tokio::time::{Duration, sleep};

#[derive(Clone, Copy)]
pub(crate) enum TestBackend {
    WranglerDev,
    Miniflare,
}

impl TestBackend {
    pub(crate) fn all() -> [Self; 2] {
        [Self::WranglerDev, Self::Miniflare]
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::WranglerDev => "wrangler_dev",
            Self::Miniflare => "miniflare",
        }
    }
}

pub(crate) struct TransportHarness {
    _temp: tempfile::TempDir,
    pub(crate) router: axum::Router,
    pub(crate) lease_port_start: u16,
}

impl TransportHarness {
    pub(crate) fn with_capacity(lease_count: u16) -> Self {
        let temp = make_temp_dir();
        let worker_bin = bin("worker-runtime-host-worker");
        let wrangler_bin = temp.path().join("bin/wrangler");
        make_executable(&wrangler_bin, &fake_wrangler_script());
        let ((lease_port_start, lease_port_end), (inspector_port_start, inspector_port_end)) =
            available_port_ranges(lease_count);

        let config = lease_config(
            &temp,
            &worker_bin.display().to_string(),
            &wrangler_bin.display().to_string(),
            lease_port_start,
            lease_port_end,
            inspector_port_start,
            inspector_port_end,
        );
        let router = worker_runtime_host_gen::docs::app(config, test_plan());
        Self {
            _temp: temp,
            router,
            lease_port_start,
        }
    }
}

fn lease_config(
    temp: &tempfile::TempDir,
    worker_bin: &str,
    wrangler_bin: &str,
    lease_port_start: u16,
    lease_port_end: u16,
    inspector_port_start: u16,
    inspector_port_end: u16,
) -> worker_runtime_host_gen::service::DocsServiceConfig {
    worker_runtime_host_gen::service::DocsServiceConfig {
        bind: "127.0.0.1".to_string(),
        port: 8786,
        plan_file: temp.path().join("work/host/config/projects.plan.json"),
        host_root: temp.path().join("work/host"),
        lease_root: temp.path().join("work/host/leases"),
        lease_port_start,
        lease_port_end,
        lease_inspector_port_start: inspector_port_start,
        lease_inspector_port_end: inspector_port_end,
        worker_bin: worker_bin.into(),
        wrangler_bin: wrangler_bin.into(),
        failure_report_ttl_secs: 86_400,
        failure_report_max_entries: 100,
    }
}

pub(crate) async fn provision_lease(
    router: axum::Router,
    name: &str,
    protocol: &str,
    backend: TestBackend,
    env_vars: serde_json::Value,
) -> (String, serde_json::Value) {
    let created = request(
        router.clone(),
        axum::http::Method::POST,
        "/leases",
        serde_json::json!({
            "name": name,
            "health_path": "/health",
            "env": "dev",
            "protocol": protocol,
            "log_level": "warn",
            "backend": backend.name(),
            "env_vars": env_vars
        }),
    )
    .await;
    let lease_id = created["id"].as_str().expect("lease id").to_string();

    let _ = request(
        router,
        axum::http::Method::POST,
        &format!("/leases/{lease_id}/bundle"),
        lease_bundle(),
    )
    .await;
    assert_eq!(created["status"]["state"], "created");

    (lease_id, created)
}

pub(crate) async fn restart_lease(router: axum::Router, lease_id: &str) -> serde_json::Value {
    request(
        router,
        axum::http::Method::POST,
        &format!("/leases/{lease_id}/restart"),
        serde_json::json!({}),
    )
    .await
}

pub(crate) async fn delete_lease(router: axum::Router, lease_id: &str) -> serde_json::Value {
    request(
        router,
        axum::http::Method::DELETE,
        &format!("/leases/{lease_id}"),
        serde_json::json!({}),
    )
    .await
}

pub(crate) async fn wait_for_ready(router: axum::Router, lease_id: &str) -> serde_json::Value {
    for _ in 0..80 {
        let lease = request(
            router.clone(),
            axum::http::Method::GET,
            &format!("/leases/{lease_id}"),
            serde_json::json!({}),
        )
        .await;
        if lease["status"]["state"] == "ready" {
            return lease;
        }
        assert!(
            lease["status"]["state"] != "failed",
            "lease failed: {lease:?}"
        );
        sleep(Duration::from_millis(100)).await;
    }
    panic!("lease did not become ready");
}
