use crate::lease_bundle::write_bundle_files;
use crate::lease_manager::LeaseRecord;
use crate::lease_manager::{LeaseError, LeaseManager};
use crate::lease_model::{
    LeaseBundleRequest, LeaseCreateRequest, LeaseResponse, LeaseState, LeaseStatus,
};
use crate::secure_fs;
use std::path::Path;
use uuid::Uuid;

impl LeaseManager {
    /// Creates a new ephemeral lease.
    ///
    /// # Errors
    ///
    /// Returns an error if no port is available or the lease directories
    /// cannot be created.
    pub async fn create(&self, request: LeaseCreateRequest) -> Result<LeaseResponse, LeaseError> {
        let id = Uuid::new_v4().to_string();
        let name = request.name.unwrap_or_else(|| format!("lease-{id}"));
        let paths = self.paths_for(&id);
        let protocol = validate_protocol(request.protocol)?;

        secure_fs::create_ambient_private_dir_all(&self.config.lease_root)?;
        let lease_root = secure_fs::open_ambient_dir(&self.config.lease_root)?;
        let lease_dir = secure_fs::create_private_dir_all(&lease_root, Path::new(&id))?;
        secure_fs::create_private_dir_all(&lease_dir, Path::new("runtime"))?;
        secure_fs::create_private_dir_all(&lease_dir, Path::new("static"))?;
        secure_fs::create_private_dir_all(&lease_dir, Path::new("state"))?;
        secure_fs::create_private_dir_all(&lease_dir, Path::new("logs"))?;

        self.insert_with_allocated_ports(&id, id.clone(), |port, inspector_port| LeaseRecord {
            id: id.clone(),
            name: name.clone(),
            port,
            inspector_port,
            runtime_dir: paths.runtime.clone(),
            static_dir: paths.static_assets.clone(),
            state_dir: paths.state.clone(),
            log_dir: paths.logs.clone(),
            health_path: request
                .health_path
                .clone()
                .unwrap_or_else(|| "/health".to_string()),
            env: request.env.clone().unwrap_or_else(|| "dev".to_string()),
            protocol: protocol.clone(),
            log_level: request
                .log_level
                .clone()
                .unwrap_or_else(|| "warn".to_string()),
            env_vars: request.env_vars.clone(),
            persist_state: request.persist_state,
            backend: request.backend,
            bundle_metadata: None,
            bundle_uploaded_at: None,
            startup_diagnostics: None,
            startup_details: None,
            last_probe: None,
            status: LeaseStatus::new(LeaseState::Created),
            generation: 0,
            child: None,
            startup_permit: None,
        })
        .await
    }

    /// Returns the current snapshot for a lease.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease does not exist.
    pub async fn get(&self, id: &str) -> Result<LeaseResponse, LeaseError> {
        let store = self.state.lock().await;
        let record = store.lease(id)?;
        Ok(crate::lease_manager::lease_response(record))
    }

    /// Writes a runtime bundle into the lease directories.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease does not exist or the bundle contents
    /// are invalid.
    pub async fn bundle(
        &self,
        id: &str,
        request: LeaseBundleRequest,
    ) -> Result<LeaseResponse, LeaseError> {
        let mut store = self.state.lock().await;
        let record = store.lease_mut(id)?;
        write_bundle_files(&record.runtime_dir, &record.static_dir, &request)?;
        record.bundle_metadata = request.metadata;
        record.bundle_uploaded_at = Some(std::time::SystemTime::now());
        record.status = LeaseStatus::new(LeaseState::Bundled);
        Ok(crate::lease_manager::lease_response(record))
    }
}

fn validate_protocol(protocol: Option<String>) -> Result<String, LeaseError> {
    let protocol = protocol.unwrap_or_else(|| "http".to_string());
    match protocol.as_str() {
        "http" | "https" => Ok(protocol),
        _ => Err(LeaseError::usage(format!(
            "unsupported protocol: {protocol}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lease_manager::LeaseManagerConfig;
    use crate::lease_model::LeaseBackend;
    use crate::lease_runtime::real_lease_spawner;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn create_retries_until_ports_become_available() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (worker_port, reserved_ports) = crate::test_ports::reserved_port_block(2);
        let inspector_port = worker_port + 1;
        let manager = LeaseManager::new(LeaseManagerConfig {
            lease_root: temp.path().join("leases"),
            worker_bin: PathBuf::from("/tmp/fake-worker"),
            wrangler_bin: PathBuf::from("/tmp/fake-wrangler"),
            port_start: worker_port,
            port_end: worker_port,
            inspector_port_start: inspector_port,
            inspector_port_end: inspector_port,
            failure_report_ttl_secs: 86_400,
            failure_report_max_entries: 100,
            spawner: real_lease_spawner(),
        });

        tokio::spawn(async move {
            sleep(Duration::from_millis(150)).await;
            drop(reserved_ports);
        });

        manager
            .create(LeaseCreateRequest {
                name: Some("retrying".to_string()),
                health_path: Some("/health".to_string()),
                env: Some("dev".to_string()),
                protocol: Some("http".to_string()),
                log_level: Some("warn".to_string()),
                env_vars: BTreeMap::new(),
                persist_state: false,
                backend: LeaseBackend::WranglerDev,
            })
            .await
            .expect("create should retry until the single port pair is available");
    }

    #[tokio::test]
    async fn concurrent_creates_do_not_allocate_duplicate_ports() {
        let temp = tempfile::tempdir().expect("tempdir");
        let worker_port = crate::test_ports::available_port_block(2);
        let inspector_port = worker_port + 1;
        let manager = LeaseManager::new(LeaseManagerConfig {
            lease_root: temp.path().join("leases"),
            worker_bin: PathBuf::from("/tmp/fake-worker"),
            wrangler_bin: PathBuf::from("/tmp/fake-wrangler"),
            port_start: worker_port,
            port_end: worker_port,
            inspector_port_start: inspector_port,
            inspector_port_end: inspector_port,
            failure_report_ttl_secs: 86_400,
            failure_report_max_entries: 100,
            spawner: real_lease_spawner(),
        });

        let (alpha, beta) = tokio::join!(
            manager.create(test_create_request("alpha")),
            manager.create(test_create_request("beta")),
        );
        let successes = [&alpha, &beta]
            .iter()
            .filter(|result| result.is_ok())
            .count();
        let failures = [&alpha, &beta]
            .iter()
            .filter(|result| result.is_err())
            .count();

        assert_eq!(successes, 1);
        assert_eq!(failures, 1);
    }

    fn test_create_request(name: &str) -> LeaseCreateRequest {
        LeaseCreateRequest {
            name: Some(name.to_string()),
            health_path: Some("/health".to_string()),
            env: Some("dev".to_string()),
            protocol: Some("http".to_string()),
            log_level: Some("warn".to_string()),
            env_vars: BTreeMap::new(),
            persist_state: false,
            backend: LeaseBackend::WranglerDev,
        }
    }
}
