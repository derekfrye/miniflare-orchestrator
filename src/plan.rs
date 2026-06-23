use crate::cli::SharedArgs;
use crate::error::{CliError, Result};
use crate::manifest::{Manifest, ProjectInput, read_manifest};
use crate::validate::{validate_port, validate_project_name, validate_project_paths};
use schemars::JsonSchema;
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;

const PROJECT_INSPECTOR_PORT_START: u16 = 9100;

#[derive(Debug, Clone, Serialize, serde::Deserialize, JsonSchema)]
pub struct Plan {
    pub manifest: PathBuf,
    pub output_dir: PathBuf,
    pub debug_plan_file: PathBuf,
    pub service_root: PathBuf,
    pub dry_run: bool,
    pub projects: Vec<PlannedProject>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, JsonSchema)]
pub struct PlannedProject {
    pub name: String,
    pub runtime_dir: PathBuf,
    pub state_dir: PathBuf,
    pub log_dir: PathBuf,
    pub static_dir: PathBuf,
    pub config_file: PathBuf,
    pub reload_token: PathBuf,
    pub health_path: String,
    pub port: u16,
    pub inspector_port: u16,
    pub env: String,
    pub protocol: String,
    pub worker_service: PathBuf,
    pub watcher_service: PathBuf,
    pub worker_run: PathBuf,
    pub watcher_run: PathBuf,
}

/// Validates the manifest and constructs the in-memory execution plan.
///
/// # Errors
///
/// Returns an error if the manifest is missing, malformed, or violates the
/// project naming and path constraints.
pub fn build_plan(shared: &SharedArgs, dry_run: bool) -> Result<Plan> {
    let manifest = read_manifest(&shared.manifest)?;
    validate_manifest_root(&shared.manifest, &manifest)?;

    let mut seen_names = HashSet::new();
    let mut seen_ports = HashSet::new();
    let mut seen_inspector_ports = HashSet::new();
    let mut projects = Vec::with_capacity(manifest.projects.len());

    for (index, project) in manifest.projects.into_iter().enumerate() {
        validate_project_name(&project.name)?;
        validate_port(&project.name, project.port)?;
        let inspector_port = PROJECT_INSPECTOR_PORT_START
            .checked_add(
                u16::try_from(index)
                    .map_err(|_| CliError::Usage("too many manifest projects".to_string()))?,
            )
            .ok_or_else(|| CliError::Usage("too many manifest projects".to_string()))?;
        validate_port(&format!("{} inspector", project.name), inspector_port)?;

        if !seen_names.insert(project.name.clone()) {
            return Err(CliError::Usage(format!(
                "duplicate project name in manifest: {}",
                project.name
            )));
        }

        if !seen_ports.insert(project.port) {
            return Err(CliError::Usage(format!(
                "duplicate project port in manifest: {}",
                project.port
            )));
        }
        if project.port == inspector_port || !seen_inspector_ports.insert(inspector_port) {
            return Err(CliError::Usage(format!(
                "duplicate project inspector port in manifest plan: {inspector_port}"
            )));
        }

        validate_project_paths(&project)?;
        projects.push(to_planned_project(shared, project, inspector_port));
    }

    Ok(Plan {
        manifest: shared.manifest.clone(),
        output_dir: shared.output_dir.clone(),
        debug_plan_file: shared.plan_file.clone(),
        service_root: shared.service_root.clone(),
        dry_run,
        projects,
    })
}

fn validate_manifest_root(path: &std::path::Path, manifest: &Manifest) -> Result<()> {
    if manifest.projects.is_empty() {
        return Err(CliError::Usage(format!(
            "manifest contains no projects: {}",
            path.display()
        )));
    }
    Ok(())
}

fn to_planned_project(
    shared: &SharedArgs,
    project: ProjectInput,
    inspector_port: u16,
) -> PlannedProject {
    let worker_service = shared.output_dir.join(&project.name);
    let watcher_service = shared.output_dir.join(format!("{}-watch", project.name));

    PlannedProject {
        name: project.name,
        runtime_dir: project.runtime_dir,
        state_dir: project.state_dir,
        log_dir: project.log_dir,
        static_dir: project.static_dir,
        config_file: project.config_file,
        reload_token: project.reload_token,
        health_path: project.health_path,
        port: project.port,
        inspector_port,
        env: project.env,
        protocol: project.protocol,
        worker_service: worker_service.clone(),
        watcher_service: watcher_service.clone(),
        worker_run: worker_service.join("run"),
        watcher_run: watcher_service.join("run"),
    }
}
