use crate::lease_manager::{LeaseError, LeaseManager, LeaseRecord, LeaseStore};
use crate::lease_model::LeaseResponse;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use tokio::time::{Duration, Instant};

const PORT_ALLOCATION_RETRY_TIMEOUT: Duration = Duration::from_secs(5);
const PORT_ALLOCATION_RETRY_BASE_DELAY: Duration = Duration::from_millis(20);
const PORT_ALLOCATION_RETRY_JITTER_RANGE_MS: u64 = 40;

impl LeaseManager {
    pub(crate) async fn insert_with_allocated_ports<F>(
        &self,
        jitter_key: &str,
        id: String,
        record: F,
    ) -> Result<LeaseResponse, LeaseError>
    where
        F: Fn(u16, u16) -> LeaseRecord,
    {
        let deadline = Instant::now() + PORT_ALLOCATION_RETRY_TIMEOUT;
        let jitter_seed = jitter_key.bytes().fold(0u64, |acc, byte| {
            acc.wrapping_mul(33).wrapping_add(u64::from(byte))
        });
        let mut attempt = 0u64;

        loop {
            let result = {
                let mut store = self.state.lock().await;
                self.allocate_ports_locked(&store)
                    .map(|(port, inspector_port)| {
                        let record = record(port, inspector_port);
                        let response = crate::lease_manager::lease_response(&record);
                        store.leases.insert(id.clone(), record);
                        response
                    })
            };
            match result {
                Ok(response) => return Ok(response),
                Err(LeaseError::Unavailable(_)) if Instant::now() < deadline => {
                    let delay_ms = PORT_ALLOCATION_RETRY_BASE_DELAY.as_millis() as u64
                        + ((jitter_seed + attempt) % PORT_ALLOCATION_RETRY_JITTER_RANGE_MS);
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    attempt = attempt.saturating_add(1);
                }
                Err(error) => return Err(error),
            }
        }
    }

    pub(crate) fn allocate_ports_locked(
        &self,
        store: &LeaseStore,
    ) -> Result<(u16, u16), LeaseError> {
        for port in self.config.port_start..=self.config.port_end {
            if !worker_port_available(store, port) {
                continue;
            }

            for inspector_port in self.config.inspector_port_start..=self.config.inspector_port_end
            {
                if inspector_port_available(store, port, inspector_port) {
                    return Ok((port, inspector_port));
                }
            }
        }
        Err(LeaseError::unavailable(format!(
            "no lease worker and inspector port pair available in worker range {}..={} and inspector range {}..={}",
            self.config.port_start,
            self.config.port_end,
            self.config.inspector_port_start,
            self.config.inspector_port_end
        )))
    }
}

fn worker_port_available(store: &LeaseStore, port: u16) -> bool {
    !store
        .leases
        .values()
        .any(|lease| lease.port == port || lease.inspector_port == port)
        && port_is_available_on(port, "0.0.0.0")
}

fn inspector_port_available(store: &LeaseStore, port: u16, inspector_port: u16) -> bool {
    inspector_port != port
        && !store
            .leases
            .values()
            .any(|lease| lease.port == inspector_port || lease.inspector_port == inspector_port)
        && port_is_available_on(inspector_port, "127.0.0.1")
}

fn port_is_available_on(port: u16, addr: &str) -> bool {
    TcpListener::bind((addr, port)).is_ok()
}

pub(crate) async fn wait_for_ports_available(
    worker_port: u16,
    inspector_port: u16,
    timeout_duration: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout_duration;
    loop {
        let public = bind_result(Ipv4Addr::UNSPECIFIED, worker_port);
        let inspector = bind_result(Ipv4Addr::LOCALHOST, inspector_port);
        if public.is_ok() && inspector.is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "startup ports did not become available after worker cleanup: public {worker_port} {}, inspector {inspector_port} {}",
                bind_status(&public),
                bind_status(&inspector)
            ));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn bind_result(addr: Ipv4Addr, port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind(SocketAddr::new(IpAddr::V4(addr), port))
}

fn bind_status(result: &std::io::Result<TcpListener>) -> String {
    match result {
        Ok(_) => "available=true".to_string(),
        Err(error) => format!(
            "available=false error_kind={:?} error={error}",
            error.kind()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lease_manager::{LeaseManagerConfig, LeaseRecord};
    use crate::lease_model::{LeaseBackend, LeaseState, LeaseStatus};
    use crate::lease_runtime::RealLeaseSpawner;
    use std::collections::{BTreeMap, HashMap};
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn allocate_port_skips_ports_occupied_by_other_processes() {
        let occupied = crate::test_ports::available_port_block(3);
        let available = occupied + 1;
        let listener = TcpListener::bind(("0.0.0.0", occupied)).expect("bind occupied port");
        let manager = test_manager(occupied, available, available + 1, available + 1);
        let store = test_store();

        let (port, inspector_port) = manager
            .allocate_ports_locked(&store)
            .expect("allocated ports");
        assert_ne!(port, occupied);
        assert_ne!(port, inspector_port);
        drop(listener);
    }

    #[test]
    fn allocate_ports_avoid_cross_collisions_between_worker_and_inspector_ports() {
        let base = crate::test_ports::available_port_block(4);
        let manager = test_manager(base, base + 3, base, base + 3);
        let mut store = test_store();
        store
            .leases
            .insert("alpha".to_string(), test_record("alpha", base, base + 1));

        let (port, inspector_port) = manager
            .allocate_ports_locked(&store)
            .expect("allocated ports");
        assert_eq!((port, inspector_port), (base + 2, base + 3));
    }

    fn test_manager(
        port_start: u16,
        port_end: u16,
        inspector_port_start: u16,
        inspector_port_end: u16,
    ) -> LeaseManager {
        LeaseManager::new(LeaseManagerConfig {
            lease_root: PathBuf::from("/tmp/lease-root"),
            worker_bin: PathBuf::from("/tmp/worker-bin"),
            wrangler_bin: PathBuf::from("/tmp/wrangler-bin"),
            port_start,
            port_end,
            inspector_port_start,
            inspector_port_end,
            failure_report_ttl_secs: 86_400,
            failure_report_max_entries: 100,
            spawner: Arc::new(RealLeaseSpawner),
        })
    }

    fn test_store() -> LeaseStore {
        LeaseStore {
            leases: HashMap::new(),
            failure_reports: HashMap::new(),
            next_failure_report_sequence: 0,
        }
    }

    fn test_record(id: &str, port: u16, inspector_port: u16) -> LeaseRecord {
        LeaseRecord {
            id: id.to_string(),
            name: id.to_string(),
            port,
            inspector_port,
            runtime_dir: PathBuf::from(format!("/tmp/{id}/runtime")),
            static_dir: PathBuf::from(format!("/tmp/{id}/static")),
            state_dir: PathBuf::from(format!("/tmp/{id}/state")),
            log_dir: PathBuf::from(format!("/tmp/{id}/logs")),
            health_path: "/health".to_string(),
            env: "dev".to_string(),
            protocol: "http".to_string(),
            log_level: "warn".to_string(),
            env_vars: BTreeMap::new(),
            persist_state: true,
            backend: LeaseBackend::Miniflare,
            bundle_metadata: None,
            bundle_uploaded_at: None,
            startup_diagnostics: None,
            startup_details: None,
            last_probe: None,
            status: LeaseStatus::new(LeaseState::Created),
            generation: 0,
            child: None,
            startup_permit: None,
        }
    }
}
