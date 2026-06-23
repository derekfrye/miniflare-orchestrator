use std::path::Path;
use tempfile::TempDir;

pub fn bin(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!(concat!("CARGO_BIN_EXE_", env!("CARGO_PKG_NAME"))))
        .with_file_name(name)
}

pub fn make_temp_dir() -> TempDir {
    tempfile::tempdir().expect("tempdir")
}

pub fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(path, contents).expect("write file");
}

pub fn make_executable(path: &Path, contents: &str) {
    use std::os::unix::fs::PermissionsExt;

    write_file(path, contents);
    let mut perms = std::fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("chmod");
}

pub fn prepend_path(dir: &Path) -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    format!("{}:{}", dir.display(), current)
}

pub fn wait_for<F>(timeout: std::time::Duration, mut predicate: F) -> bool
where
    F: FnMut() -> bool,
{
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if predicate() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    false
}
