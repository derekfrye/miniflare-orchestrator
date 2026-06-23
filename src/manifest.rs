use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Manifest {
    pub projects: Vec<ProjectInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectInput {
    pub name: String,
    pub runtime_dir: PathBuf,
    pub state_dir: PathBuf,
    pub log_dir: PathBuf,
    pub static_dir: PathBuf,
    pub config_file: PathBuf,
    pub reload_token: PathBuf,
    #[serde(default = "default_health_path")]
    pub health_path: String,
    pub port: u16,
    #[serde(default = "default_env")]
    pub env: String,
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

/// Reads and parses a JSON manifest from disk.
///
/// # Errors
///
/// Returns an error if the file cannot be read or the JSON is invalid.
pub fn read_manifest(path: &std::path::Path) -> crate::error::Result<Manifest> {
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn default_health_path() -> String {
    "/health".to_string()
}

fn default_env() -> String {
    "dev".to_string()
}

fn default_protocol() -> String {
    "http".to_string()
}
