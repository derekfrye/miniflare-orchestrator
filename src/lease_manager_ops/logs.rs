use crate::lease_logs::{read_lease_logs, tail_lease_logs};
use crate::lease_manager::LeaseError;
use crate::lease_manager::LeaseManager;

impl LeaseManager {
    /// Returns the current log contents for a lease.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease does not exist or the log files cannot
    /// be read.
    pub async fn logs(&self, id: &str) -> Result<String, LeaseError> {
        let store = self.state.lock().await;
        let record = store.lease(id)?;
        read_lease_logs(&record.log_dir)
    }

    /// Returns the trailing log lines for a lease.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease does not exist or the log files cannot
    /// be read.
    pub async fn tail_logs(&self, id: &str, max_lines: usize) -> Result<String, LeaseError> {
        let store = self.state.lock().await;
        let record = store.lease(id)?;
        tail_lease_logs(&record.log_dir, max_lines)
    }
}
