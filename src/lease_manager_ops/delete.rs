use crate::lease_logs::tail_lease_logs;
use crate::lease_manager::LeaseError;
use crate::lease_manager::LeaseManager;
use crate::lease_manager::{lease_failure_report, lease_is_failed};
use crate::lease_model::{LeaseResponse, LeaseState, LeaseStatus};
use crate::secure_fs;

impl LeaseManager {
    /// Stops and deletes a lease.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease does not exist.
    pub async fn delete(&self, id: &str) -> Result<LeaseResponse, LeaseError> {
        let (child, startup_permit, snapshot, failure_report) = {
            let mut store = self.state.lock().await;
            let record = store
                .leases
                .get_mut(id)
                .ok_or_else(|| LeaseError::unknown_lease(id))?;
            let failure_report = if lease_is_failed(record) || record.startup_diagnostics.is_some()
            {
                let log_tail = tail_lease_logs(&record.log_dir, 200).ok();
                Some(lease_failure_report(record, log_tail))
            } else {
                None
            };

            record.generation = record.generation.saturating_add(1);
            record.status = LeaseStatus::new(LeaseState::Stopped);
            let snapshot = crate::lease_manager::lease_response(record);
            (
                record.child.take(),
                record.startup_permit.take(),
                snapshot,
                failure_report,
            )
        };

        if let Some(report) = failure_report {
            let mut store = self.state.lock().await;
            self.retain_failure_report(&mut store, id.to_string(), report);
        }

        self.kill_child(child).await;
        drop(startup_permit);
        let cleanup_result = crate::lease_manager::wait_for_ports_available(
            snapshot.port,
            snapshot.inspector_port,
            std::time::Duration::from_secs(5),
        )
        .await
        .map_err(LeaseError::unavailable)
        .and_then(|()| {
            let lease_root = secure_fs::open_ambient_dir(&self.config.lease_root)?;
            match lease_root.remove_dir_all(id) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.into()),
            }
        });

        let mut store = self.state.lock().await;
        store.leases.remove(id);
        drop(store);

        cleanup_result.map(|()| snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lease_manager::LeaseManagerConfig;
    use crate::lease_model::{LeaseBackend, LeaseCreateRequest, LeaseRestartRequest};
    use crate::lease_runtime::{LeaseLaunchConfig, LeaseRuntimeConfig, LeaseSpawner};
    use std::collections::BTreeMap;
    use std::future::Future;
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::sync::Arc;
    use tokio::process::{Child, Command};
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn create_waits_for_delete_cleanup_before_reusing_ports() {
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
            spawner: Arc::new(SleepingSpawner),
        });

        let lease = manager
            .create(LeaseCreateRequest {
                name: Some("deleting".to_string()),
                health_path: Some("/health".to_string()),
                env: Some("dev".to_string()),
                protocol: Some("http".to_string()),
                log_level: Some("warn".to_string()),
                env_vars: BTreeMap::new(),
                persist_state: false,
                backend: LeaseBackend::WranglerDev,
            })
            .await
            .expect("create lease");
        manager
            .restart(
                &lease.id,
                LeaseRestartRequest {
                    persist_state: None,
                    backend: None,
                },
            )
            .await
            .expect("restart lease");

        let deleting_manager = manager.clone();
        let deleting_id = lease.id.clone();
        let delete = tokio::spawn(async move { deleting_manager.delete(&deleting_id).await });
        sleep(Duration::from_millis(50)).await;

        let create_while_delete_is_reaping = manager
            .create(LeaseCreateRequest {
                name: Some("replacement".to_string()),
                health_path: Some("/health".to_string()),
                env: Some("dev".to_string()),
                protocol: Some("http".to_string()),
                log_level: Some("warn".to_string()),
                env_vars: BTreeMap::new(),
                persist_state: false,
                backend: LeaseBackend::WranglerDev,
            })
            .await;
        let replacement = create_while_delete_is_reaping
            .expect("create should wait for delete cleanup before reusing ports");

        delete.await.expect("delete task").expect("delete lease");
        assert_eq!(replacement.port, worker_port);
        assert_eq!(replacement.inspector_port, inspector_port);
    }

    #[tokio::test]
    async fn delete_failure_does_not_leak_reserved_ports() {
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
            spawner: Arc::new(SleepingSpawner),
        });

        let lease = manager
            .create(LeaseCreateRequest {
                name: Some("deleting".to_string()),
                health_path: Some("/health".to_string()),
                env: Some("dev".to_string()),
                protocol: Some("http".to_string()),
                log_level: Some("warn".to_string()),
                env_vars: BTreeMap::new(),
                persist_state: false,
                backend: LeaseBackend::WranglerDev,
            })
            .await
            .expect("create lease");

        let inspector_listener =
            TcpListener::bind(("127.0.0.1", inspector_port)).expect("bind inspector port");
        let error = manager
            .delete(&lease.id)
            .await
            .expect_err("delete should fail while inspector port is externally occupied");
        assert!(
            error
                .to_string()
                .contains("startup ports did not become available after worker cleanup"),
            "unexpected delete error: {error}"
        );
        drop(inspector_listener);

        manager
            .create(LeaseCreateRequest {
                name: Some("replacement".to_string()),
                health_path: Some("/health".to_string()),
                env: Some("dev".to_string()),
                protocol: Some("http".to_string()),
                log_level: Some("warn".to_string()),
                env_vars: BTreeMap::new(),
                persist_state: false,
                backend: LeaseBackend::WranglerDev,
            })
            .await
            .expect("ports should be reusable after a failed delete once the OS releases them");
    }

    #[derive(Debug)]
    struct SleepingSpawner;

    impl LeaseSpawner for SleepingSpawner {
        fn spawn_worker<'a>(
            &'a self,
            _config: &'a LeaseRuntimeConfig,
            _launch: &'a LeaseLaunchConfig,
        ) -> Pin<Box<dyn Future<Output = Result<Child, LeaseError>> + Send + 'a>> {
            Box::pin(async move {
                Command::new("sleep")
                    .arg("30")
                    .spawn()
                    .map_err(LeaseError::from)
            })
        }
    }
}
