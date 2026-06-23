use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use worker_runtime_host_gen::plan::{Plan, PlannedProject};
use worker_runtime_host_gen::service::DocsServiceConfig;

#[path = "../support/ports.rs"]
mod ports;

pub fn test_plan() -> Plan {
    Plan {
        manifest: "/work/host/config/projects.json".into(),
        output_dir: "/etc/s6-overlay/s6-rc.d".into(),
        debug_plan_file: "/work/host/config/projects.plan.json".into(),
        service_root: "/run/service".into(),
        dry_run: false,
        projects: vec![PlannedProject {
            name: "food-tracker".to_string(),
            runtime_dir: "/work/host/projects/food-tracker/runtime".into(),
            state_dir: "/work/host/projects/food-tracker/state".into(),
            log_dir: "/work/host/projects/food-tracker/logs".into(),
            static_dir: "/work/host/projects/food-tracker/static".into(),
            config_file: "/work/host/projects/food-tracker/runtime/wrangler.toml".into(),
            reload_token: "/work/host/projects/food-tracker/runtime/.reload-token".into(),
            health_path: "/health".to_string(),
            port: 8788,
            inspector_port: 9100,
            env: "dev".to_string(),
            protocol: "http".to_string(),
            worker_service: "/etc/s6-overlay/s6-rc.d/food-tracker".into(),
            watcher_service: "/etc/s6-overlay/s6-rc.d/food-tracker-watch".into(),
            worker_run: "/etc/s6-overlay/s6-rc.d/food-tracker/run".into(),
            watcher_run: "/etc/s6-overlay/s6-rc.d/food-tracker-watch/run".into(),
        }],
    }
}

pub fn test_config() -> DocsServiceConfig {
    let ((lease_port_start, lease_port_end), (inspector_port_start, inspector_port_end)) =
        ports::available_port_ranges(100);
    DocsServiceConfig {
        bind: "127.0.0.1".to_string(),
        port: 8786,
        plan_file: "/work/host/config/projects.plan.json".into(),
        host_root: "/work/host".into(),
        lease_root: "/work/host/leases".into(),
        lease_port_start,
        lease_port_end,
        lease_inspector_port_start: inspector_port_start,
        lease_inspector_port_end: inspector_port_end,
        worker_bin: "/usr/local/bin/worker-runtime-host-worker".into(),
        wrangler_bin: "wrangler".into(),
        failure_report_ttl_secs: 86_400,
        failure_report_max_entries: 100,
    }
}

pub fn empty_plan() -> Plan {
    Plan {
        manifest: "/work/host/config/projects.json".into(),
        output_dir: "/etc/s6-overlay/s6-rc.d".into(),
        debug_plan_file: "/work/host/config/projects.plan.json".into(),
        service_root: "/run/service".into(),
        dry_run: false,
        projects: Vec::new(),
    }
}

pub async fn route_text(router: axum::Router, uri: &str) -> (StatusCode, String, String) {
    let response = router
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let content_type = response.headers()["content-type"]
        .to_str()
        .unwrap()
        .to_string();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        content_type,
        String::from_utf8(body.to_vec()).unwrap(),
    )
}
