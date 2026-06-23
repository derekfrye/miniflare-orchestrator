use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use reqwest::Method;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::redirect::Policy;

// Sensible defaults for the bundled Quadlet/Podman network example. Override
// these in LeaseWorkerConfig if your Podman network, host IP, or published
// control-plane port differs.
pub const DEFAULT_HOST_BASE_URL: &str = "http://10.3.0.105:8786";
pub const DEFAULT_WORKER_HOST: &str = "10.3.0.105";

#[derive(Clone, Debug)]
pub struct LeaseWorkerConfig {
    pub host_base_url: String,
    pub worker_host: String,
    pub lease_name_prefix: String,
    pub health_path: String,
    pub env: String,
    pub protocol: String,
    pub log_level: String,
    pub backend: String,
    pub persist_state: bool,
    pub env_vars: BTreeMap<String, String>,
    pub runtime_files: Vec<LeaseFile>,
    pub static_files: Vec<LeaseFile>,
    pub bearer_token: Option<String>,
    pub readiness_timeout: Duration,
}

impl LeaseWorkerConfig {
    #[must_use]
    pub fn new(lease_name_prefix: impl Into<String>) -> Self {
        Self {
            host_base_url: DEFAULT_HOST_BASE_URL.to_string(),
            worker_host: DEFAULT_WORKER_HOST.to_string(),
            lease_name_prefix: lease_name_prefix.into(),
            health_path: "/health".to_string(),
            env: "dev".to_string(),
            protocol: "https".to_string(),
            log_level: "warn".to_string(),
            backend: "miniflare".to_string(),
            persist_state: false,
            env_vars: BTreeMap::new(),
            runtime_files: Vec::new(),
            static_files: Vec::new(),
            bearer_token: None,
            readiness_timeout: Duration::from_secs(120),
        }
    }
}

pub struct LeaseWorkerHarness {
    lease_cleanup: Option<LeaseCleanup>,
    lease: LeaseResponse,
    client: Client,
    no_redirect_client: Client,
    host_base_url: String,
    worker_host: String,
    bearer_token: Option<String>,
}

impl LeaseWorkerHarness {
    /// Spawns a worker lease on the configured orchestration host.
    ///
    /// # Panics
    ///
    /// Panics if the lease cannot be created, bundled, started, or polled to readiness.
    #[must_use]
    pub fn spawn(config: LeaseWorkerConfig) -> Self {
        let client = http_client(false);
        let no_redirect_client = http_client(true);
        let lease_name = unique_lease_name(&config.lease_name_prefix);
        let lease = create_lease(&client, &config, &lease_name);
        let lease_cleanup = Some(LeaseCleanup::new(&client, &config.host_base_url, &lease));
        let lease = bundle_lease(&client, &config.host_base_url, &lease, &config);
        let lease = restart_lease(&client, &config.host_base_url, &lease, config.persist_state);
        let mut harness = Self {
            lease_cleanup,
            lease,
            client,
            no_redirect_client,
            host_base_url: config.host_base_url,
            worker_host: config.worker_host,
            bearer_token: config.bearer_token,
        };
        harness.wait_until_ready(config.readiness_timeout);
        harness.lease_cleanup = None;
        harness
    }

    pub fn get(&self, path: &str) -> RequestBuilder {
        self.client.request(Method::GET, self.url(path))
    }

    pub fn get_no_redirect(&self, path: &str) -> RequestBuilder {
        self.no_redirect_client.request(Method::GET, self.url(path))
    }

    pub fn post(&self, path: &str) -> RequestBuilder {
        self.client.request(Method::POST, self.url(path))
    }

    pub fn authed_get(&self, path: &str) -> RequestBuilder {
        self.with_bearer(self.get(path))
    }

    pub fn authed_post(&self, path: &str) -> RequestBuilder {
        self.with_bearer(self.post(path))
    }

    pub fn authed_put(&self, path: &str) -> RequestBuilder {
        let request = self.client.request(Method::PUT, self.url(path));
        self.with_bearer(request)
    }

    pub fn authed_delete(&self, path: &str) -> RequestBuilder {
        let request = self.client.request(Method::DELETE, self.url(path));
        self.with_bearer(request)
    }

    #[must_use]
    pub fn combined_logs(&self) -> String {
        self.client
            .get(self.lease_url("/logs"))
            .send()
            .ok()
            .and_then(|response| response.text().ok())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn debug_snapshot(&self) -> String {
        self.client
            .get(self.lease_url("/debug"))
            .send()
            .ok()
            .and_then(|response| response.text().ok())
            .unwrap_or_default()
    }

    /// Waits until the combined worker logs contain `needle`.
    ///
    /// # Panics
    ///
    /// Panics if the expected log line does not appear before the timeout expires.
    pub fn wait_for_log_contains(&self, needle: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let logs = self.combined_logs();
            if logs.contains(needle) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for log containing {needle:?}\nlogs:\n{logs}"
            );
            thread::sleep(Duration::from_millis(200));
        }
    }

    #[must_use]
    pub fn base_url(&self) -> String {
        self.worker_base_url()
    }

    fn wait_until_ready(&mut self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            debug(format!(
                "polling lease {} at {} (worker url {})",
                self.lease.id,
                self.lease.base_url,
                self.worker_base_url()
            ));
            let lease = self
                .client
                .get(self.lease_url(""))
                .send()
                .and_then(reqwest::blocking::Response::error_for_status)
                .and_then(reqwest::blocking::Response::json::<LeaseResponse>);

            match lease {
                Ok(lease) if lease.status.state == "ready" => {
                    debug(format!("lease {} reported ready", lease.id));
                    let health = self.client.get(self.url(&self.lease.health_path)).send();
                    if matches!(health, Ok(response) if response.status().is_success()) {
                        debug(format!(
                            "lease {} health probe succeeded at {}",
                            lease.id,
                            self.url(&self.lease.health_path)
                        ));
                        self.lease = lease;
                        return;
                    }
                    debug(format!(
                        "lease {} ready but health probe failed; retrying",
                        lease.id
                    ));
                    self.lease = lease;
                }
                Ok(lease) if lease.status.state == "failed" || lease.status.state == "stopped" => {
                    panic!(
                        "lease failed to start: state={}, message={:?}\nlogs:\n{}",
                        lease.status.state,
                        lease.status.message,
                        self.combined_logs()
                    );
                }
                Ok(lease) => {
                    debug(format!(
                        "lease {} state {} message {:?}",
                        lease.id, lease.status.state, lease.status.message
                    ));
                    self.lease = lease;
                }
                Err(err) => {
                    debug(format!("lease status poll error: {err}"));
                    assert!(
                        Instant::now() < deadline,
                        "timed out waiting for lease readiness: {err}\nlogs:\n{}",
                        self.combined_logs()
                    );
                }
            }

            assert!(
                Instant::now() < deadline,
                "timed out waiting for lease readiness\nlogs:\n{}",
                self.combined_logs()
            );
            thread::sleep(Duration::from_millis(250));
        }
    }

    fn with_bearer(&self, request: RequestBuilder) -> RequestBuilder {
        match self.bearer_token.as_deref() {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.worker_base_url(), path)
    }

    fn worker_base_url(&self) -> String {
        format!(
            "{}://{}:{}",
            self.lease.protocol, self.worker_host, self.lease.port
        )
    }

    fn lease_url(&self, path: &str) -> String {
        format!("{}/leases/{}{}", self.host_base_url, self.lease.id, path)
    }
}

impl Drop for LeaseWorkerHarness {
    fn drop(&mut self) {
        if let Some(cleanup) = self.lease_cleanup.take() {
            cleanup.delete_and_wait();
        } else {
            let cleanup = LeaseCleanup::new(&self.client, &self.host_base_url, &self.lease);
            cleanup.delete_and_wait();
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct LeaseFile {
    pub path: String,
    pub content_b64: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct LeaseResponse {
    pub id: String,
    #[serde(default)]
    pub base_url: String,
    pub port: u16,
    pub protocol: String,
    pub health_path: String,
    pub status: LeaseStatus,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct LeaseStatus {
    pub state: String,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(serde::Serialize)]
struct LeaseCreateRequest {
    name: Option<String>,
    health_path: String,
    env: Option<String>,
    protocol: Option<String>,
    log_level: Option<String>,
    env_vars: BTreeMap<String, String>,
    persist_state: Option<bool>,
    backend: Option<String>,
}

#[derive(serde::Serialize)]
struct LeaseRestartRequest {
    persist_state: Option<bool>,
}

#[derive(serde::Serialize)]
struct LeaseBundleRequest {
    runtime_files: Vec<LeaseFile>,
    static_files: Vec<LeaseFile>,
}

struct LeaseCleanup {
    client: Client,
    host_base_url: String,
    lease_id: String,
}

impl LeaseCleanup {
    fn new(client: &Client, host_base_url: &str, lease: &LeaseResponse) -> Self {
        Self {
            client: client.clone(),
            host_base_url: host_base_url.to_string(),
            lease_id: lease.id.clone(),
        }
    }

    fn delete_and_wait(&self) {
        let _ = self
            .client
            .delete(format!("{}/leases/{}", self.host_base_url, self.lease_id))
            .send();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let status = self
                .client
                .get(format!("{}/leases/{}", self.host_base_url, self.lease_id))
                .send()
                .map(|response| response.status());
            if matches!(status, Ok(reqwest::StatusCode::NOT_FOUND)) {
                return;
            }
            if Instant::now() >= deadline {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

#[must_use]
pub fn collect_files(root: &Path, prefix: &str) -> Vec<LeaseFile> {
    let mut files = Vec::new();
    collect_files_recursive(root, root, prefix, &mut files);
    files
}

#[must_use]
pub fn lease_file_from_string(content: String, relative_path: &str) -> LeaseFile {
    LeaseFile {
        path: relative_path.to_string(),
        content_b64: STANDARD.encode(content),
    }
}

#[must_use]
pub fn lease_file_from_bytes(path: &Path, relative_path: String) -> LeaseFile {
    let content = fs::read(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    LeaseFile {
        path: relative_path,
        content_b64: STANDARD.encode(content),
    }
}

fn create_lease(client: &Client, config: &LeaseWorkerConfig, lease_name: &str) -> LeaseResponse {
    debug("creating lease");
    let response = client
        .post(format!("{}/leases", config.host_base_url))
        .json(&LeaseCreateRequest {
            backend: Some(config.backend.clone()),
            env: Some(config.env.clone()),
            env_vars: config.env_vars.clone(),
            health_path: config.health_path.clone(),
            log_level: Some(config.log_level.clone()),
            name: Some(lease_name.to_string()),
            persist_state: Some(config.persist_state),
            protocol: Some(config.protocol.clone()),
        })
        .send()
        .expect("create lease request");
    let lease: LeaseResponse = response_json(response, "create lease");
    debug(format!(
        "created lease {} at {} on port {}",
        lease.id, lease.base_url, lease.port
    ));
    lease
}

fn bundle_lease(
    client: &Client,
    host_base_url: &str,
    lease: &LeaseResponse,
    config: &LeaseWorkerConfig,
) -> LeaseResponse {
    debug(format!("bundling lease {}", lease.id));
    let response = client
        .post(format!("{host_base_url}/leases/{}/bundle", lease.id))
        .json(&LeaseBundleRequest {
            runtime_files: config.runtime_files.clone(),
            static_files: config.static_files.clone(),
        })
        .send()
        .expect("bundle lease request");
    let lease: LeaseResponse = response_json(response, "bundle lease");
    debug(format!(
        "bundled lease {} state {}",
        lease.id, lease.status.state
    ));
    lease
}

fn restart_lease(
    client: &Client,
    host_base_url: &str,
    lease: &LeaseResponse,
    persist_state: bool,
) -> LeaseResponse {
    debug(format!("restarting lease {}", lease.id));
    let response = client
        .post(format!("{host_base_url}/leases/{}/restart", lease.id))
        .json(&LeaseRestartRequest {
            persist_state: Some(persist_state),
        })
        .send()
        .expect("restart lease request");
    let lease: LeaseResponse = response_json(response, "restart lease");
    debug(format!(
        "restarted lease {} state {}",
        lease.id, lease.status.state
    ));
    lease
}

fn collect_files_recursive(root: &Path, path: &Path, prefix: &str, files: &mut Vec<LeaseFile>) {
    let entries =
        fs::read_dir(path).unwrap_or_else(|err| panic!("read dir {}: {err}", path.display()));
    for entry in entries {
        let entry = entry.expect("directory entry");
        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_files_recursive(root, &entry_path, prefix, files);
            continue;
        }
        if entry_path.is_file() {
            let relative = entry_path
                .strip_prefix(root)
                .unwrap_or_else(|err| panic!("strip prefix for {}: {err}", entry_path.display()));
            let mut relative = relative.to_string_lossy().replace('\\', "/");
            if !prefix.is_empty() {
                relative = format!("{prefix}/{relative}");
            }
            files.push(lease_file_from_bytes(&entry_path, relative));
        }
    }
}

fn unique_lease_name(prefix: &str) -> String {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis();
    format!("{prefix}-{now_ms}-{}", std::process::id())
}

fn response_json<T>(response: reqwest::blocking::Response, operation: &str) -> T
where
    T: serde::de::DeserializeOwned,
{
    let status = response.status();
    let body = response
        .text()
        .unwrap_or_else(|err| format!("failed to read response body: {err}"));
    assert!(
        status.is_success(),
        "{operation} failed with status {status}: {body}"
    );
    serde_json::from_str(&body)
        .unwrap_or_else(|err| panic!("{operation} invalid JSON: {err}\n{body}"))
}

fn http_client(no_redirect: bool) -> Client {
    let mut builder = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(15));
    if no_redirect {
        builder = builder.redirect(Policy::none());
    }
    builder.build().expect("http client")
}

fn debug(message: impl AsRef<str>) {
    if std::env::var_os("LEASE_WORKER_HARNESS_DEBUG").is_some() {
        eprintln!("[lease-harness] {}", message.as_ref());
    }
}
