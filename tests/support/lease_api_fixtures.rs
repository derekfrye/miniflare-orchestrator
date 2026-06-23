use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use worker_runtime_host_gen::lease_manager::LeaseError;
use worker_runtime_host_gen::lease_model::LeaseBackend;
use worker_runtime_host_gen::lease_runtime::{LeaseLaunchConfig, LeaseRuntimeConfig, LeaseSpawner};
use worker_runtime_host_gen::service::DocsServiceConfig;

pub use crate::basic::available_port_ranges;

#[must_use]
pub fn test_config(
    temp: &tempfile::TempDir,
    worker_bin: &str,
    wrangler_bin: &str,
    lease_count: u16,
) -> DocsServiceConfig {
    test_config_with_retention(temp, worker_bin, wrangler_bin, lease_count, 86_400, 100)
}

#[must_use]
pub fn test_config_with_retention(
    temp: &tempfile::TempDir,
    worker_bin: &str,
    wrangler_bin: &str,
    lease_count: u16,
    failure_report_ttl_secs: u64,
    failure_report_max_entries: usize,
) -> DocsServiceConfig {
    let ((lease_port_start, lease_port_end), (inspector_port_start, inspector_port_end)) =
        available_port_ranges(lease_count);

    DocsServiceConfig {
        bind: "127.0.0.1".to_string(),
        port: 8786,
        plan_file: temp.path().join("work/host/config/projects.plan.json"),
        host_root: temp.path().join("work/host"),
        lease_root: temp.path().join("work/host/leases"),
        lease_port_start,
        lease_port_end,
        lease_inspector_port_start: inspector_port_start,
        lease_inspector_port_end: inspector_port_end,
        worker_bin: worker_bin.into(),
        wrangler_bin: wrangler_bin.into(),
        failure_report_ttl_secs,
        failure_report_max_entries,
    }
}

#[must_use]
pub fn fake_worker_script() -> String {
    r#"#!/usr/bin/env python3
import http.server
import socketserver
import sys

port = int(sys.argv[1])

class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("content-type", "text/plain")
        self.end_headers()
        self.wfile.write(b"ok")

    def log_message(self, format, *args):
        return

class Server(socketserver.TCPServer):
    allow_reuse_address = True

with Server(("127.0.0.1", port), Handler) as server:
    server.serve_forever()
"#
    .to_string()
}

#[derive(Debug, Clone)]
pub struct SpawnCall {
    pub worker_bin: std::path::PathBuf,
    pub wrangler_bin: std::path::PathBuf,
    pub runtime_dir: std::path::PathBuf,
    pub static_dir: std::path::PathBuf,
    pub state_dir: std::path::PathBuf,
    pub log_dir: std::path::PathBuf,
    pub config_file: std::path::PathBuf,
    pub port: u16,
    pub inspector_port: u16,
    pub env_name: String,
    pub protocol: String,
    pub log_level: String,
    pub env_vars: BTreeMap<String, String>,
    pub persist_state: bool,
    pub backend: LeaseBackend,
}

#[derive(Debug, Clone)]
pub struct RecordingSpawner {
    pub calls: Arc<Mutex<Vec<SpawnCall>>>,
}

impl Default for RecordingSpawner {
    fn default() -> Self {
        Self::new()
    }
}

impl RecordingSpawner {
    #[must_use]
    pub fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl LeaseSpawner for RecordingSpawner {
    fn spawn_worker<'a>(
        &'a self,
        config: &'a LeaseRuntimeConfig,
        launch: &'a LeaseLaunchConfig,
    ) -> Pin<Box<dyn Future<Output = Result<Child, LeaseError>> + Send + 'a>> {
        let calls = self.calls.clone();
        Box::pin(async move {
            calls.lock().await.push(SpawnCall {
                worker_bin: config.worker_bin.clone(),
                wrangler_bin: config.wrangler_bin.clone(),
                runtime_dir: launch.runtime_dir.clone(),
                static_dir: launch.static_dir.clone(),
                state_dir: launch.state_dir.clone(),
                log_dir: launch.log_dir.clone(),
                config_file: launch.config_file.clone(),
                port: launch.port,
                inspector_port: launch.inspector_port,
                env_name: launch.env_name.clone(),
                protocol: launch.protocol.clone(),
                log_level: launch.log_level.clone(),
                env_vars: launch.env_vars.clone(),
                persist_state: launch.persist_state,
                backend: launch.backend,
            });

            let child = Command::new("python3")
                .arg("-c")
                .arg(fake_worker_script())
                .arg(launch.port.to_string())
                .spawn()
                .map_err(LeaseError::from)?;
            Ok(child)
        })
    }
}
