use std::collections::HashSet;
use std::path::PathBuf;

use serde_json::Value;

use super::lease_api_fixtures::SpawnCall;
use worker_runtime_host_gen::lease_model::LeaseBackend;

pub fn assert_spawn_calls(calls: &[SpawnCall], worker_bin: &PathBuf, wrangler_bin: &PathBuf) {
    assert_eq!(calls.len(), 2);
    let ports: HashSet<u16> = calls.iter().map(|call| call.port).collect();
    let inspector_ports: HashSet<u16> = calls.iter().map(|call| call.inspector_port).collect();
    assert_eq!(ports.len(), 2);
    assert_eq!(inspector_ports.len(), 2);
    assert!(ports.is_disjoint(&inspector_ports));
    assert!(calls.iter().any(|call| call.worker_bin == *worker_bin));
    assert!(calls.iter().any(|call| call.wrangler_bin == *wrangler_bin));
    assert!(calls.iter().any(|call| call.static_dir.ends_with("static")));
    assert!(calls.iter().any(|call| call.env_name == "dev"
        && call.protocol == "http"
        && call.log_level == "warn"
        && call.persist_state
        && call.backend == LeaseBackend::Miniflare));
    assert!(calls.iter().any(|call| call.env_vars.is_empty()));
}

pub fn assert_lease_artifacts(
    response: &Value,
    runtime_body: &str,
    static_body: &str,
    lease_name: &str,
) {
    let runtime_dir = PathBuf::from(response["runtime_dir"].as_str().expect("runtime dir"));
    let static_dir = PathBuf::from(response["static_dir"].as_str().expect("static dir"));

    assert_eq!(
        std::fs::read_to_string(runtime_dir.join("wrangler.toml")).expect("wrangler"),
        format!("name = \"{lease_name}\"\n")
    );
    assert_eq!(
        std::fs::read_to_string(runtime_dir.join("worker_entry.mjs")).expect("worker"),
        runtime_body
    );
    assert_eq!(
        std::fs::read_to_string(static_dir.join("index.html")).expect("static file"),
        static_body
    );
}
