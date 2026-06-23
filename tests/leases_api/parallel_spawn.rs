use super::*;

use std::path::Path;
use std::sync::Arc;

#[tokio::test]
async fn parallel_leases_write_their_files_and_reach_ready_via_injected_spawner() {
    let temp = make_temp_dir();
    let recording_spawner = RecordingSpawner::new();
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
        worker_bin: temp.path().join("bin/fake-worker"),
        wrangler_bin: temp.path().join("bin/fake-wrangler"),
        failure_report_ttl_secs: 86_400,
        failure_report_max_entries: 100,
    };
    let worker_bin = config.worker_bin.clone();
    let wrangler_bin = config.wrangler_bin.clone();
    let router = worker_runtime_host_gen::docs_routes::app_with_spawner(
        config,
        test_plan(),
        Arc::new(recording_spawner.clone()),
    );

    let (alpha, beta) = tokio::join!(
        lease_flow(
            router.clone(),
            "alpha",
            "export default { alpha: true }\n",
            "<h1>alpha</h1>\n"
        ),
        lease_flow(
            router.clone(),
            "beta",
            "export default { beta: true }\n",
            "<h1>beta</h1>\n"
        )
    );

    assert_eq!(alpha.response["status"]["state"], "ready");
    assert_eq!(beta.response["status"]["state"], "ready");
    assert_lease_artifacts(
        &alpha.response,
        "export default { alpha: true }\n",
        "<h1>alpha</h1>\n",
        "alpha",
    );
    assert_lease_artifacts(
        &beta.response,
        "export default { beta: true }\n",
        "<h1>beta</h1>\n",
        "beta",
    );

    let calls = recording_spawner.calls.lock().await;
    assert_spawn_calls(&calls, &worker_bin, &wrangler_bin);
    assert!(calls.iter().any(|call| {
        call.runtime_dir
            == Path::new(
                alpha.response["runtime_dir"]
                    .as_str()
                    .expect("alpha runtime"),
            )
    }));
    assert!(calls.iter().any(|call| {
        call.runtime_dir == Path::new(beta.response["runtime_dir"].as_str().expect("beta runtime"))
    }));
    assert!(calls.iter().any(|call| call.state_dir.ends_with("state")));
    assert!(calls.iter().any(|call| call.log_dir.ends_with("logs")));
    assert!(
        calls
            .iter()
            .any(|call| call.config_file.ends_with("wrangler.toml"))
    );

    let _ = request(
        router.clone(),
        axum::http::Method::DELETE,
        &format!("/leases/{}", alpha.id),
        serde_json::json!({}),
    )
    .await;
    let _ = request(
        router,
        axum::http::Method::DELETE,
        &format!("/leases/{}", beta.id),
        serde_json::json!({}),
    )
    .await;
}
