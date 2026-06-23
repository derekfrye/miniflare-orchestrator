use crate::lease_manager::LeaseError;
use crate::lease_model::LeaseBackend;
use crate::secure_fs;
use crate::service::{
    MINIFLARE_WORKERD_CONFIG_DEBUG_ENV, MINIFLARE_WORKERD_PATH_ENV, WORKER_CONFIG_FILE_ENV,
    WORKER_ENV_ENV, WORKER_INSPECTOR_PORT_ENV, WORKER_LOG_DIR_ENV, WORKER_LOG_LEVEL_ENV,
    WORKER_MINIFLARE_DISABLE_INSPECTOR_ENV, WORKER_MINIFLARE_MODULE_ENV,
    WORKER_MINIFLARE_VERBOSE_ENV, WORKER_MINIFLARE_WORKERD_CONFIG_DEBUG_ENV, WORKER_NODE_BIN_ENV,
    WORKER_PORT_ENV, WORKER_PROTOCOL_ENV, WORKER_RUNTIME_BACKEND_ENV, WORKER_RUNTIME_DIR_ENV,
    WORKER_STATE_DIR_ENV, WORKER_WRANGLER_BIN_ENV,
};
use std::collections::BTreeMap;
use std::fs;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};

pub struct LeaseRuntimeConfig {
    pub worker_bin: PathBuf,
    pub wrangler_bin: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LeaseLaunchConfig {
    pub runtime_dir: PathBuf,
    pub static_dir: PathBuf,
    pub state_dir: PathBuf,
    pub log_dir: PathBuf,
    pub config_file: PathBuf,
    pub port: u16,
    pub inspector_port: u16,
    pub env_name: String,
    pub protocol: String,
    pub log_level: String,
    pub env_vars: BTreeMap<String, String>,
    pub persist_state: bool,
    pub backend: LeaseBackend,
}
pub trait LeaseSpawner: Send + Sync + std::fmt::Debug {
    fn spawn_worker<'a>(
        &'a self,
        config: &'a LeaseRuntimeConfig,
        launch: &'a LeaseLaunchConfig,
    ) -> Pin<Box<dyn Future<Output = Result<Child, LeaseError>> + Send + 'a>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealLeaseSpawner;
impl LeaseSpawner for RealLeaseSpawner {
    fn spawn_worker<'a>(
        &'a self,
        config: &'a LeaseRuntimeConfig,
        launch: &'a LeaseLaunchConfig,
    ) -> Pin<Box<dyn Future<Output = Result<Child, LeaseError>> + Send + 'a>> {
        Box::pin(async move { spawn_worker(config, launch) })
    }
}

#[must_use]
pub fn real_lease_spawner() -> Arc<dyn LeaseSpawner> {
    Arc::new(RealLeaseSpawner)
}

/// Spawns the worker process for a lease.
///
/// # Errors
///
/// Returns an error if the runtime directories or config file are missing or
/// if the worker process cannot be started.
pub fn spawn_worker(
    config: &LeaseRuntimeConfig,
    launch: &LeaseLaunchConfig,
) -> Result<Child, LeaseError> {
    secure_fs::create_ambient_private_dir_all(&launch.runtime_dir)?;
    secure_fs::create_ambient_private_dir_all(&launch.state_dir)?;
    secure_fs::create_ambient_private_dir_all(&launch.log_dir)?;
    secure_fs::create_ambient_private_dir_all(&launch.static_dir)?;
    let runtime_dir = secure_fs::open_ambient_dir(&launch.runtime_dir)?;
    let state_dir = secure_fs::open_ambient_dir(&launch.state_dir)?;
    let log_dir = secure_fs::open_ambient_dir(&launch.log_dir)?;
    let _static_dir = secure_fs::open_ambient_dir(&launch.static_dir)?;

    if !launch.persist_state {
        secure_fs::clear_dir_contents(&state_dir)?;
    }

    let config_relative = launch
        .config_file
        .strip_prefix(&launch.runtime_dir)
        .map_err(|_| {
            LeaseError::usage(format!(
                "Wrangler config must live inside runtime dir: {}",
                launch.config_file.display()
            ))
        })?;
    if let Err(error) = runtime_dir.open(config_relative) {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error.into());
        }
        return Err(LeaseError::usage(format!(
            "missing Wrangler config: {}",
            launch.config_file.display()
        )));
    }

    let stdout_log = secure_fs::open_append_private_file(&log_dir, "stdout.log")?;
    let stderr_log = secure_fs::open_append_private_file(&log_dir, "stderr.log")?;

    let child = worker_command(config, launch, stdout_log, stderr_log)
        .spawn()
        .map_err(LeaseError::from)?;

    Ok(child)
}

#[must_use]
pub fn lease_launch_env(
    launch: &LeaseLaunchConfig,
    wrangler_bin: &Path,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::from([
        ("HOME".to_string(), launch.state_dir.display().to_string()),
        (
            "PATH".to_string(),
            "/usr/local/bin:/usr/bin:/bin".to_string(),
        ),
        ("TMPDIR".to_string(), launch.state_dir.display().to_string()),
        (
            WORKER_RUNTIME_DIR_ENV.to_string(),
            launch.runtime_dir.display().to_string(),
        ),
        (
            WORKER_STATE_DIR_ENV.to_string(),
            launch.state_dir.display().to_string(),
        ),
        (
            WORKER_LOG_DIR_ENV.to_string(),
            launch.log_dir.display().to_string(),
        ),
        (
            WORKER_CONFIG_FILE_ENV.to_string(),
            launch.config_file.display().to_string(),
        ),
        (WORKER_PORT_ENV.to_string(), launch.port.to_string()),
        (
            WORKER_INSPECTOR_PORT_ENV.to_string(),
            launch.inspector_port.to_string(),
        ),
        (WORKER_ENV_ENV.to_string(), launch.env_name.clone()),
        (WORKER_PROTOCOL_ENV.to_string(), launch.protocol.clone()),
        (WORKER_LOG_LEVEL_ENV.to_string(), launch.log_level.clone()),
        (
            WORKER_RUNTIME_BACKEND_ENV.to_string(),
            backend_name(launch.backend).to_string(),
        ),
        (
            WORKER_WRANGLER_BIN_ENV.to_string(),
            wrangler_bin.display().to_string(),
        ),
        (WORKER_NODE_BIN_ENV.to_string(), "node".to_string()),
    ]);
    env.extend(launch.env_vars.clone());
    env.extend(miniflare_debug_env());
    env
}

fn miniflare_debug_env() -> BTreeMap<String, String> {
    [
        WORKER_MINIFLARE_MODULE_ENV,
        WORKER_MINIFLARE_VERBOSE_ENV,
        WORKER_MINIFLARE_WORKERD_CONFIG_DEBUG_ENV,
        WORKER_MINIFLARE_DISABLE_INSPECTOR_ENV,
        MINIFLARE_WORKERD_PATH_ENV,
        MINIFLARE_WORKERD_CONFIG_DEBUG_ENV,
    ]
    .into_iter()
    .filter_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| (name.to_string(), value))
    })
    .collect()
}

#[must_use]
pub fn backend_name(backend: LeaseBackend) -> &'static str {
    match backend {
        LeaseBackend::WranglerDev => "wrangler_dev",
        LeaseBackend::Miniflare => "miniflare",
    }
}

fn worker_command(
    config: &LeaseRuntimeConfig,
    launch: &LeaseLaunchConfig,
    stdout_log: fs::File,
    stderr_log: fs::File,
) -> Command {
    let mut command = Command::new(&config.worker_bin);
    #[cfg(unix)]
    command.process_group(0);
    command
        .stdout(Stdio::from(stdout_log))
        .stderr(Stdio::from(stderr_log));
    command.env_clear();
    for (name, value) in lease_launch_env(launch, &config.wrangler_bin) {
        command.env(name, value);
    }
    command.current_dir(&launch.runtime_dir);
    command.stdin(Stdio::null());
    command
}
