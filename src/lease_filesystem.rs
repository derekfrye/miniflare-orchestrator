use crate::lease_manager::LeaseError;
use crate::lease_model::{LeaseFilesystemEntry, LeaseFilesystemEntryKind, LeaseFilesystemSnapshot};
use crate::secure_fs;
use crate::time_format::format_epoch_millis;
use std::path::{Path, PathBuf};

/// Builds a recursive snapshot of the lease runtime directories.
///
/// # Errors
///
/// Returns an error if any directory cannot be read.
pub fn snapshot_lease_filesystem(
    runtime_dir: &Path,
    static_dir: &Path,
    state_dir: &Path,
    log_dir: &Path,
) -> Result<LeaseFilesystemSnapshot, LeaseError> {
    let mut entries = Vec::new();
    collect_tree("runtime", runtime_dir, &mut entries)?;
    collect_tree("static", static_dir, &mut entries)?;
    collect_tree("state", state_dir, &mut entries)?;
    collect_tree("logs", log_dir, &mut entries)?;
    entries.sort_by(|a, b| a.root.cmp(&b.root).then(a.path.cmp(&b.path)));

    Ok(LeaseFilesystemSnapshot {
        runtime_dir: runtime_dir.display().to_string(),
        static_dir: static_dir.display().to_string(),
        state_dir: state_dir.display().to_string(),
        log_dir: log_dir.display().to_string(),
        entries,
    })
}

fn collect_tree(
    root_name: &str,
    root: &Path,
    entries: &mut Vec<LeaseFilesystemEntry>,
) -> Result<(), LeaseError> {
    let root_dir = match secure_fs::open_ambient_dir(root) {
        Ok(root_dir) => root_dir,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };

    collect_open_tree(root_name, Path::new(""), &root_dir, entries)
}

fn collect_open_tree(
    root_name: &str,
    prefix: &Path,
    dir: &cap_std::fs::Dir,
    entries: &mut Vec<LeaseFilesystemEntry>,
) -> Result<(), LeaseError> {
    for entry in dir.entries()? {
        let entry = entry?;
        let relative_path = prefix.join(entry.file_name());
        let relative = relative_to_string(&relative_path);
        let file_type = entry.file_type()?;
        let metadata = entry.metadata()?;
        if file_type.is_dir() {
            entries.push(LeaseFilesystemEntry {
                root: root_name.to_string(),
                path: relative.clone(),
                kind: LeaseFilesystemEntryKind::Directory,
                size: None,
                modified: format_epoch_millis(metadata.modified().ok().map(|time| time.into_std())),
            });
            let child = entry.open_dir()?;
            collect_open_tree(root_name, &relative_path, &child, entries)?;
        } else if file_type.is_file() {
            entries.push(LeaseFilesystemEntry {
                root: root_name.to_string(),
                path: relative,
                kind: LeaseFilesystemEntryKind::File,
                size: Some(metadata.len()),
                modified: format_epoch_millis(metadata.modified().ok().map(|time| time.into_std())),
            });
        }
    }
    Ok(())
}

fn relative_to_string(path: &Path) -> String {
    let value = PathBuf::from(path);
    value.display().to_string()
}
