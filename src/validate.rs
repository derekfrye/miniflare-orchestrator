use crate::error::{CliError, Result};
use crate::manifest::ProjectInput;
use std::env;
use std::path::{Path, PathBuf};

/// Validates a project name for use in the generated service tree.
///
/// # Errors
///
/// Returns an error if the name is empty or contains unsupported characters.
pub fn validate_project_name(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        return Err(CliError::Usage(format!("invalid project name: {name}")));
    }
    Ok(())
}

/// Validates that a project port is usable.
///
/// # Errors
///
/// Returns an error if the port is zero.
pub fn validate_port(name: &str, port: u16) -> Result<()> {
    if port == 0 {
        return Err(CliError::Usage(format!(
            "project port out of range for {name}: {port}"
        )));
    }
    Ok(())
}

/// Validates that the project paths match the expected host layout.
///
/// # Errors
///
/// Returns an error if any configured path is outside `/work/host` or does not
/// match the project naming convention.
pub fn validate_project_paths(project: &ProjectInput) -> Result<()> {
    validate_host_path(&project.runtime_dir)?;
    validate_host_path(&project.state_dir)?;
    validate_host_path(&project.log_dir)?;
    validate_host_path(&project.static_dir)?;
    validate_host_path(&project.config_file)?;
    validate_host_path(&project.reload_token)?;

    let host_root = host_root_path();
    let expected_runtime = host_root
        .join("projects")
        .join(&project.name)
        .join("runtime");
    let expected_state = host_root.join("projects").join(&project.name).join("state");
    let expected_log = host_root.join("projects").join(&project.name).join("logs");
    let expected_static = host_root
        .join("projects")
        .join(&project.name)
        .join("static");

    if project.runtime_dir != expected_runtime {
        return Err(CliError::Usage(format!(
            "runtime_dir does not match project name {}: {}",
            project.name,
            project.runtime_dir.display()
        )));
    }
    if project.state_dir != expected_state {
        return Err(CliError::Usage(format!(
            "state_dir does not match project name {}: {}",
            project.name,
            project.state_dir.display()
        )));
    }
    if project.log_dir != expected_log {
        return Err(CliError::Usage(format!(
            "log_dir does not match project name {}: {}",
            project.name,
            project.log_dir.display()
        )));
    }
    if project.static_dir != expected_static {
        return Err(CliError::Usage(format!(
            "static_dir does not match project name {}: {}",
            project.name,
            project.static_dir.display()
        )));
    }

    if project.config_file != project.runtime_dir.join("wrangler.toml") {
        return Err(CliError::Usage(format!(
            "config_file must live inside the runtime dir for {}: {}",
            project.name,
            project.config_file.display()
        )));
    }

    if project.reload_token != project.runtime_dir.join(".reload-token") {
        return Err(CliError::Usage(format!(
            "reload_token must live inside the runtime dir for {}: {}",
            project.name,
            project.reload_token.display()
        )));
    }

    if project.config_file.parent() != Some(project.runtime_dir.as_path()) {
        return Err(CliError::Usage(format!(
            "config_file must be a direct child of runtime_dir for {}: {}",
            project.name,
            project.config_file.display()
        )));
    }

    Ok(())
}

fn validate_host_path(path: &Path) -> Result<()> {
    let host_root = host_root_path();
    if !path.starts_with(&host_root) {
        return Err(CliError::Usage(format!(
            "path is outside {}: {}",
            host_root.display(),
            path.display()
        )));
    }
    Ok(())
}

fn host_root_path() -> PathBuf {
    env::var("WORKER_RUNTIME_HOST_ROOT").map_or_else(|_| PathBuf::from("/work/host"), PathBuf::from)
}
