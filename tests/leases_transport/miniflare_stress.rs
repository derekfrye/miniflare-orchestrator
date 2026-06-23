use crate::basic::{available_port_ranges, bin, make_executable, make_temp_dir};
use crate::lease_shared_fixtures::{fake_wrangler_script, lease_bundle};
use crate::plan_fixture::test_plan;
use crate::request_json::request;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::task::JoinSet;
use tower::ServiceExt;

const DEFAULT_LANES: usize = 8;
const DEFAULT_ITERATIONS: usize = 10;
const DEFAULT_PROGRESS_INTERVAL_SECS: usize = 10;
const LEASES_PER_LANE: u16 = 2;

struct StressHarness {
    temp: tempfile::TempDir,
    router: axum::Router,
    lane: usize,
}

struct LeasePair {
    alpha_id: String,
    beta_id: String,
}

struct StressFailure {
    lane: usize,
    iteration: usize,
    message: String,
    retained_root: PathBuf,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "Miniflare startup stress test; run explicitly when investigating lease flakiness"]
async fn miniflare_concurrent_lease_startup_stress_retains_logs() {
    let lanes = env_usize("MINIFLARE_STRESS_LANES", DEFAULT_LANES);
    let iterations = env_usize("MINIFLARE_STRESS_ITERATIONS", DEFAULT_ITERATIONS);
    assert!(lanes > 0, "MINIFLARE_STRESS_LANES must be positive");
    assert!(
        lanes <= usize::from(u16::MAX / LEASES_PER_LANE),
        "MINIFLARE_STRESS_LANES is too large"
    );

    let total_leases = u16::try_from(lanes).expect("lanes fit u16") * LEASES_PER_LANE;
    let ((worker_start, _worker_end), (inspector_start, _inspector_end)) =
        available_port_ranges(total_leases);
    let mut tasks = JoinSet::new();
    let stop = Arc::new(AtomicBool::new(false));
    let completed = Arc::new(AtomicUsize::new(0));
    let total_iterations = lanes
        .checked_mul(iterations)
        .expect("stress progress total overflow");
    let progress_interval_secs = env_usize(
        "MINIFLARE_STRESS_PROGRESS_INTERVAL_SECS",
        DEFAULT_PROGRESS_INTERVAL_SECS,
    );
    let progress = tokio::spawn(report_progress(
        completed.clone(),
        stop.clone(),
        total_iterations,
        Duration::from_secs(progress_interval_secs as u64),
    ));

    for lane in 0..lanes {
        let offset = u16::try_from(lane).expect("lane fits u16") * LEASES_PER_LANE;
        let worker_ports = (worker_start + offset, worker_start + offset + 1);
        let inspector_ports = (inspector_start + offset, inspector_start + offset + 1);
        let stop = stop.clone();
        let completed = completed.clone();
        tasks.spawn(async move {
            run_lane(
                lane,
                iterations,
                worker_ports,
                inspector_ports,
                stop,
                completed,
            )
            .await
        });
    }

    let mut failures = Vec::new();
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(failure)) => {
                stop.store(true, Ordering::SeqCst);
                failures.push(failure);
            }
            Err(error) => panic!("stress lane task: {error}"),
        }
    }
    stop.store(true, Ordering::SeqCst);
    progress.abort();
    print_progress(
        "complete",
        completed.load(Ordering::SeqCst),
        total_iterations,
    );

    if !failures.is_empty() {
        let mut details = String::new();
        for failure in failures {
            details.push_str(&format!(
                "\nlane={} iteration={} retained_root={}\n{}\n",
                failure.lane,
                failure.iteration,
                failure.retained_root.display(),
                failure.message
            ));
        }
        panic!("Miniflare lease startup stress failed:{details}");
    }
}

async fn run_lane(
    lane: usize,
    iterations: usize,
    worker_ports: (u16, u16),
    inspector_ports: (u16, u16),
    stop: Arc<AtomicBool>,
    completed: Arc<AtomicUsize>,
) -> Result<(), StressFailure> {
    for iteration in 1..=iterations {
        if stop.load(Ordering::SeqCst) {
            return Ok(());
        }
        let harness = StressHarness::new(lane, worker_ports, inspector_ports);
        if let Err(message) = run_cycle(&harness, iteration).await {
            stop.store(true, Ordering::SeqCst);
            let retained_root = retain_temp_dir(harness.temp);
            return Err(StressFailure {
                lane,
                iteration,
                message,
                retained_root,
            });
        }
        completed.fetch_add(1, Ordering::SeqCst);
    }
    Ok(())
}

async fn report_progress(
    completed: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    total_iterations: usize,
    interval: Duration,
) {
    if interval.is_zero() {
        return;
    }
    let started = Instant::now();
    loop {
        tokio::time::sleep(interval).await;
        let completed = completed.load(Ordering::SeqCst);
        print_progress(
            &format!("elapsed={}s", started.elapsed().as_secs()),
            completed,
            total_iterations,
        );
        if stop.load(Ordering::SeqCst) || completed >= total_iterations {
            return;
        }
    }
}

fn print_progress(label: &str, completed: usize, total_iterations: usize) {
    let percent = if total_iterations == 0 {
        100.0
    } else {
        (completed as f64 / total_iterations as f64) * 100.0
    };
    eprintln!("miniflare_stress progress {label}: {completed}/{total_iterations} ({percent:.1}%)");
}

async fn run_cycle(harness: &StressHarness, iteration: usize) -> Result<(), String> {
    let pair = create_pair(harness, iteration).await?;

    let result = run_started_pair(harness, &pair).await;
    let cleanup_result = cleanup_pair(harness.router.clone(), &pair).await;
    match (result, cleanup_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(cleanup_error)) => Err(cleanup_error),
        (Err(error), Err(cleanup_error)) => {
            Err(format!("{error}\ncleanup failed:\n{cleanup_error}"))
        }
    }
}

async fn run_started_pair(harness: &StressHarness, pair: &LeasePair) -> Result<(), String> {
    let (alpha_restart, beta_restart) = tokio::join!(
        restart_lease(harness.router.clone(), &pair.alpha_id),
        restart_lease(harness.router.clone(), &pair.beta_id),
    );
    assert_restart_started(&alpha_restart, "alpha")?;
    assert_restart_started(&beta_restart, "beta")?;

    let (alpha_ready, beta_ready) = tokio::join!(
        wait_for_ready(harness.router.clone(), &pair.alpha_id),
        wait_for_ready(harness.router.clone(), &pair.beta_id),
    );
    let alpha_ready = alpha_ready?;
    let beta_ready = beta_ready?;

    let env_checks = async {
        assert_env(&alpha_ready, "ALPHA_TOKEN", "alpha-secret").await?;
        assert_env(&beta_ready, "BETA_TOKEN", "beta-secret").await?;
        assert_env(&alpha_ready, "BETA_TOKEN", "").await?;
        assert_env(&beta_ready, "ALPHA_TOKEN", "").await?;
        Ok::<(), String>(())
    }
    .await;
    if let Err(error) = env_checks {
        return Err(pair_failure_report(harness.router.clone(), pair, error).await);
    }

    Ok(())
}

async fn cleanup_pair(router: axum::Router, pair: &LeasePair) -> Result<(), String> {
    let (alpha_delete, beta_delete) = tokio::join!(
        delete_lease(router.clone(), &pair.alpha_id),
        delete_lease(router, &pair.beta_id),
    );
    let mut errors = Vec::new();
    if let Err(error) = assert_state(&alpha_delete, "stopped", "alpha delete") {
        errors.push(error);
    }
    if let Err(error) = assert_state(&beta_delete, "stopped", "beta delete") {
        errors.push(error);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

async fn create_pair(harness: &StressHarness, iteration: usize) -> Result<LeasePair, String> {
    let alpha_name = format!("stress-l{}-i{}-alpha", harness.lane, iteration);
    let beta_name = format!("stress-l{}-i{}-beta", harness.lane, iteration);
    let (alpha, beta) = tokio::join!(
        create_lease(
            harness.router.clone(),
            &alpha_name,
            serde_json::json!({ "ALPHA_TOKEN": "alpha-secret" }),
        ),
        create_lease(
            harness.router.clone(),
            &beta_name,
            serde_json::json!({ "BETA_TOKEN": "beta-secret" }),
        ),
    );
    let alpha_id = lease_id(&alpha, "alpha create")?;
    let beta_id = lease_id(&beta, "beta create")?;

    let (alpha_bundle, beta_bundle) = tokio::join!(
        bundle_lease(harness.router.clone(), &alpha_id),
        bundle_lease(harness.router.clone(), &beta_id),
    );
    assert_state(&alpha_bundle, "bundled", "alpha bundle")?;
    assert_state(&beta_bundle, "bundled", "beta bundle")?;

    Ok(LeasePair { alpha_id, beta_id })
}

async fn create_lease(
    router: axum::Router,
    name: &str,
    env_vars: serde_json::Value,
) -> serde_json::Value {
    request(
        router,
        Method::POST,
        "/leases",
        serde_json::json!({
            "name": name,
            "health_path": "/health",
            "env": "dev",
            "protocol": "http",
            "log_level": "warn",
            "backend": "miniflare",
            "env_vars": env_vars
        }),
    )
    .await
}

async fn bundle_lease(router: axum::Router, lease_id: &str) -> serde_json::Value {
    request(
        router,
        Method::POST,
        &format!("/leases/{lease_id}/bundle"),
        lease_bundle(),
    )
    .await
}

async fn restart_lease(router: axum::Router, lease_id: &str) -> serde_json::Value {
    request(
        router,
        Method::POST,
        &format!("/leases/{lease_id}/restart"),
        serde_json::json!({}),
    )
    .await
}

async fn delete_lease(router: axum::Router, lease_id: &str) -> serde_json::Value {
    request(
        router,
        Method::DELETE,
        &format!("/leases/{lease_id}"),
        serde_json::json!({}),
    )
    .await
}

async fn wait_for_ready(router: axum::Router, lease_id: &str) -> Result<serde_json::Value, String> {
    let mut last = None;
    for _ in 0..2000 {
        let lease = get_json(router.clone(), &format!("/leases/{lease_id}")).await;
        if lease["status"]["state"] == "ready" {
            return Ok(lease);
        }
        if lease["status"]["state"] == "failed" {
            return Err(lease_failure_report(router, lease_id, lease).await);
        }
        last = Some(lease);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let lease = last.unwrap_or(serde_json::Value::Null);
    let report = lease_failure_report(router, lease_id, lease).await;
    Err(report.replace("failed", "did not become ready"))
}

async fn lease_failure_report(
    router: axum::Router,
    lease_id: &str,
    lease: serde_json::Value,
) -> String {
    let debug = get_json(router.clone(), &format!("/leases/{lease_id}/debug")).await;
    let logs = get_text(router, &format!("/leases/{lease_id}/logs")).await;
    format!("lease {lease_id} failed\nlease={lease}\ndebug={debug}\nlogs:\n{logs}")
}

async fn pair_failure_report(router: axum::Router, pair: &LeasePair, message: String) -> String {
    let alpha = lease_snapshot(router.clone(), "alpha", &pair.alpha_id).await;
    let beta = lease_snapshot(router, "beta", &pair.beta_id).await;
    format!("{message}\n{alpha}\n{beta}")
}

async fn lease_snapshot(router: axum::Router, label: &str, lease_id: &str) -> String {
    let lease = get_json(router.clone(), &format!("/leases/{lease_id}")).await;
    let debug = get_json(router.clone(), &format!("/leases/{lease_id}/debug")).await;
    let logs = get_text(router, &format!("/leases/{lease_id}/logs")).await;
    format!("{label} lease {lease_id}\nlease={lease}\ndebug={debug}\nlogs:\n{logs}")
}

async fn get_json(router: axum::Router, uri: &str) -> serde_json::Value {
    request(router, Method::GET, uri, serde_json::json!({})).await
}

async fn get_text(router: axum::Router, uri: &str) -> String {
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    format!("status={status}\n{}", String::from_utf8_lossy(&body))
}

async fn assert_env(lease: &serde_json::Value, name: &str, expected: &str) -> Result<(), String> {
    let lease_id = lease["id"].as_str().unwrap_or("<missing-id>");
    let lease_name = lease["name"].as_str().unwrap_or("<missing-name>");
    let base_url = lease["base_url"].as_str().expect("base url");
    let url = format!("{base_url}/env?name={name}");
    let response = reqwest::get(&url)
        .await
        .map_err(|error| format!("env request {name} for {lease_id} at {url}: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("env body {name} for {lease_id} at {url}: {error}"))?;
    if body != expected {
        return Err(format!(
            "env {name} for lease {lease_id} ({lease_name}) at {url}: expected {expected:?}, got {body:?}, status={status}, port={}, inspector_port={}",
            lease["port"], lease["inspector_port"]
        ));
    }
    Ok(())
}

fn assert_restart_started(value: &serde_json::Value, context: &str) -> Result<(), String> {
    assert_state(value, "starting", context)?;
    if value["backend"] != "miniflare" {
        return Err(format!(
            "{context}: expected miniflare backend, got {value}"
        ));
    }
    Ok(())
}

fn assert_state(value: &serde_json::Value, state: &str, context: &str) -> Result<(), String> {
    if value["status"]["state"] != state {
        return Err(format!("{context}: expected state {state:?}, got {value}"));
    }
    Ok(())
}

fn lease_id(value: &serde_json::Value, context: &str) -> Result<String, String> {
    assert_state(value, "created", context)?;
    if value["backend"] != "miniflare" {
        return Err(format!(
            "{context}: expected miniflare backend, got {value}"
        ));
    }
    value["id"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("{context}: missing lease id in {value}"))
}

impl StressHarness {
    fn new(lane: usize, worker_ports: (u16, u16), inspector_ports: (u16, u16)) -> Self {
        let temp = make_temp_dir();
        let worker_bin = bin("worker-runtime-host-worker");
        let wrangler_bin = temp.path().join("bin/wrangler");
        make_executable(&wrangler_bin, &fake_wrangler_script());
        let config = worker_runtime_host_gen::service::DocsServiceConfig {
            bind: "127.0.0.1".to_string(),
            port: 8786,
            plan_file: temp.path().join("work/host/config/projects.plan.json"),
            host_root: temp.path().join("work/host"),
            lease_root: temp.path().join("work/host/leases"),
            lease_port_start: worker_ports.0,
            lease_port_end: worker_ports.1,
            lease_inspector_port_start: inspector_ports.0,
            lease_inspector_port_end: inspector_ports.1,
            worker_bin,
            wrangler_bin,
            failure_report_ttl_secs: 86_400,
            failure_report_max_entries: 100,
        };
        let router = worker_runtime_host_gen::docs::app(config, test_plan());
        Self { temp, router, lane }
    }
}

fn retain_temp_dir(temp: tempfile::TempDir) -> PathBuf {
    temp.keep()
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .map(|value| {
            value
                .parse()
                .unwrap_or_else(|error| panic!("{name}: {error}"))
        })
        .unwrap_or(default)
}
