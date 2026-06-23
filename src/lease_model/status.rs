use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeaseState {
    Created,
    Bundled,
    Starting,
    Ready,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LeaseStatus {
    pub state: LeaseState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl LeaseStatus {
    #[must_use]
    pub fn new(state: LeaseState) -> Self {
        Self {
            state,
            message: None,
        }
    }

    #[must_use]
    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            state: LeaseState::Failed,
            message: Some(message.into()),
        }
    }
}
