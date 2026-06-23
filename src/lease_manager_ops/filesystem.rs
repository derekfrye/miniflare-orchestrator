use crate::lease_filesystem::snapshot_lease_filesystem;
use crate::lease_manager::{LeaseError, LeaseManager};
use crate::lease_model::LeaseFilesystemSnapshot;

impl LeaseManager {
    /// Returns a snapshot of the lease filesystem tree.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease does not exist or the filesystem cannot
    /// be read.
    pub async fn filesystem_snapshot(
        &self,
        id: &str,
    ) -> Result<LeaseFilesystemSnapshot, LeaseError> {
        let store = self.state.lock().await;
        let record = store.lease(id)?;
        snapshot_lease_filesystem(
            &record.runtime_dir,
            &record.static_dir,
            &record.state_dir,
            &record.log_dir,
        )
    }
}
