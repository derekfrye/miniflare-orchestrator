use crate::lease_logs::tail_lease_logs;
use crate::lease_manager::{
    HEALTH_CHECK_TIMED_OUT, HTTPS_REDIRECT_MESSAGE, LeaseManager, LeaseRecord,
};
use crate::lease_model::{
    LeaseHealthProbeOutcome, LeaseLaunchDetails, LeaseStartupDiagnosticKind,
    LeaseStartupDiagnostics, LeaseState, LeaseStatus,
};
use crate::lease_runtime::{LeaseLaunchConfig, probe_health_report_with_protocol};
use tokio::sync::OwnedSemaphorePermit;
use tokio::time::Duration;

const STARTUP_ATTEMPTS: u8 = 3;

pub(crate) async fn monitor_lease(
    manager: LeaseManager,
    id: String,
    generation: u64,
    launch: LeaseLaunchConfig,
) {
    let startup_details = manager.startup_details(&launch);
    for attempt in 1..=STARTUP_ATTEMPTS {
        match monitor_startup_attempt(&manager, &id, generation).await {
            StartupAttempt::Ready => return,
            StartupAttempt::Stale => return,
            StartupAttempt::Failed {
                child,
                startup_permit,
                retryable,
            } => {
                kill_child(child).await;
                if retryable && attempt < STARTUP_ATTEMPTS {
                    mark_retrying(&manager, &id, generation).await;
                    sleep_retry_jitter(&id, attempt).await;
                    if retry_startup(
                        &manager,
                        &id,
                        generation,
                        &launch,
                        &startup_details,
                        startup_permit,
                    )
                    .await
                    .is_ok()
                    {
                        continue;
                    }
                }
                manager.retain_current_failure_report(&id).await;
                return;
            }
        }
    }
}

async fn mark_retrying(manager: &LeaseManager, id: &str, generation: u64) {
    let mut store = manager.state.lock().await;
    if let Some(record) = store.leases.get_mut(id)
        && record.generation == generation
    {
        record.status = LeaseStatus::new(LeaseState::Starting);
        record.startup_diagnostics = None;
        record.last_probe = None;
    }
}

enum StartupAttempt {
    Ready,
    Stale,
    Failed {
        child: Option<tokio::process::Child>,
        startup_permit: Option<OwnedSemaphorePermit>,
        retryable: bool,
    },
}

async fn monitor_startup_attempt(
    manager: &LeaseManager,
    id: &str,
    generation: u64,
) -> StartupAttempt {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);

    loop {
        let Some(snapshot) = ({
            let mut store = manager.state.lock().await;
            let record = match store.leases.get_mut(id) {
                Some(record) if record.generation == generation => record,
                _ => return StartupAttempt::Stale,
            };
            if record.child.is_none() {
                None
            } else if let Some(message) = child_exit_message(record) {
                let child = mark_worker_exited(record, &message);
                return StartupAttempt::Failed {
                    child: child.child,
                    startup_permit: child.startup_permit,
                    retryable: true,
                };
            } else {
                Some((
                    record.port,
                    record.health_path.clone(),
                    record.protocol.clone(),
                    record.generation,
                ))
            }
        }) else {
            return StartupAttempt::Stale;
        };

        if snapshot.3 != generation {
            return StartupAttempt::Stale;
        }

        match probe_health_report_with_protocol(snapshot.0, &snapshot.1, &snapshot.2).await {
            Ok(report) => {
                let (child, ready) = {
                    let mut store = manager.state.lock().await;
                    if let Some(record) = store.leases.get_mut(id)
                        && record.generation == generation
                    {
                        record.last_probe = Some(report.clone());
                        apply_probe_outcome(record, report.outcome)
                    } else {
                        (None, false)
                    }
                };
                if let Some(child) = child {
                    return StartupAttempt::Failed {
                        child: child.child,
                        startup_permit: child.startup_permit,
                        retryable: false,
                    };
                }
                if ready {
                    return StartupAttempt::Ready;
                }
            }
            Err(error) => {
                let child = {
                    let mut store = manager.state.lock().await;
                    if let Some(record) = store.leases.get_mut(id)
                        && record.generation == generation
                    {
                        Some(mark_probe_error(record, error.to_string()))
                    } else {
                        None
                    }
                };
                let (child, startup_permit) = split_failed_startup_child(child);
                return StartupAttempt::Failed {
                    child,
                    startup_permit,
                    retryable: false,
                };
            }
        }

        if tokio::time::Instant::now() >= deadline {
            let child = {
                let mut store = manager.state.lock().await;
                if let Some(record) = store.leases.get_mut(id)
                    && record.generation == generation
                {
                    Some(mark_timeout(record))
                } else {
                    None
                }
            };
            let (child, startup_permit) = split_failed_startup_child(child);
            return StartupAttempt::Failed {
                child,
                startup_permit,
                retryable: true,
            };
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn retry_startup(
    manager: &LeaseManager,
    id: &str,
    generation: u64,
    launch: &LeaseLaunchConfig,
    startup_details: &LeaseLaunchDetails,
    startup_permit: Option<OwnedSemaphorePermit>,
) -> Result<(), ()> {
    if let Err(error) = crate::lease_manager::wait_for_ports_available(
        launch.port,
        launch.inspector_port,
        Duration::from_secs(3),
    )
    .await
    {
        let mut store = manager.state.lock().await;
        if let Some(record) = store.leases.get_mut(id)
            && record.generation == generation
        {
            record.startup_details = Some(startup_details.clone());
            record.startup_diagnostics = Some(LeaseStartupDiagnostics {
                kind: LeaseStartupDiagnosticKind::ProbeError,
                message: Some(error.clone()),
                suggested_protocol: None,
            });
            record.status = LeaseStatus::failed(error);
        }
        return Err(());
    }

    let worker = match manager
        .spawn_worker_with_existing_startup_permit(launch, startup_permit)
        .await
    {
        Ok(worker) => worker,
        Err(error) => {
            let mut store = manager.state.lock().await;
            if let Some(record) = store.leases.get_mut(id)
                && record.generation == generation
            {
                record.startup_details = Some(startup_details.clone());
                record.startup_diagnostics = Some(LeaseStartupDiagnostics {
                    kind: LeaseStartupDiagnosticKind::ProbeError,
                    message: Some(error.to_string()),
                    suggested_protocol: None,
                });
                record.status = LeaseStatus::failed(error.to_string());
            }
            return Err(());
        }
    };

    let mut store = manager.state.lock().await;
    if let Some(record) = store.leases.get_mut(id)
        && record.generation == generation
    {
        record.startup_diagnostics = None;
        record.last_probe = None;
        record.status = LeaseStatus::new(LeaseState::Starting);
        record.startup_details = Some(startup_details.clone());
        record.child = Some(worker.child);
        record.startup_permit = Some(worker.startup_permit);
        Ok(())
    } else {
        drop(store);
        kill_child(Some(worker.child)).await;
        Err(())
    }
}

async fn sleep_retry_jitter(id: &str, attempt: u8) {
    let hash = id.bytes().fold(u64::from(attempt), |acc, byte| {
        acc.wrapping_mul(33) ^ u64::from(byte)
    });
    let delay = 100 + (hash % 200);
    tokio::time::sleep(Duration::from_millis(delay)).await;
}

fn child_exit_message(record: &mut LeaseRecord) -> Option<String> {
    match record.child.as_mut() {
        Some(child) => match child.try_wait() {
            Ok(Some(status)) => Some(status.to_string()),
            Ok(None) => None,
            Err(error) => Some(error.to_string()),
        },
        None => None,
    }
}

struct FailedStartupChild {
    child: Option<tokio::process::Child>,
    startup_permit: Option<OwnedSemaphorePermit>,
}

fn split_failed_startup_child(
    child: Option<FailedStartupChild>,
) -> (Option<tokio::process::Child>, Option<OwnedSemaphorePermit>) {
    child
        .map(|child| (child.child, child.startup_permit))
        .unwrap_or((None, None))
}

fn mark_worker_exited(record: &mut LeaseRecord, message: &str) -> FailedStartupChild {
    let message = format!("worker exited: {message}");
    record.startup_diagnostics = Some(LeaseStartupDiagnostics {
        kind: LeaseStartupDiagnosticKind::WorkerExitedEarly,
        message: Some(message.clone()),
        suggested_protocol: None,
    });
    record.status = LeaseStatus::failed(message);
    FailedStartupChild {
        child: record.child.take(),
        startup_permit: record.startup_permit.take(),
    }
}

fn apply_probe_outcome(
    record: &mut LeaseRecord,
    outcome: LeaseHealthProbeOutcome,
) -> (Option<FailedStartupChild>, bool) {
    match outcome {
        LeaseHealthProbeOutcome::Healthy => {
            record.status = LeaseStatus::new(LeaseState::Ready);
            record.startup_permit = None;
            (None, true)
        }
        LeaseHealthProbeOutcome::RedirectedToHttps => {
            (Some(mark_redirected_to_https(record)), false)
        }
        LeaseHealthProbeOutcome::Unhealthy => (None, false),
    }
}

fn mark_redirected_to_https(record: &mut LeaseRecord) -> FailedStartupChild {
    record.startup_diagnostics = Some(LeaseStartupDiagnostics {
        kind: LeaseStartupDiagnosticKind::RedirectedToHttps,
        message: Some(HTTPS_REDIRECT_MESSAGE.to_string()),
        suggested_protocol: Some("https".to_string()),
    });
    record.status = LeaseStatus::failed(HTTPS_REDIRECT_MESSAGE);
    FailedStartupChild {
        child: record.child.take(),
        startup_permit: record.startup_permit.take(),
    }
}

fn mark_probe_error(record: &mut LeaseRecord, message: String) -> FailedStartupChild {
    record.startup_diagnostics = Some(LeaseStartupDiagnostics {
        kind: LeaseStartupDiagnosticKind::ProbeError,
        message: Some(message.clone()),
        suggested_protocol: None,
    });
    record.status = LeaseStatus::failed(message);
    FailedStartupChild {
        child: record.child.take(),
        startup_permit: record.startup_permit.take(),
    }
}

fn mark_timeout(record: &mut LeaseRecord) -> FailedStartupChild {
    record.startup_diagnostics = Some(LeaseStartupDiagnostics {
        kind: LeaseStartupDiagnosticKind::HealthCheckTimedOut,
        message: Some(HEALTH_CHECK_TIMED_OUT.to_string()),
        suggested_protocol: None,
    });
    record.status = LeaseStatus::failed(HEALTH_CHECK_TIMED_OUT);
    FailedStartupChild {
        child: record.child.take(),
        startup_permit: record.startup_permit.take(),
    }
}

async fn kill_child(child: Option<tokio::process::Child>) {
    crate::process_tree::kill_child_process_group(child).await;
}

impl LeaseManager {
    pub(crate) async fn retain_current_failure_report(&self, id: &str) {
        let mut store = self.state.lock().await;
        let Some(record) = store.leases.get(id) else {
            return;
        };
        if record.startup_diagnostics.is_none() {
            return;
        }
        let log_tail = tail_lease_logs(&record.log_dir, 200).ok();
        let report = crate::lease_manager::lease_failure_report(record, log_tail);
        self.retain_failure_report(&mut store, id.to_string(), report);
    }
}
