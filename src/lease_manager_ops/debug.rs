use crate::lease_manager::{LeaseError, LeaseManager, lease_debug_response};
use crate::lease_model::{LeaseDebugResponse, LeaseHealthProbeReport};
use crate::lease_runtime::probe_health_report_with_protocol;

impl LeaseManager {
    /// Returns the current lease debug snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease does not exist.
    pub async fn debug(&self, id: &str) -> Result<LeaseDebugResponse, LeaseError> {
        let store = self.state.lock().await;
        let record = store.lease(id)?;
        Ok(lease_debug_response(record))
    }

    /// Runs the current lease health probe and returns the full request and
    /// response details.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease does not exist or the probe client cannot
    /// be constructed.
    pub async fn probe(&self, id: &str) -> Result<LeaseHealthProbeReport, LeaseError> {
        let (port, health_path, protocol) = {
            let store = self.state.lock().await;
            let record = store.lease(id)?;
            (
                record.port,
                record.health_path.clone(),
                record.protocol.clone(),
            )
        };

        let report = probe_health_report_with_protocol(port, &health_path, &protocol).await?;
        let mut store = self.state.lock().await;
        if let Some(record) = store.leases.get_mut(id) {
            record.last_probe = Some(report.clone());
        }
        Ok(report)
    }
}
