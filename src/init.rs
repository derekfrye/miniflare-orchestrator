use crate::cli::SharedArgs;
use crate::error::Result;
use crate::plan::build_plan;
use crate::render::{render_plan, write_plan_file};
use crate::runtime_mode::RuntimeMode;
use crate::service::env_helpers::{env_path, env_path_default, env_string};
use std::path::PathBuf;

#[must_use]
pub fn run_main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("worker-runtime-host-init: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Runs the container bootstrap phase that generates the per-project services.
///
/// # Errors
///
/// Returns an error if the required environment variables are missing, the
/// manifest is invalid, or the service tree cannot be rendered.
pub fn run() -> Result<()> {
    let mode = RuntimeMode::from_env()?;
    let output_dir = env_path_default(
        "WORKER_RUNTIME_HOST_SERVICES_DIR",
        "/etc/s6-overlay/s6-rc.d",
    )?;
    let plan_file = env_path_default(
        "WORKER_RUNTIME_HOST_PLAN_FILE",
        "/work/host/config/projects.plan.json",
    )?;
    let service_root = env_path_default("WORKER_RUNTIME_HOST_SERVICE_ROOT", "/run/service")?;
    let log_level = env_string("WORKER_RUNTIME_HOST_LOG_LEVEL", "warn")?;
    let plan = match mode {
        RuntimeMode::LeasesOnly => empty_plan(output_dir, plan_file, service_root),
        RuntimeMode::ManifestAndLeases => {
            let shared = SharedArgs {
                manifest: env_path("WORKER_RUNTIME_HOST_MANIFEST")?,
                output_dir,
                plan_file,
                service_root,
                log_level: log_level.clone(),
            };
            build_plan(&shared, false)?
        }
    };
    write_plan_file(&plan.debug_plan_file, &plan)?;
    render_plan(&plan, true, &log_level)?;
    Ok(())
}

fn empty_plan(output_dir: PathBuf, plan_file: PathBuf, service_root: PathBuf) -> crate::plan::Plan {
    crate::plan::Plan {
        manifest: PathBuf::from("/work/host/config/projects.json"),
        output_dir,
        debug_plan_file: plan_file,
        service_root,
        dry_run: false,
        projects: Vec::new(),
    }
}
