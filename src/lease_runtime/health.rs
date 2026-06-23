use crate::lease_manager::{HEALTH_CHECK_TIMED_OUT, HTTPS_REDIRECT_MESSAGE, LeaseError};
use crate::lease_model::{LeaseHealthProbeOutcome, LeaseHealthProbeReport};
use reqwest::header::LOCATION;
use std::collections::BTreeMap;
use tokio::process::Child;
use tokio::time::{Duration, Instant, sleep};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthProbeOutcome {
    Healthy,
    Unhealthy,
    RedirectedToHttps,
}

/// Probes a lease worker's HTTP health endpoint.
///
/// # Errors
///
/// Returns an error if the TCP connection or HTTP I/O fails unexpectedly.
pub async fn probe_health(port: u16, health_path: &str) -> Result<bool, LeaseError> {
    Ok(matches!(
        probe_health_with_protocol(port, health_path, "http").await?,
        HealthProbeOutcome::Healthy
    ))
}

/// Probes a lease worker's HTTP health endpoint and returns the full request
/// and response details.
///
/// # Errors
///
/// Returns an error if the client cannot be constructed.
pub async fn probe_health_report(
    port: u16,
    health_path: &str,
) -> Result<LeaseHealthProbeReport, LeaseError> {
    probe_health_report_with_protocol(port, health_path, "http").await
}

/// Probes a lease worker's HTTP or HTTPS health endpoint.
///
/// # Errors
///
/// Returns an error if the client cannot be constructed.
pub async fn probe_health_with_protocol(
    port: u16,
    health_path: &str,
    protocol: &str,
) -> Result<HealthProbeOutcome, LeaseError> {
    Ok(
        match probe_health_report_with_protocol(port, health_path, protocol)
            .await?
            .outcome
        {
            LeaseHealthProbeOutcome::Healthy => HealthProbeOutcome::Healthy,
            LeaseHealthProbeOutcome::Unhealthy => HealthProbeOutcome::Unhealthy,
            LeaseHealthProbeOutcome::RedirectedToHttps => HealthProbeOutcome::RedirectedToHttps,
        },
    )
}

/// Probes a lease worker's HTTP or HTTPS health endpoint and returns the full
/// request and response details.
///
/// # Errors
///
/// Returns an error if the client cannot be constructed.
pub async fn probe_health_report_with_protocol(
    port: u16,
    health_path: &str,
    protocol: &str,
) -> Result<LeaseHealthProbeReport, LeaseError> {
    let scheme = if protocol == "https" { "https" } else { "http" };
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(scheme == "https")
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_millis(250))
        .build()
        .map_err(|error| LeaseError::process(error.to_string()))?;
    let request_url = format!("{scheme}://127.0.0.1:{port}{health_path}");
    match client.get(&request_url).send().await {
        Ok(response) => {
            let status_code = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .filter_map(|(name, value)| {
                    value
                        .to_str()
                        .ok()
                        .map(|value| (name.to_string(), value.to_string()))
                })
                .collect::<BTreeMap<_, _>>();
            let redirect_target = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .map(std::string::ToString::to_string);
            let outcome = if response.status() == reqwest::StatusCode::OK {
                LeaseHealthProbeOutcome::Healthy
            } else if protocol == "http"
                && response.status().is_redirection()
                && redirect_target
                    .as_deref()
                    .is_some_and(|value| value.starts_with("https://"))
            {
                LeaseHealthProbeOutcome::RedirectedToHttps
            } else {
                LeaseHealthProbeOutcome::Unhealthy
            };

            Ok(LeaseHealthProbeReport {
                request_url,
                request_method: "GET".to_string(),
                protocol: protocol.to_string(),
                health_path: health_path.to_string(),
                outcome,
                status_code: Some(status_code),
                headers,
                redirect_target,
                error: None,
            })
        }
        Err(error) => Ok(LeaseHealthProbeReport {
            request_url,
            request_method: "GET".to_string(),
            protocol: protocol.to_string(),
            health_path: health_path.to_string(),
            outcome: LeaseHealthProbeOutcome::Unhealthy,
            status_code: None,
            headers: BTreeMap::new(),
            redirect_target: None,
            error: Some(error.to_string()),
        }),
    }
}

/// Waits for a lease worker to report healthy.
///
/// # Errors
///
/// Returns an error if the worker exits early or the health check times out.
pub async fn wait_for_ready(
    child: &mut Child,
    port: u16,
    health_path: &str,
    protocol: &str,
    timeout: Duration,
) -> Result<(), LeaseError> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Err(LeaseError::process(format!(
                "worker exited before becoming ready: {status}"
            )));
        }

        let report = probe_health_report_with_protocol(port, health_path, protocol).await?;
        match report.outcome {
            LeaseHealthProbeOutcome::Healthy => return Ok(()),
            LeaseHealthProbeOutcome::RedirectedToHttps => {
                return Err(LeaseError::process(format!(
                    "health probe for port {port}{health_path} redirected to HTTPS; {HTTPS_REDIRECT_MESSAGE}"
                )));
            }
            LeaseHealthProbeOutcome::Unhealthy => {}
        }

        if Instant::now() >= deadline {
            return Err(LeaseError::unavailable(format!(
                "{HEALTH_CHECK_TIMED_OUT} for port {port}"
            )));
        }

        sleep(Duration::from_millis(100)).await;
    }
}
#[cfg(test)]
#[path = "health_tests.rs"]
mod tests;
