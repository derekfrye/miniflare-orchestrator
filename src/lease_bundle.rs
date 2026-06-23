use crate::lease_manager::LeaseError;
use crate::lease_model::{LeaseBundleRequest, LeaseFile};
use crate::secure_fs;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// Writes the lease bundle into the runtime and static directories.
///
/// # Errors
///
/// Returns an error if any directory cannot be replaced or a bundle file path
/// is invalid.
pub fn write_bundle_files(
    runtime_dir: &Path,
    static_dir: &Path,
    bundle: &LeaseBundleRequest,
) -> Result<(), LeaseError> {
    replace_dir_contents(runtime_dir, &bundle.runtime_files)?;
    replace_dir_contents(static_dir, &bundle.static_files)?;
    Ok(())
}

fn replace_dir_contents(dir: &Path, files: &[LeaseFile]) -> Result<(), LeaseError> {
    let root = secure_fs::open_ambient_dir(dir)?;
    secure_fs::clear_dir_contents(&root)?;
    let mut seen_paths = BTreeSet::new();

    for file in files {
        let path = normalize_relative_path(&file.path)?;
        if !seen_paths.insert(path.clone()) {
            return Err(LeaseError::usage(format!(
                "bundle contains duplicate file path: {}",
                path.display()
            )));
        }
        let parent = path.parent().unwrap_or_else(|| Path::new(""));
        let file_name = path
            .file_name()
            .ok_or_else(|| LeaseError::usage("bundle file path is missing a file name"))?;
        let parent_dir = secure_fs::create_private_dir_all(&root, parent)?;
        let bytes = STANDARD.decode(file.content_b64.as_bytes())?;
        secure_fs::create_private_file_new(&parent_dir, file_name, &bytes)?;
    }
    Ok(())
}

fn normalize_relative_path(path: &str) -> Result<PathBuf, LeaseError> {
    let candidate = Path::new(path);
    if candidate.as_os_str().is_empty() {
        return Err(LeaseError::usage("bundle file path is empty"));
    }
    if candidate.is_absolute() {
        return Err(LeaseError::usage(format!(
            "bundle file path must be relative: {path}"
        )));
    }

    let mut normalized = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            _ => {
                return Err(LeaseError::usage(format!(
                    "bundle file path contains an invalid segment: {path}"
                )));
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err(LeaseError::usage(format!(
            "bundle file path resolved empty: {path}"
        )));
    }
    Ok(normalized)
}
