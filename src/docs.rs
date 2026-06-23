use crate::docs_routes::{DocsState, load_plan, router};
use crate::error::Result;
use crate::service::DocsServiceConfig;
use std::sync::Arc;

#[must_use]
pub fn run_main() -> std::process::ExitCode {
    match tokio::runtime::Runtime::new() {
        Ok(runtime) => match runtime.block_on(run()) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("worker-runtime-host-docs: {err}");
                std::process::ExitCode::FAILURE
            }
        },
        Err(err) => {
            eprintln!("worker-runtime-host-docs: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Serves runtime host documentation and machine-readable instructions.
///
/// # Errors
///
/// Returns an error if the plan cannot be loaded or the server cannot bind.
pub async fn run() -> Result<()> {
    let config = DocsServiceConfig::from_env()?;
    let plan = load_plan(&config.plan_file)?;
    let state = Arc::new(DocsState::new(config, plan));
    let listener = tokio::net::TcpListener::bind(state.config.socket_addr()).await?;
    axum::serve(listener, router(state)).await?;
    Ok(())
}

pub fn app(config: DocsServiceConfig, plan: crate::plan::Plan) -> axum::Router {
    router(Arc::new(DocsState::new(config, plan)))
}
