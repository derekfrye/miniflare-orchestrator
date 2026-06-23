use serde_json::json;

use crate::docs_model::ClientContractExample;

pub(super) fn filesystem_snapshot_example() -> ClientContractExample {
    ClientContractExample {
        title: "Inspect lease filesystem snapshot".to_string(),
        purpose: "Read the runtime, static, state, and log filesystem tree for one lease."
            .to_string(),
        method: "GET".to_string(),
        path: "/leases/{id}/filesystem-snapshot".to_string(),
        request_example: json!(null),
        response_example: json!({
            "runtime_dir": "/work/host/leases/<lease-id>/runtime",
            "static_dir": "/work/host/leases/<lease-id>/static",
            "state_dir": "/work/host/leases/<lease-id>/state",
            "log_dir": "/work/host/leases/<lease-id>/logs",
            "entries": [
                {
                    "root": "runtime",
                    "path": "wrangler.toml",
                    "kind": "file",
                    "size": 18,
                    "modified": "1713878400.000Z"
                },
                {
                    "root": "runtime",
                    "path": "worker_entry.mjs",
                    "kind": "file",
                    "size": 22,
                    "modified": "1713878400.000Z"
                }
            ]
        }),
    }
}

pub(super) fn fetch_lease_logs_example() -> ClientContractExample {
    ClientContractExample {
        title: "Fetch lease logs".to_string(),
        purpose: "Read the log files for one lease without exposing other lease logs.".to_string(),
        method: "GET".to_string(),
        path: "/leases/{id}/logs".to_string(),
        request_example: json!(null),
        response_example: json!("== stdout.log ==\n..."),
    }
}
