use crate::error::{CliError, Result};
use std::env;

pub const RUNTIME_MODE_ENV: &str = "WORKER_RUNTIME_HOST_MODE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    LeasesOnly,
    ManifestAndLeases,
}

impl RuntimeMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LeasesOnly => "leases-only",
            Self::ManifestAndLeases => "manifest_and_leases",
        }
    }

    #[must_use]
    pub fn requires_manifest(self) -> bool {
        matches!(self, Self::ManifestAndLeases)
    }

    /// Loads the bootstrap mode from the environment.
    ///
    /// # Errors
    ///
    /// Returns an error if the environment value is invalid.
    pub fn from_env() -> Result<Self> {
        match env::var(RUNTIME_MODE_ENV) {
            Ok(value) if value == "leases-only" => Ok(Self::LeasesOnly),
            Ok(value) if value == "manifest_and_leases" => Ok(Self::ManifestAndLeases),
            Ok(value) if value.is_empty() => Err(CliError::Usage(format!(
                "env var is empty: {RUNTIME_MODE_ENV}"
            ))),
            Ok(value) => Err(CliError::Usage(format!(
                "invalid bootstrap mode in env var {RUNTIME_MODE_ENV}: {value}"
            ))),
            Err(env::VarError::NotPresent) => Ok(Self::LeasesOnly),
            Err(env::VarError::NotUnicode(_)) => Err(CliError::Usage(format!(
                "env var contains invalid unicode: {RUNTIME_MODE_ENV}"
            ))),
        }
    }
}
