#[path = "service/env.rs"]
pub(crate) mod env_helpers;

use crate::error::Result;
use env_helpers::{
    env_path, env_path_default, env_string, env_u16, env_u16_default, env_u64_default,
    env_usize_default,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const WORKER_RUNTIME_DIR_ENV: &str = "WORKER_RUNTIME_HOST_RUNTIME_DIR";
pub const WORKER_STATE_DIR_ENV: &str = "WORKER_RUNTIME_HOST_STATE_DIR";
pub const WORKER_LOG_DIR_ENV: &str = "WORKER_RUNTIME_HOST_LOG_DIR";
pub const WORKER_CONFIG_FILE_ENV: &str = "WORKER_RUNTIME_HOST_CONFIG_FILE";
pub const WORKER_PORT_ENV: &str = "WORKER_RUNTIME_HOST_PORT";
pub const WORKER_INSPECTOR_PORT_ENV: &str = "WORKER_RUNTIME_HOST_INSPECTOR_PORT";
pub const WORKER_ENV_ENV: &str = "WORKER_RUNTIME_HOST_ENV";
pub const WORKER_PROTOCOL_ENV: &str = "WORKER_RUNTIME_HOST_PROTOCOL";
pub const WORKER_LOG_LEVEL_ENV: &str = "WORKER_RUNTIME_HOST_LOG_LEVEL";
pub const WORKER_WRANGLER_BIN_ENV: &str = "WORKER_RUNTIME_HOST_WRANGLER_BIN";
pub const WORKER_RUNTIME_BACKEND_ENV: &str = "WORKER_RUNTIME_HOST_BACKEND";
pub const WORKER_NODE_BIN_ENV: &str = "WORKER_RUNTIME_HOST_NODE_BIN";
pub const WORKER_MINIFLARE_MODULE_ENV: &str = "WORKER_RUNTIME_HOST_MINIFLARE_MODULE";
pub const WORKER_MINIFLARE_VERBOSE_ENV: &str = "WORKER_RUNTIME_HOST_MINIFLARE_VERBOSE";
pub const WORKER_MINIFLARE_WORKERD_CONFIG_DEBUG_ENV: &str =
    "WORKER_RUNTIME_HOST_MINIFLARE_WORKERD_CONFIG_DEBUG";
pub const WORKER_MINIFLARE_DISABLE_INSPECTOR_ENV: &str =
    "WORKER_RUNTIME_HOST_MINIFLARE_DISABLE_INSPECTOR";
pub const MINIFLARE_WORKERD_PATH_ENV: &str = "MINIFLARE_WORKERD_PATH";
pub const MINIFLARE_WORKERD_CONFIG_DEBUG_ENV: &str = "MINIFLARE_WORKERD_CONFIG_DEBUG";

pub const WATCH_RELOAD_TOKEN_ENV: &str = "WORKER_RUNTIME_HOST_RELOAD_TOKEN";
pub const WATCH_WORKER_SERVICE_ENV: &str = "WORKER_RUNTIME_HOST_WORKER_SERVICE";

pub const DOCS_BIND_ENV: &str = "WORKER_RUNTIME_HOST_DOCS_BIND";
pub const DOCS_PORT_ENV: &str = "WORKER_RUNTIME_HOST_DOCS_PORT";
pub const DOCS_PLAN_FILE_ENV: &str = "WORKER_RUNTIME_HOST_DOCS_PLAN_FILE";
pub const DOCS_HOST_ROOT_ENV: &str = "WORKER_RUNTIME_HOST_DOCS_HOST_ROOT";
pub const DOCS_LEASE_ROOT_ENV: &str = "WORKER_RUNTIME_HOST_DOCS_LEASE_ROOT";
pub const DOCS_LEASE_PORT_START_ENV: &str = "WORKER_RUNTIME_HOST_DOCS_LEASE_PORT_START";
pub const DOCS_LEASE_PORT_END_ENV: &str = "WORKER_RUNTIME_HOST_DOCS_LEASE_PORT_END";
pub const DOCS_LEASE_INSPECTOR_PORT_START_ENV: &str =
    "WORKER_RUNTIME_HOST_DOCS_LEASE_INSPECTOR_PORT_START";
pub const DOCS_LEASE_INSPECTOR_PORT_END_ENV: &str =
    "WORKER_RUNTIME_HOST_DOCS_LEASE_INSPECTOR_PORT_END";
pub const DOCS_WORKER_BIN_ENV: &str = "WORKER_RUNTIME_HOST_DOCS_WORKER_BIN";
pub const DOCS_WRANGLER_BIN_ENV: &str = "WORKER_RUNTIME_HOST_DOCS_WRANGLER_BIN";
pub const DOCS_FAILURE_REPORT_TTL_SECS_ENV: &str =
    "WORKER_RUNTIME_HOST_DOCS_FAILURE_REPORT_TTL_SECS";
pub const DOCS_FAILURE_REPORT_MAX_ENTRIES_ENV: &str =
    "WORKER_RUNTIME_HOST_DOCS_FAILURE_REPORT_MAX_ENTRIES";

#[derive(Debug, Clone)]
pub struct WorkerServiceConfig {
    pub runtime_dir: PathBuf,
    pub state_dir: PathBuf,
    pub log_dir: PathBuf,
    pub config_file: PathBuf,
    pub port: u16,
    pub inspector_port: u16,
    pub env_name: String,
    pub protocol: String,
    pub log_level: String,
    pub wrangler_bin: PathBuf,
    pub node_bin: PathBuf,
    pub backend: crate::lease_model::LeaseBackend,
    pub env_vars: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct WatchServiceConfig {
    pub reload_token: PathBuf,
    pub worker_service: PathBuf,
}

#[derive(Debug, Clone)]
pub struct DocsServiceConfig {
    pub bind: String,
    pub port: u16,
    pub plan_file: PathBuf,
    pub host_root: PathBuf,
    pub lease_root: PathBuf,
    pub lease_port_start: u16,
    pub lease_port_end: u16,
    pub lease_inspector_port_start: u16,
    pub lease_inspector_port_end: u16,
    pub worker_bin: PathBuf,
    pub wrangler_bin: PathBuf,
    pub failure_report_ttl_secs: u64,
    pub failure_report_max_entries: usize,
}

impl WorkerServiceConfig {
    /// Loads the worker service configuration from the environment.
    ///
    /// # Errors
    ///
    /// Returns an error if any required variable is missing or invalid.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            runtime_dir: env_path(WORKER_RUNTIME_DIR_ENV)?,
            state_dir: env_path(WORKER_STATE_DIR_ENV)?,
            log_dir: env_path(WORKER_LOG_DIR_ENV)?,
            config_file: env_path(WORKER_CONFIG_FILE_ENV)?,
            port: env_u16(WORKER_PORT_ENV)?,
            inspector_port: env_u16_default(WORKER_INSPECTOR_PORT_ENV, 9229)?,
            env_name: env_string(WORKER_ENV_ENV, "dev")?,
            protocol: env_string(WORKER_PROTOCOL_ENV, "http")?,
            log_level: env_string(WORKER_LOG_LEVEL_ENV, "warn")?,
            wrangler_bin: env_path_default(WORKER_WRANGLER_BIN_ENV, "wrangler")?,
            node_bin: env_path_default(WORKER_NODE_BIN_ENV, "node")?,
            backend: worker_backend_from_env()?,
            env_vars: worker_env_vars(),
        })
    }
}

fn worker_env_vars() -> BTreeMap<String, String> {
    let known = [
        WORKER_RUNTIME_DIR_ENV,
        WORKER_STATE_DIR_ENV,
        WORKER_LOG_DIR_ENV,
        WORKER_CONFIG_FILE_ENV,
        WORKER_PORT_ENV,
        WORKER_INSPECTOR_PORT_ENV,
        WORKER_ENV_ENV,
        WORKER_PROTOCOL_ENV,
        WORKER_LOG_LEVEL_ENV,
        WORKER_WRANGLER_BIN_ENV,
        WORKER_RUNTIME_BACKEND_ENV,
        WORKER_NODE_BIN_ENV,
        WORKER_MINIFLARE_MODULE_ENV,
        WORKER_MINIFLARE_VERBOSE_ENV,
        WORKER_MINIFLARE_WORKERD_CONFIG_DEBUG_ENV,
        WORKER_MINIFLARE_DISABLE_INSPECTOR_ENV,
        MINIFLARE_WORKERD_PATH_ENV,
        MINIFLARE_WORKERD_CONFIG_DEBUG_ENV,
        "HOME",
        "PATH",
        "TMPDIR",
    ];
    std::env::vars()
        .filter(|(name, _)| !known.contains(&name.as_str()))
        .collect()
}

fn worker_backend_from_env() -> Result<crate::lease_model::LeaseBackend> {
    let value = env_string(WORKER_RUNTIME_BACKEND_ENV, "miniflare")?;
    match value.as_str() {
        "wrangler_dev" | "wrangler-dev" | "wrangler" => {
            Ok(crate::lease_model::LeaseBackend::WranglerDev)
        }
        "miniflare" => Ok(crate::lease_model::LeaseBackend::Miniflare),
        _ => Err(crate::error::CliError::Usage(format!(
            "unsupported worker backend: {value}"
        ))),
    }
}

impl WatchServiceConfig {
    /// Loads the watcher service configuration from the environment.
    ///
    /// # Errors
    ///
    /// Returns an error if any required variable is missing or invalid.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            reload_token: env_path(WATCH_RELOAD_TOKEN_ENV)?,
            worker_service: env_path(WATCH_WORKER_SERVICE_ENV)?,
        })
    }
}

impl DocsServiceConfig {
    /// Loads the docs service configuration from the environment.
    ///
    /// # Errors
    ///
    /// Returns an error if any required variable is missing or invalid.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            bind: env_string(DOCS_BIND_ENV, "0.0.0.0")?,
            port: env_u16_default(DOCS_PORT_ENV, 8786)?,
            plan_file: env_path_default(
                DOCS_PLAN_FILE_ENV,
                "/work/host/config/projects.plan.json",
            )?,
            host_root: env_path_default(DOCS_HOST_ROOT_ENV, "/work/host")?,
            lease_root: env_path_default(DOCS_LEASE_ROOT_ENV, "/work/host/leases")?,
            lease_port_start: env_u16_default(DOCS_LEASE_PORT_START_ENV, 8900)?,
            lease_port_end: env_u16_default(DOCS_LEASE_PORT_END_ENV, 8999)?,
            lease_inspector_port_start: env_u16_default(DOCS_LEASE_INSPECTOR_PORT_START_ENV, 9000)?,
            lease_inspector_port_end: env_u16_default(DOCS_LEASE_INSPECTOR_PORT_END_ENV, 9099)?,
            worker_bin: env_path_default(
                DOCS_WORKER_BIN_ENV,
                "/usr/local/bin/worker-runtime-host-worker",
            )?,
            wrangler_bin: env_path_default(DOCS_WRANGLER_BIN_ENV, "wrangler")?,
            failure_report_ttl_secs: env_u64_default(DOCS_FAILURE_REPORT_TTL_SECS_ENV, 86_400)?,
            failure_report_max_entries: env_usize_default(
                DOCS_FAILURE_REPORT_MAX_ENTRIES_ENV,
                100,
            )?,
        })
    }

    #[must_use]
    pub fn socket_addr(&self) -> std::net::SocketAddr {
        let port = self.port;
        let addr = self
            .bind
            .parse::<std::net::IpAddr>()
            .unwrap_or(std::net::IpAddr::from([0, 0, 0, 0]));
        std::net::SocketAddr::new(addr, port)
    }
}
