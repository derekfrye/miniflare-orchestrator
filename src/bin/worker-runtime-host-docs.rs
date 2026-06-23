#[tokio::main]
async fn main() -> std::process::ExitCode {
    match worker_runtime_host_gen::docs::run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("worker-runtime-host-docs: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}
