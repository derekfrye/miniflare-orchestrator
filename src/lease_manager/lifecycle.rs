use crate::lease_manager::LeaseStore;
use crate::lease_runtime::LeaseSpawner;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::process::Child;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

const DEFAULT_CONCURRENT_STARTUPS: usize = 16;
const MAX_CONCURRENT_STARTUPS: usize = 16;

static STARTUP_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct LeaseManagerConfig {
    pub lease_root: PathBuf,
    pub worker_bin: PathBuf,
    pub wrangler_bin: PathBuf,
    pub port_start: u16,
    pub port_end: u16,
    pub inspector_port_start: u16,
    pub inspector_port_end: u16,
    pub failure_report_ttl_secs: u64,
    pub failure_report_max_entries: usize,
    pub spawner: Arc<dyn LeaseSpawner>,
}

#[derive(Debug, Clone)]
pub struct LeaseManager {
    pub(crate) config: LeaseManagerConfig,
    pub(crate) state: Arc<Mutex<LeaseStore>>,
}

impl LeaseManager {
    #[must_use]
    pub fn new(config: LeaseManagerConfig) -> Self {
        let manager = Self {
            config,
            state: Arc::new(Mutex::new(LeaseStore {
                leases: HashMap::new(),
                failure_reports: HashMap::new(),
                next_failure_report_sequence: 0,
            })),
        };
        manager.spawn_failure_report_janitor();
        manager
    }

    pub(crate) async fn kill_child(&self, child: Option<Child>) {
        crate::process_tree::kill_child_process_group(child).await;
    }

    pub(crate) async fn acquire_startup_permit(&self) -> OwnedSemaphorePermit {
        STARTUP_SEMAPHORE
            .get_or_init(|| Arc::new(Semaphore::new(configured_concurrent_startups())))
            .clone()
            .acquire_owned()
            .await
            .expect("startup semaphore should not be closed")
    }
}

fn configured_concurrent_startups() -> usize {
    std::env::var("WORKER_RUNTIME_HOST_MAX_CONCURRENT_STARTUPS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_CONCURRENT_STARTUPS)
        .min(MAX_CONCURRENT_STARTUPS)
}
