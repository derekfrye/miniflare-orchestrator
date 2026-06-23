use serde_json::{Value, json};

use crate::lease_manager::HEALTH_CHECK_TIMED_OUT;

pub(super) fn create_request() -> Value {
    json!({
        "name": "alpha-test",
        "health_path": "/health",
        "env": "dev",
        "protocol": "http",
        "log_level": "warn",
        "env_vars": {
            "API_URL": "https://example.invalid"
        },
        "persist_state": true,
        "backend": "miniflare"
    })
}

pub(super) fn lease_response(state: &str, persist_state: bool) -> Value {
    json!({
        "id": "<lease-id>",
        "name": "alpha-test",
        "port": 8900,
        "inspector_port": 9000,
        "base_url": "http://127.0.0.1:8900",
        "health_url": "http://127.0.0.1:8900/health",
        "status": { "state": state },
        "runtime_dir": "/work/host/leases/<lease-id>/runtime",
        "static_dir": "/work/host/leases/<lease-id>/static",
        "state_dir": "/work/host/leases/<lease-id>/state",
        "log_dir": "/work/host/leases/<lease-id>/logs",
        "health_path": "/health",
        "env": "dev",
        "protocol": "http",
        "log_level": "warn",
        "env_vars": {
            "API_URL": "https://example.invalid"
        },
        "persist_state": persist_state,
        "backend": "miniflare",
        "prebuilt_bundle_notice": prebuilt_bundle_notice()
    })
}

pub(super) fn failed_lease_response() -> Value {
    json!({
        "id": "<lease-id>",
        "name": "alpha-test",
        "port": 8900,
        "inspector_port": 9000,
        "base_url": "http://127.0.0.1:8900",
        "health_url": "http://127.0.0.1:8900/health",
        "status": { "state": "failed", "message": HEALTH_CHECK_TIMED_OUT },
        "runtime_dir": "/work/host/leases/<lease-id>/runtime",
        "static_dir": "/work/host/leases/<lease-id>/static",
        "state_dir": "/work/host/leases/<lease-id>/state",
        "log_dir": "/work/host/leases/<lease-id>/logs",
        "health_path": "/health",
        "env": "dev",
        "protocol": "http",
        "log_level": "warn",
        "env_vars": {
            "API_URL": "https://example.invalid"
        },
        "persist_state": true,
        "backend": "miniflare",
        "prebuilt_bundle_notice": prebuilt_bundle_notice(),
        "startup_diagnostics": {
            "kind": "health_check_timed_out",
            "message": HEALTH_CHECK_TIMED_OUT
        }
    })
}

pub(super) fn startup_details() -> Value {
    json!({
        "backend": "miniflare",
        "worker_bin": "/usr/local/bin/worker-runtime-host-worker",
        "wrangler_bin": "wrangler",
        "config_file": "/work/host/leases/<lease-id>/runtime/wrangler.toml",
        "runtime_dir": "/work/host/leases/<lease-id>/runtime",
        "static_dir": "/work/host/leases/<lease-id>/static",
        "state_dir": "/work/host/leases/<lease-id>/state",
        "log_dir": "/work/host/leases/<lease-id>/logs",
        "port": 8900,
        "inspector_port": 9000,
        "env": "dev",
        "protocol": "http",
        "log_level": "warn",
        "persist_state": true,
        "env_vars": {
            "API_URL": "https://example.invalid"
        },
        "injected_env": injected_env(),
        "effective_bindings": {
            "backend": "miniflare",
            "env": "dev",
            "vars": ["API_URL"]
        }
    })
}

fn prebuilt_bundle_notice() -> Value {
    json!({
        "message": "Lease backends serve the uploaded prebuilt bundle only. Wrangler build hooks are skipped because workers run with prebuilt artifacts; rebuild worker artifacts and POST /leases/{id}/bundle again after changing Worker source.",
        "required_action": "Rebuild Worker artifacts and upload a fresh bundle before restarting or rerunning tests after Worker source changes.",
        "applies_to": [
            "POST /leases/{id}/bundle",
            "POST /leases/{id}/restart",
            "GET /leases/{id}/debug",
            "GET /leases/{id}/failure-report"
        ]
    })
}

pub(super) fn healthy_probe() -> Value {
    probe("healthy", 200)
}

pub(super) fn unhealthy_probe() -> Value {
    probe("unhealthy", 500)
}

fn probe(outcome: &str, status_code: u16) -> Value {
    json!({
        "request_url": "http://127.0.0.1:8900/health",
        "request_method": "GET",
        "protocol": "http",
        "health_path": "/health",
        "outcome": outcome,
        "status_code": status_code,
        "headers": {
            "content-type": "text/plain"
        }
    })
}

fn injected_env() -> Value {
    json!({
        "HOME": "/work/host/leases/<lease-id>/state",
        "PATH": "/usr/local/bin:/usr/bin:/bin",
        "TMPDIR": "/work/host/leases/<lease-id>/state",
        "WORKER_RUNTIME_HOST_CONFIG_FILE": "/work/host/leases/<lease-id>/runtime/wrangler.toml",
        "WORKER_RUNTIME_HOST_ENV": "dev",
        "WORKER_RUNTIME_HOST_LOG_DIR": "/work/host/leases/<lease-id>/logs",
        "WORKER_RUNTIME_HOST_LOG_LEVEL": "warn",
        "WORKER_RUNTIME_HOST_INSPECTOR_PORT": "9000",
        "WORKER_RUNTIME_HOST_PORT": "8900",
        "WORKER_RUNTIME_HOST_PROTOCOL": "http",
        "WORKER_RUNTIME_HOST_RUNTIME_DIR": "/work/host/leases/<lease-id>/runtime",
        "WORKER_RUNTIME_HOST_STATE_DIR": "/work/host/leases/<lease-id>/state",
        "WORKER_RUNTIME_HOST_WRANGLER_BIN": "wrangler",
        "WORKER_RUNTIME_HOST_NODE_BIN": "node",
        "API_URL": "https://example.invalid"
    })
}
