#[path = "contract/diagnostics.rs"]
mod diagnostics;
#[path = "contract/examples.rs"]
mod examples;
#[path = "contract/filesystem_logs.rs"]
mod filesystem_logs;
#[path = "contract/lifecycle.rs"]
mod lifecycle;

use super::ClientContractExample;

pub fn build_client_contract() -> Vec<ClientContractExample> {
    vec![
        lifecycle::create_lease_example(),
        lifecycle::bundle_runtime_files_example(),
        lifecycle::restart_lease_example(),
        diagnostics::debug_lease_example(),
        diagnostics::failure_report_example(),
        diagnostics::probe_lease_example(),
        filesystem_logs::filesystem_snapshot_example(),
        filesystem_logs::fetch_lease_logs_example(),
        lifecycle::inspect_lease_example(),
        lifecycle::delete_lease_example(),
    ]
}
