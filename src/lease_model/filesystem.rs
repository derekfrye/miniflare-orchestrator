use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeaseFilesystemEntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseFilesystemEntry {
    pub root: String,
    pub path: String,
    pub kind: LeaseFilesystemEntryKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseFilesystemSnapshot {
    pub runtime_dir: String,
    pub static_dir: String,
    pub state_dir: String,
    pub log_dir: String,
    pub entries: Vec<LeaseFilesystemEntry>,
}
