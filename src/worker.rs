use crate::error::{CliError, Result};
use crate::lease_model::LeaseBackend;
use crate::secure_fs;
use crate::service::WorkerServiceConfig;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const MINIFLARE_RUNNER: &str = include_str!("miniflare_runner.mjs");

#[must_use]
pub fn run_main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("worker-runtime-host-worker: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Starts the project worker service by exec'ing `wrangler dev`.
/// The runtime host consumes already-built bundles and explicitly disables
/// Wrangler build hooks.
///
/// # Errors
///
/// Returns an error if the required environment is missing, the runtime
/// directories cannot be created, or `wrangler` cannot be replaced into the
/// current process.
pub fn run() -> Result<()> {
    let config = WorkerServiceConfig::from_env()?;

    secure_fs::create_ambient_private_dir_all(&config.runtime_dir)?;
    secure_fs::create_ambient_private_dir_all(&config.state_dir)?;
    secure_fs::create_ambient_private_dir_all(&config.log_dir)?;
    let runtime_dir = secure_fs::open_ambient_dir(&config.runtime_dir)?;
    let state_dir = secure_fs::open_ambient_dir(&config.state_dir)?;
    let _log_dir = secure_fs::open_ambient_dir(&config.log_dir)?;

    let config_relative = config
        .config_file
        .strip_prefix(&config.runtime_dir)
        .map_err(|_| {
            CliError::Usage(format!(
                "Wrangler config must live inside runtime dir: {}",
                config.config_file.display()
            ))
        })?;
    if let Err(error) = runtime_dir.open(config_relative) {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error.into());
        }
        return Err(CliError::Usage(format!(
            "missing Wrangler config: {}",
            config.config_file.display()
        )));
    }

    let error = match config.backend {
        LeaseBackend::WranglerDev => wrangler_dev_command(&config).exec(),
        LeaseBackend::Miniflare => miniflare_command(&config, &state_dir)?.exec(),
    };

    Err(error.into())
}

fn wrangler_dev_command(config: &WorkerServiceConfig) -> Command {
    let mut command = Command::new(&config.wrangler_bin);
    command
        .env_clear()
        .arg("dev")
        .arg("--local")
        .arg("--no-bundle")
        .arg("--env")
        .arg(&config.env_name)
        .arg("--config")
        .arg(&config.config_file)
        .arg("--ip")
        .arg("0.0.0.0")
        .arg("--port")
        .arg(config.port.to_string())
        .arg("--inspector-ip")
        .arg("127.0.0.1")
        .arg("--inspector-port")
        .arg(config.inspector_port.to_string())
        .arg("--local-protocol")
        .arg(&config.protocol)
        .arg("--persist-to")
        .arg(&config.state_dir)
        .arg("--log-level")
        .arg(&config.log_level)
        .arg("--show-interactive-dev-session=false")
        .env("HOME", &config.state_dir)
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("TMPDIR", &config.state_dir)
        .env(crate::service::WORKER_RUNTIME_DIR_ENV, &config.runtime_dir)
        .env(crate::service::WORKER_STATE_DIR_ENV, &config.state_dir)
        .env(crate::service::WORKER_LOG_DIR_ENV, &config.log_dir)
        .env(crate::service::WORKER_CONFIG_FILE_ENV, &config.config_file)
        .env(crate::service::WORKER_PORT_ENV, config.port.to_string())
        .env(
            crate::service::WORKER_INSPECTOR_PORT_ENV,
            config.inspector_port.to_string(),
        )
        .env(crate::service::WORKER_ENV_ENV, &config.env_name)
        .env(crate::service::WORKER_PROTOCOL_ENV, &config.protocol)
        .env(crate::service::WORKER_LOG_LEVEL_ENV, &config.log_level)
        .env(
            crate::service::WORKER_WRANGLER_BIN_ENV,
            &config.wrangler_bin,
        )
        .env(
            crate::service::WORKER_RUNTIME_BACKEND_ENV,
            crate::lease_runtime::backend_name(config.backend),
        )
        .current_dir(&config.runtime_dir);
    for (name, value) in &config.env_vars {
        command.env(name, value);
    }
    command
}

fn miniflare_command(
    config: &WorkerServiceConfig,
    state_dir: &cap_std::fs::Dir,
) -> Result<Command> {
    let runner = miniflare_runner_path(config);
    let runner_name = runner.file_name().ok_or_else(|| {
        CliError::Usage("Miniflare runner path is missing a file name".to_string())
    })?;
    secure_fs::write_private_file(state_dir, runner_name, MINIFLARE_RUNNER.as_bytes())?;
    let mut command = Command::new(&config.node_bin);
    command
        .env_clear()
        .arg(&runner)
        .env("HOME", &config.state_dir)
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("TMPDIR", &config.state_dir)
        .env(crate::service::WORKER_RUNTIME_DIR_ENV, &config.runtime_dir)
        .env(crate::service::WORKER_STATE_DIR_ENV, &config.state_dir)
        .env(crate::service::WORKER_LOG_DIR_ENV, &config.log_dir)
        .env(crate::service::WORKER_CONFIG_FILE_ENV, &config.config_file)
        .env(crate::service::WORKER_PORT_ENV, config.port.to_string())
        .env(
            crate::service::WORKER_INSPECTOR_PORT_ENV,
            config.inspector_port.to_string(),
        )
        .env(crate::service::WORKER_ENV_ENV, &config.env_name)
        .env(crate::service::WORKER_PROTOCOL_ENV, &config.protocol)
        .env(crate::service::WORKER_LOG_LEVEL_ENV, &config.log_level)
        .env(
            crate::service::WORKER_WRANGLER_BIN_ENV,
            &config.wrangler_bin,
        )
        .env(crate::service::WORKER_NODE_BIN_ENV, &config.node_bin)
        .env(
            crate::service::WORKER_RUNTIME_BACKEND_ENV,
            crate::lease_runtime::backend_name(config.backend),
        )
        .current_dir(&config.runtime_dir);
    copy_env_if_present(&mut command, crate::service::WORKER_MINIFLARE_MODULE_ENV);
    copy_env_if_present(&mut command, crate::service::WORKER_MINIFLARE_VERBOSE_ENV);
    copy_env_if_present(
        &mut command,
        crate::service::WORKER_MINIFLARE_WORKERD_CONFIG_DEBUG_ENV,
    );
    copy_env_if_present(
        &mut command,
        crate::service::WORKER_MINIFLARE_DISABLE_INSPECTOR_ENV,
    );
    copy_env_if_present(&mut command, crate::service::MINIFLARE_WORKERD_PATH_ENV);
    copy_env_if_present(
        &mut command,
        crate::service::MINIFLARE_WORKERD_CONFIG_DEBUG_ENV,
    );
    for (name, value) in &config.env_vars {
        command.env(name, value);
    }
    Ok(command)
}

fn miniflare_runner_path(config: &WorkerServiceConfig) -> PathBuf {
    config
        .state_dir
        .join(".worker-runtime-host-miniflare-runner.mjs")
}

fn copy_env_if_present(command: &mut Command, name: &str) {
    if let Ok(value) = std::env::var(name) {
        command.env(name, value);
    }
}
