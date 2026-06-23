use crate::error::{CliError, Result};
use crate::service::WatchServiceConfig;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

#[must_use]
pub fn run_main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("worker-runtime-host-watch: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Watches the reload token and requests service restarts when it changes.
///
/// # Errors
///
/// Returns an error if the watcher cannot be created or the required
/// environment is missing.
pub fn run() -> Result<()> {
    let config = WatchServiceConfig::from_env()?;
    ensure_reload_token(&config.reload_token)?;

    let watch_dir = config.reload_token.parent().ok_or_else(|| {
        CliError::Usage(format!(
            "reload token must have a parent directory: {}",
            config.reload_token.display()
        ))
    })?;

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |result| {
            let _ = tx.send(result);
        },
        Config::default(),
    )?;

    watcher.watch(watch_dir, RecursiveMode::NonRecursive)?;

    loop {
        match rx.recv() {
            Ok(Ok(event)) if event_matches(&event, &config.reload_token) => {
                drain_event_burst(&rx, &config.reload_token)?;
                restart_worker(&config.worker_service);
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                eprintln!("worker-runtime-host-watch: watcher error: {error}");
            }
            Err(error) => {
                return Err(CliError::Usage(format!("watcher channel closed: {error}")));
            }
        }
    }
}

fn ensure_reload_token(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = match fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    file.write_all(b"initial\n")?;
    Ok(())
}

fn event_matches(event: &Event, token: &Path) -> bool {
    event.paths.iter().any(|path| path == token)
        && matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
        )
}

fn drain_event_burst(rx: &mpsc::Receiver<notify::Result<Event>>, token: &Path) -> Result<()> {
    let deadline = Instant::now() + Duration::from_millis(100);

    loop {
        let now = Instant::now();
        if now >= deadline {
            return Ok(());
        }

        match rx.recv_timeout(deadline.saturating_duration_since(now)) {
            Ok(Ok(event)) if event_matches(&event, token) => {}
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                eprintln!("worker-runtime-host-watch: watcher error: {error}");
            }
            Err(RecvTimeoutError::Timeout) => return Ok(()),
            Err(RecvTimeoutError::Disconnected) => {
                return Err(CliError::Usage("watcher channel disconnected".to_string()));
            }
        }
    }
}

fn restart_worker(worker_service: &Path) {
    match Command::new("s6-svc")
        .arg("-r")
        .arg(worker_service)
        .status()
    {
        Ok(status) if !status.success() => {
            eprintln!(
                "worker-runtime-host-watch: restart request failed for {}: {status}",
                worker_service.display()
            );
        }
        Ok(_) => {}
        Err(error) => {
            eprintln!(
                "worker-runtime-host-watch: could not request restart for {}: {error}",
                worker_service.display()
            );
        }
    }
}
