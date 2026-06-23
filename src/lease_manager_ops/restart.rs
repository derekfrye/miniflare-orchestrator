#[path = "restart/launch_helpers.rs"]
mod launch_helpers;

use crate::lease_manager::{LeaseError, LeaseManager, LeaseStore};
use crate::lease_model::LeaseLaunchDetails;
use crate::lease_model::{
    LeaseBundleDiagnosticKind, LeaseResponse, LeaseRestartRequest, LeaseStartupDiagnosticKind,
    LeaseStartupDiagnostics, LeaseState, LeaseStatus,
};
use crate::lease_runtime::{LeaseLaunchConfig, LeaseRuntimeConfig};
use std::path::PathBuf;
use tokio::process::Child;
use tokio::sync::OwnedSemaphorePermit;

struct RestartSnapshot {
    old_child: Option<Child>,
    old_startup_permit: Option<OwnedSemaphorePermit>,
    response: LeaseResponse,
    generation: u64,
    launch: LeaseLaunchConfig,
}

pub(crate) struct SpawnedLeaseWorker {
    pub(crate) child: Child,
    pub(crate) startup_permit: OwnedSemaphorePermit,
}

impl LeaseManager {
    /// Starts or restarts the lease worker.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease does not exist, the worker binary cannot
    /// be launched, or the startup probe fails.
    pub async fn restart(
        &self,
        id: &str,
        request: LeaseRestartRequest,
    ) -> Result<LeaseResponse, LeaseError> {
        let restart = {
            let mut store = self.state.lock().await;
            prepare_restart(&mut store, id, request)?
        };

        self.kill_child(restart.old_child).await;
        drop(restart.old_startup_permit);

        let startup_details = self.startup_details(&restart.launch);
        let worker = match self.spawn_worker_with_startup_permit(&restart.launch).await {
            Ok(worker) => worker,
            Err(error) => {
                self.record_spawn_error(id, restart.generation, &startup_details, &error)
                    .await;
                return Err(error);
            }
        };

        {
            let mut store = self.state.lock().await;
            let record = store.lease_mut(id)?;
            record.startup_details = Some(startup_details);
            record.child = Some(worker.child);
            record.startup_permit = Some(worker.startup_permit);
        }

        let manager = self.clone();
        let id = id.to_string();
        let generation = restart.generation;
        let launch = restart.launch;
        tokio::spawn(async move {
            crate::lease_manager_monitor::monitor_lease(manager, id, generation, launch).await;
        });

        Ok(restart.response)
    }

    pub(crate) fn startup_details(&self, launch: &LeaseLaunchConfig) -> LeaseLaunchDetails {
        launch_helpers::startup_details(launch, &self.config.worker_bin, &self.config.wrangler_bin)
    }

    pub(crate) async fn spawn_worker_with_startup_permit(
        &self,
        launch: &LeaseLaunchConfig,
    ) -> Result<SpawnedLeaseWorker, LeaseError> {
        let startup_permit = self.acquire_startup_permit().await;
        self.spawn_worker_with_existing_startup_permit(launch, Some(startup_permit))
            .await
    }

    pub(crate) async fn spawn_worker_with_existing_startup_permit(
        &self,
        launch: &LeaseLaunchConfig,
        startup_permit: Option<OwnedSemaphorePermit>,
    ) -> Result<SpawnedLeaseWorker, LeaseError> {
        let startup_permit = match startup_permit {
            Some(startup_permit) => startup_permit,
            None => self.acquire_startup_permit().await,
        };
        let config = LeaseRuntimeConfig {
            worker_bin: self.config.worker_bin.clone(),
            wrangler_bin: self.config.wrangler_bin.clone(),
        };
        let child = self.config.spawner.spawn_worker(&config, launch).await?;
        Ok(SpawnedLeaseWorker {
            child,
            startup_permit,
        })
    }

    async fn record_spawn_error(
        &self,
        id: &str,
        generation: u64,
        startup_details: &LeaseLaunchDetails,
        error: &LeaseError,
    ) {
        let mut store = self.state.lock().await;
        if let Some(record) = store.leases.get_mut(id)
            && record.generation == generation
        {
            record.startup_details = Some(startup_details.clone());
            record.startup_permit = None;
            record.startup_diagnostics = Some(LeaseStartupDiagnostics {
                kind: LeaseStartupDiagnosticKind::ProbeError,
                message: Some(error.to_string()),
                suggested_protocol: None,
            });
            record.status = LeaseStatus::failed(error.to_string());
        }
    }
}

fn prepare_restart(
    store: &mut LeaseStore,
    id: &str,
    request: LeaseRestartRequest,
) -> Result<RestartSnapshot, LeaseError> {
    let record = store.lease_mut(id)?;
    if let Some(persist_state) = request.persist_state {
        record.persist_state = persist_state;
    }
    if let Some(backend) = request.backend {
        record.backend = backend;
    }
    record.generation = record.generation.saturating_add(1);
    let old_child = record.child.take();
    let old_startup_permit = record.startup_permit.take();
    record.status = LeaseStatus::new(LeaseState::Starting);
    record.startup_diagnostics = None;
    record.last_probe = None;
    if let Some(stale_bundle) = crate::lease_manager::bundle_diagnostics(record)
        .into_iter()
        .find(|diagnostic| {
            matches!(
                diagnostic.kind,
                LeaseBundleDiagnosticKind::PossiblyStaleBundle
            )
        })
    {
        record.startup_diagnostics = Some(LeaseStartupDiagnostics {
            kind: LeaseStartupDiagnosticKind::PossiblyStaleBundle,
            message: Some(stale_bundle.message),
            suggested_protocol: None,
        });
    }
    let launch = launch_config(record.runtime_dir.join("wrangler.toml"), record);
    Ok(RestartSnapshot {
        old_child,
        old_startup_permit,
        response: crate::lease_manager::lease_response(record),
        generation: record.generation,
        launch,
    })
}

fn launch_config(
    config_file: PathBuf,
    record: &crate::lease_manager::LeaseRecord,
) -> LeaseLaunchConfig {
    LeaseLaunchConfig {
        runtime_dir: record.runtime_dir.clone(),
        static_dir: record.static_dir.clone(),
        state_dir: record.state_dir.clone(),
        log_dir: record.log_dir.clone(),
        config_file,
        port: record.port,
        inspector_port: record.inspector_port,
        env_name: record.env.clone(),
        protocol: record.protocol.clone(),
        log_level: record.log_level.clone(),
        env_vars: record.env_vars.clone(),
        persist_state: record.persist_state,
        backend: record.backend,
    }
}
