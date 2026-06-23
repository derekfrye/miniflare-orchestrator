mod docs_service;

use crate::error::Result;
use crate::plan::{Plan, PlannedProject};
use crate::service::{
    WATCH_RELOAD_TOKEN_ENV, WATCH_WORKER_SERVICE_ENV, WORKER_CONFIG_FILE_ENV, WORKER_ENV_ENV,
    WORKER_INSPECTOR_PORT_ENV, WORKER_LOG_DIR_ENV, WORKER_LOG_LEVEL_ENV, WORKER_PORT_ENV,
    WORKER_PROTOCOL_ENV, WORKER_RUNTIME_DIR_ENV, WORKER_STATE_DIR_ENV, WORKER_WRANGLER_BIN_ENV,
};
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// Renders the s6 service directories for the validated plan.
///
/// # Errors
///
/// Returns an error if any service directory, env file, or symlink cannot be
/// created.
pub fn render_plan(plan: &Plan, create_project_dirs: bool, log_level: &str) -> Result<()> {
    fs::create_dir_all(&plan.output_dir)?;
    fs::create_dir_all(plan.output_dir.join("user/contents.d"))?;
    fs::create_dir_all(
        plan.debug_plan_file
            .parent()
            .unwrap_or_else(|| Path::new(".")),
    )?;

    for project in &plan.projects {
        render_project_service(plan, project, create_project_dirs, log_level)?;
    }

    docs_service::render(plan)?;

    Ok(())
}

/// Writes the debug plan JSON to disk.
///
/// # Errors
///
/// Returns an error if the destination directory or file cannot be written.
pub fn write_plan_file(path: &Path, plan: &Plan) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(plan)?)?;
    Ok(())
}

fn render_project_service(
    plan: &Plan,
    project: &PlannedProject,
    create_project_dirs: bool,
    log_level: &str,
) -> Result<()> {
    if create_project_dirs {
        create_project_dirs_for(project)?;
    }

    fs::create_dir_all(&project.worker_service)?;
    fs::create_dir_all(&project.watcher_service)?;

    write_worker_env(project, log_level)?;
    write_watcher_env(plan, project)?;

    write_longrun_service(
        &project.worker_service,
        Path::new("/usr/local/bin/worker-runtime-host-worker"),
        &[],
    )?;
    write_longrun_service(
        &project.watcher_service,
        Path::new("/usr/local/bin/worker-runtime-host-watch"),
        &[&project.name],
    )?;
    write_bundle_member(&plan.output_dir, &project.name)?;
    write_bundle_member(&plan.output_dir, &format!("{}-watch", project.name))?;
    Ok(())
}

fn create_project_dirs_for(project: &PlannedProject) -> Result<()> {
    fs::create_dir_all(&project.runtime_dir)?;
    fs::create_dir_all(&project.state_dir)?;
    fs::create_dir_all(&project.log_dir)?;
    fs::create_dir_all(&project.static_dir)?;
    Ok(())
}

fn write_worker_env(project: &PlannedProject, log_level: &str) -> Result<()> {
    let worker_env_dir = project.worker_service.join("env");
    write_env_file(
        &worker_env_dir,
        WORKER_RUNTIME_DIR_ENV,
        project.runtime_dir.as_path(),
    )?;
    write_env_file(
        &worker_env_dir,
        WORKER_STATE_DIR_ENV,
        project.state_dir.as_path(),
    )?;
    write_env_file(
        &worker_env_dir,
        WORKER_LOG_DIR_ENV,
        project.log_dir.as_path(),
    )?;
    write_env_file(
        &worker_env_dir,
        WORKER_CONFIG_FILE_ENV,
        project.config_file.as_path(),
    )?;
    write_env_file(
        &worker_env_dir,
        WORKER_PORT_ENV,
        project.port.to_string().as_str(),
    )?;
    write_env_file(
        &worker_env_dir,
        WORKER_INSPECTOR_PORT_ENV,
        project.inspector_port.to_string().as_str(),
    )?;
    write_env_file(&worker_env_dir, WORKER_ENV_ENV, project.env.as_str())?;
    write_env_file(
        &worker_env_dir,
        WORKER_PROTOCOL_ENV,
        project.protocol.as_str(),
    )?;
    write_env_file(&worker_env_dir, WORKER_LOG_LEVEL_ENV, log_level)?;
    write_env_file(&worker_env_dir, WORKER_WRANGLER_BIN_ENV, "wrangler")?;
    Ok(())
}

fn write_watcher_env(plan: &Plan, project: &PlannedProject) -> Result<()> {
    let watcher_env_dir = project.watcher_service.join("env");
    write_env_file(
        &watcher_env_dir,
        WATCH_RELOAD_TOKEN_ENV,
        project.reload_token.as_path(),
    )?;
    let worker_service = plan.service_root.join(&project.name);
    write_env_file(
        &watcher_env_dir,
        WATCH_WORKER_SERVICE_ENV,
        worker_service.as_path(),
    )?;
    Ok(())
}

trait EnvValue {
    fn write_env_value(&self, output: &mut Vec<u8>);
}

impl EnvValue for str {
    fn write_env_value(&self, output: &mut Vec<u8>) {
        output.extend_from_slice(self.as_bytes());
    }
}

impl EnvValue for Path {
    fn write_env_value(&self, output: &mut Vec<u8>) {
        #[cfg(unix)]
        output.extend_from_slice(self.as_os_str().as_bytes());
        #[cfg(not(unix))]
        output.extend_from_slice(self.display().to_string().as_bytes());
    }
}

fn write_env_file<T: EnvValue + ?Sized>(dir: &Path, name: &str, value: &T) -> Result<()> {
    fs::create_dir_all(dir)?;
    let path = dir.join(name);
    let mut bytes = Vec::new();
    value.write_env_value(&mut bytes);
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn write_env_text(dir: &Path, name: &str, value: &str) -> Result<()> {
    write_env_file(dir, name, value)
}

fn write_longrun_service(service_dir: &Path, command: &Path, dependencies: &[&str]) -> Result<()> {
    fs::create_dir_all(service_dir)?;
    fs::write(service_dir.join("type"), "longrun\n")?;
    write_run_script(service_dir, command)?;
    write_dependencies(service_dir, dependencies)?;
    Ok(())
}

fn write_run_script(service_dir: &Path, command: &Path) -> Result<()> {
    let run_path = service_dir.join("run");
    let env_dir = service_dir.join("env");
    let script = format!(
        "#!/bin/sh\nexec /command/s6-envdir {} {}\n",
        shell_quote(&env_dir)?,
        shell_quote(command)?,
    );
    #[cfg(unix)]
    {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o755)
            .open(&run_path)?;
        file.write_all(script.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&run_path, script)?;
    }
    Ok(())
}

fn write_dependencies(service_dir: &Path, dependencies: &[&str]) -> Result<()> {
    let dependencies_dir = service_dir.join("dependencies.d");
    fs::create_dir_all(&dependencies_dir)?;
    fs::write(dependencies_dir.join("base"), "")?;
    for dependency in dependencies {
        fs::write(dependencies_dir.join(dependency), "")?;
    }
    Ok(())
}

fn write_bundle_member(output_dir: &Path, service_name: &str) -> Result<()> {
    fs::write(output_dir.join("user/contents.d").join(service_name), "")?;
    Ok(())
}

fn shell_quote(path: &Path) -> Result<String> {
    let value = path.to_str().ok_or_else(|| {
        crate::error::CliError::Usage(format!(
            "service path is not valid UTF-8 for shell script: {}",
            path.display()
        ))
    })?;
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
}
