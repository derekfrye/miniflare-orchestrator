use crate::lease_manager::LeaseError;
use crate::lease_model::{
    LeaseBackend, LeaseBundleMetadata, LeaseFailureReport, LeaseHealthProbeReport,
    LeaseLaunchDetails, LeaseStartupDiagnostics, LeaseStatus,
};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::process::Child;
use tokio::sync::OwnedSemaphorePermit;

#[derive(Debug)]
pub(crate) struct LeaseStore {
    pub(crate) leases: HashMap<String, LeaseRecord>,
    pub(crate) failure_reports: HashMap<String, RetainedFailureReport>,
    pub(crate) next_failure_report_sequence: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct RetainedFailureReport {
    pub(crate) report: LeaseFailureReport,
    pub(crate) retained_at_unix_secs: u64,
    pub(crate) retained_sequence: u64,
}

#[derive(Debug)]
pub(crate) struct LeaseRecord {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) port: u16,
    pub(crate) inspector_port: u16,
    pub(crate) runtime_dir: PathBuf,
    pub(crate) static_dir: PathBuf,
    pub(crate) state_dir: PathBuf,
    pub(crate) log_dir: PathBuf,
    pub(crate) health_path: String,
    pub(crate) env: String,
    pub(crate) protocol: String,
    pub(crate) log_level: String,
    pub(crate) env_vars: BTreeMap<String, String>,
    pub(crate) persist_state: bool,
    pub(crate) backend: LeaseBackend,
    pub(crate) bundle_metadata: Option<LeaseBundleMetadata>,
    pub(crate) bundle_uploaded_at: Option<SystemTime>,
    pub(crate) startup_diagnostics: Option<LeaseStartupDiagnostics>,
    pub(crate) startup_details: Option<LeaseLaunchDetails>,
    pub(crate) last_probe: Option<LeaseHealthProbeReport>,
    pub(crate) status: LeaseStatus,
    pub(crate) generation: u64,
    pub(crate) child: Option<Child>,
    pub(crate) startup_permit: Option<OwnedSemaphorePermit>,
}

#[derive(Debug)]
pub(crate) struct LeasePaths {
    pub(crate) runtime: PathBuf,
    pub(crate) static_assets: PathBuf,
    pub(crate) state: PathBuf,
    pub(crate) logs: PathBuf,
}

impl LeaseStore {
    pub(crate) fn lease(&self, id: &str) -> Result<&LeaseRecord, LeaseError> {
        self.leases
            .get(id)
            .ok_or_else(|| LeaseError::unknown_lease(id))
    }

    pub(crate) fn lease_mut(&mut self, id: &str) -> Result<&mut LeaseRecord, LeaseError> {
        self.leases
            .get_mut(id)
            .ok_or_else(|| LeaseError::unknown_lease(id))
    }
}
