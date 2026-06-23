use crate::lease_logs::tail_lease_logs;
use crate::lease_manager::{LeaseError, LeaseManager, lease_failure_report, lease_is_failed};
use crate::lease_model::LeaseFailureReport;

impl LeaseManager {
    /// Returns a retained failure snapshot for one lease.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease never failed or no retained report exists.
    pub async fn failure_report(&self, id: &str) -> Result<LeaseFailureReport, LeaseError> {
        {
            self.prune_failure_reports().await;
            let store = self.state.lock().await;
            if let Some(record) = store.leases.get(id)
                && (lease_is_failed(record) || record.startup_diagnostics.is_some())
            {
                return Ok(lease_failure_report(
                    record,
                    tail_lease_logs(&record.log_dir, 200).ok(),
                ));
            }
            if let Some(report) = store.failure_reports.get(id) {
                return Ok(report.report.clone());
            }
        }

        Err(LeaseError::unknown_lease(id))
    }
}
