use crate::error::{CliError, Result};
use std::env;
use std::path::PathBuf;

pub(crate) fn env_string(name: &str, default_value: &str) -> Result<String> {
    match env::var(name) {
        Ok(value) if !value.is_empty() => Ok(value),
        Ok(_) => Err(CliError::Usage(format!("env var is empty: {name}"))),
        Err(env::VarError::NotPresent) => Ok(default_value.to_string()),
        Err(env::VarError::NotUnicode(_)) => Err(CliError::Usage(format!(
            "env var contains invalid unicode: {name}"
        ))),
    }
}

pub(crate) fn env_path(name: &str) -> Result<PathBuf> {
    let value = env::var(name).map_err(|error| match error {
        env::VarError::NotPresent => CliError::Usage(format!("missing env var: {name}")),
        env::VarError::NotUnicode(_) => {
            CliError::Usage(format!("env var contains invalid unicode: {name}"))
        }
    })?;
    if value.is_empty() {
        return Err(CliError::Usage(format!("env var is empty: {name}")));
    }
    Ok(PathBuf::from(value))
}

pub(crate) fn env_path_default(name: &str, default_value: &str) -> Result<PathBuf> {
    match env::var(name) {
        Ok(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        Ok(_) => Err(CliError::Usage(format!("env var is empty: {name}"))),
        Err(env::VarError::NotPresent) => Ok(PathBuf::from(default_value)),
        Err(env::VarError::NotUnicode(_)) => Err(CliError::Usage(format!(
            "env var contains invalid unicode: {name}"
        ))),
    }
}

pub(crate) fn env_u16(name: &str) -> Result<u16> {
    env_u16_default(name, 0)
}

pub(crate) fn env_u16_default(name: &str, default_value: u16) -> Result<u16> {
    match env::var(name) {
        Ok(value) if !value.is_empty() => value
            .parse::<u16>()
            .map_err(|error| CliError::Usage(format!("invalid u16 in env var {name}: {error}"))),
        Ok(_) => Err(CliError::Usage(format!("env var is empty: {name}"))),
        Err(env::VarError::NotPresent) => Ok(default_value),
        Err(env::VarError::NotUnicode(_)) => Err(CliError::Usage(format!(
            "env var contains invalid unicode: {name}"
        ))),
    }
}

pub(crate) fn env_u64_default(name: &str, default_value: u64) -> Result<u64> {
    match env::var(name) {
        Ok(value) if !value.is_empty() => value
            .parse::<u64>()
            .map_err(|error| CliError::Usage(format!("invalid u64 in env var {name}: {error}"))),
        Ok(_) => Err(CliError::Usage(format!("env var is empty: {name}"))),
        Err(env::VarError::NotPresent) => Ok(default_value),
        Err(env::VarError::NotUnicode(_)) => Err(CliError::Usage(format!(
            "env var contains invalid unicode: {name}"
        ))),
    }
}

pub(crate) fn env_usize_default(name: &str, default_value: usize) -> Result<usize> {
    match env::var(name) {
        Ok(value) if !value.is_empty() => value
            .parse::<usize>()
            .map_err(|error| CliError::Usage(format!("invalid usize in env var {name}: {error}"))),
        Ok(_) => Err(CliError::Usage(format!("env var is empty: {name}"))),
        Err(env::VarError::NotPresent) => Ok(default_value),
        Err(env::VarError::NotUnicode(_)) => Err(CliError::Usage(format!(
            "env var contains invalid unicode: {name}"
        ))),
    }
}
