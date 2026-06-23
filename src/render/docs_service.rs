use crate::error::Result;
use crate::plan::Plan;
use crate::service::{
    DOCS_BIND_ENV, DOCS_HOST_ROOT_ENV, DOCS_LEASE_PORT_END_ENV, DOCS_LEASE_PORT_START_ENV,
    DOCS_LEASE_ROOT_ENV, DOCS_PLAN_FILE_ENV, DOCS_PORT_ENV, DOCS_WORKER_BIN_ENV,
    DOCS_WRANGLER_BIN_ENV,
};
use std::path::{Path, PathBuf};

pub(super) fn render(plan: &Plan) -> Result<()> {
    let docs_service = plan.output_dir.join("worker-runtime-host-docs");
    let host_root = host_root_from_plan(plan);

    std::fs::create_dir_all(&docs_service)?;
    let docs_env_dir = docs_service.join("env");
    super::write_env_text(&docs_env_dir, DOCS_BIND_ENV, "0.0.0.0")?;
    super::write_env_text(&docs_env_dir, DOCS_PORT_ENV, "8786")?;
    super::write_env_file(
        &docs_env_dir,
        DOCS_PLAN_FILE_ENV,
        plan.debug_plan_file.as_path(),
    )?;
    super::write_env_file(&docs_env_dir, DOCS_HOST_ROOT_ENV, host_root.as_path())?;
    let lease_root = host_root.join("leases");
    super::write_env_file(&docs_env_dir, DOCS_LEASE_ROOT_ENV, lease_root.as_path())?;
    super::write_env_text(&docs_env_dir, DOCS_LEASE_PORT_START_ENV, "8900")?;
    super::write_env_text(&docs_env_dir, DOCS_LEASE_PORT_END_ENV, "8999")?;
    super::write_env_text(
        &docs_env_dir,
        DOCS_WORKER_BIN_ENV,
        "/usr/local/bin/worker-runtime-host-worker",
    )?;
    super::write_env_text(&docs_env_dir, DOCS_WRANGLER_BIN_ENV, "wrangler")?;
    super::write_longrun_service(
        &docs_service,
        Path::new("/usr/local/bin/worker-runtime-host-docs"),
        &[],
    )?;
    super::write_bundle_member(plan.output_dir.as_path(), "worker-runtime-host-docs")?;
    Ok(())
}

fn host_root_from_plan(plan: &Plan) -> PathBuf {
    plan.manifest
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| PathBuf::from("/work/host"), Path::to_path_buf)
}
