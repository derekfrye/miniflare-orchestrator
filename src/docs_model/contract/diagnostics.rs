use serde_json::json;

use super::examples;
use crate::docs_model::ClientContractExample;

pub(super) fn debug_lease_example() -> ClientContractExample {
    ClientContractExample {
        title: "Inspect lease debug data".to_string(),
        purpose:
            "Read the stored launch metadata and the last recorded probe result for one lease."
                .to_string(),
        method: "GET".to_string(),
        path: "/leases/{id}/debug".to_string(),
        request_example: json!(null),
        response_example: json!({
            "lease": examples::lease_response("ready", true),
            "startup": examples::startup_details(),
            "last_probe": examples::healthy_probe()
        }),
    }
}

pub(super) fn failure_report_example() -> ClientContractExample {
    ClientContractExample {
        title: "Inspect retained failure data".to_string(),
        purpose: "Read the preserved startup metadata, probe result, and log tail after a failed lease has been deleted. Reports are retained for up to 1 day by default and the host keeps at most 100 deleted failure reports unless configured otherwise.".to_string(),
        method: "GET".to_string(),
        path: "/leases/{id}/failure-report".to_string(),
        request_example: json!(null),
        response_example: json!({
            "lease": examples::failed_lease_response(),
            "startup": examples::startup_details(),
            "last_probe": examples::unhealthy_probe(),
            "log_tail": "..."
        }),
    }
}

pub(super) fn probe_lease_example() -> ClientContractExample {
    ClientContractExample {
        title: "Probe the lease health endpoint".to_string(),
        purpose: "Run the current health check and capture the raw request and response details."
            .to_string(),
        method: "GET".to_string(),
        path: "/leases/{id}/probe".to_string(),
        request_example: json!(null),
        response_example: examples::healthy_probe(),
    }
}
