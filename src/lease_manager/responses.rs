use crate::lease_manager::LeaseRecord;
use crate::lease_model::{
    LeaseBundleDiagnostic, LeaseBundleDiagnosticKind, LeaseDebugResponse, LeaseFailureReport,
    LeasePrebuiltBundleNotice, LeaseResponse, LeaseState,
};
use crate::secure_fs;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

pub(crate) fn lease_response(record: &LeaseRecord) -> LeaseResponse {
    let scheme = if record.protocol == "https" {
        "https"
    } else {
        "http"
    };
    LeaseResponse {
        id: record.id.clone(),
        name: record.name.clone(),
        port: record.port,
        inspector_port: record.inspector_port,
        base_url: format!("{scheme}://127.0.0.1:{}", record.port),
        health_url: format!("{scheme}://127.0.0.1:{}{}", record.port, record.health_path),
        status: record.status.clone(),
        runtime_dir: record.runtime_dir.display().to_string(),
        static_dir: record.static_dir.display().to_string(),
        state_dir: record.state_dir.display().to_string(),
        log_dir: record.log_dir.display().to_string(),
        health_path: record.health_path.clone(),
        env: record.env.clone(),
        protocol: record.protocol.clone(),
        log_level: record.log_level.clone(),
        env_vars: record.env_vars.clone(),
        persist_state: record.persist_state,
        backend: record.backend,
        prebuilt_bundle_notice: prebuilt_bundle_notice(),
        bundle_metadata: record.bundle_metadata.clone(),
        bundle_uploaded_at: crate::time_format::format_epoch_millis(record.bundle_uploaded_at),
        bundle_diagnostics: bundle_diagnostics(record),
        startup_diagnostics: record.startup_diagnostics.clone(),
    }
}

pub(crate) fn lease_debug_response(record: &LeaseRecord) -> LeaseDebugResponse {
    LeaseDebugResponse {
        lease: lease_response(record),
        startup: record.startup_details.clone(),
        last_probe: record.last_probe.clone(),
    }
}

pub(crate) fn lease_failure_report(
    record: &LeaseRecord,
    log_tail: Option<String>,
) -> LeaseFailureReport {
    LeaseFailureReport {
        lease: lease_response(record),
        startup: record.startup_details.clone(),
        last_probe: record.last_probe.clone(),
        log_tail,
    }
}

pub(crate) fn lease_is_failed(record: &LeaseRecord) -> bool {
    matches!(record.status.state, LeaseState::Failed)
}

fn prebuilt_bundle_notice() -> LeasePrebuiltBundleNotice {
    LeasePrebuiltBundleNotice {
        message: crate::lease_model::PREBUILT_BUNDLE_NOTICE.to_string(),
        required_action: "Rebuild Worker artifacts and upload a fresh bundle before restarting or rerunning tests after Worker source changes.".to_string(),
        applies_to: vec![
            "POST /leases/{id}/bundle".to_string(),
            "POST /leases/{id}/restart".to_string(),
            "GET /leases/{id}/debug".to_string(),
            "GET /leases/{id}/failure-report".to_string(),
        ],
    }
}

pub(crate) fn bundle_diagnostics(record: &LeaseRecord) -> Vec<LeaseBundleDiagnostic> {
    let mut diagnostics = Vec::new();
    let Some(metadata) = &record.bundle_metadata else {
        return diagnostics;
    };

    if metadata.source_paths.is_empty() {
        diagnostics.push(LeaseBundleDiagnostic {
            kind: LeaseBundleDiagnosticKind::MetadataIncomplete,
            message: "Bundle metadata does not include source_root/source_paths, so the host cannot tell whether Worker source changed after this bundle was uploaded.".to_string(),
            paths: Vec::new(),
        });
    }

    diagnostics.extend(missing_artifact_diagnostics(
        metadata.source_root.as_deref(),
        &record.runtime_dir,
        &metadata.artifact_paths,
    ));

    if let Some(uploaded_at) = record.bundle_uploaded_at
        && let Some(stale) = stale_source_diagnostic(
            metadata.source_root.as_deref(),
            &metadata.source_paths,
            uploaded_at,
        )
    {
        diagnostics.push(stale);
    }

    diagnostics
}

fn missing_artifact_diagnostics(
    source_root: Option<&str>,
    runtime_dir: &Path,
    artifact_paths: &[String],
) -> Vec<LeaseBundleDiagnostic> {
    let missing: Vec<String> = artifact_paths
        .iter()
        .filter(|path| !artifact_exists(source_root, runtime_dir, path))
        .cloned()
        .collect();

    if missing.is_empty() {
        Vec::new()
    } else {
        vec![LeaseBundleDiagnostic {
            kind: LeaseBundleDiagnosticKind::ArtifactMissing,
            message: "One or more declared Worker artifact paths are missing from both the uploaded runtime bundle and the caller-provided source root.".to_string(),
            paths: missing,
        }]
    }
}

fn artifact_exists(source_root: Option<&str>, runtime_dir: &Path, path: &str) -> bool {
    let Some(path) = normalize_metadata_path(path) else {
        return false;
    };

    metadata_exists(runtime_dir, &path)
        || source_root.is_some_and(|root| metadata_exists(Path::new(root), &path))
}

fn stale_source_diagnostic(
    source_root: Option<&str>,
    source_paths: &[String],
    uploaded_at: SystemTime,
) -> Option<LeaseBundleDiagnostic> {
    let source_root = source_root?;
    let source_root = secure_fs::open_ambient_dir(Path::new(source_root)).ok()?;
    let stale_paths: Vec<String> = source_paths
        .iter()
        .filter_map(|path| {
            let resolved = normalize_metadata_path(path)?;
            let modified = source_root
                .metadata(&resolved)
                .ok()?
                .modified()
                .ok()?
                .into_std();
            (modified > uploaded_at).then(|| path.clone())
        })
        .collect();

    (!stale_paths.is_empty()).then(|| LeaseBundleDiagnostic {
        kind: LeaseBundleDiagnosticKind::PossiblyStaleBundle,
        message: "Worker source files changed after the uploaded bundle. Rebuild the Worker artifact and POST /leases/{id}/bundle again before rerunning tests.".to_string(),
        paths: stale_paths,
    })
}

fn metadata_exists(root: &Path, path: &Path) -> bool {
    secure_fs::open_ambient_dir(root)
        .and_then(|root| root.metadata(path))
        .is_ok()
}

fn normalize_metadata_path(path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return None;
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }

    (!normalized.as_os_str().is_empty()).then_some(normalized)
}
