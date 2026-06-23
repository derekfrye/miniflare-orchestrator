use crate::lease_manager::LeaseError;
use crate::secure_fs;
use crate::time_format::format_epoch_millis;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

type LogFiles = Vec<LogFile>;

struct LogFile {
    relative: PathBuf,
    content: Vec<u8>,
    modified: Option<SystemTime>,
}

/// Reads every regular file under `log_dir` and concatenates them into one log bundle.
///
/// # Errors
///
/// Returns an error if the directory cannot be read or one of the files cannot be read.
pub fn read_lease_logs(log_dir: &Path) -> Result<String, LeaseError> {
    let Some(mut files) = collect_regular_log_files(log_dir)? else {
        return Ok(String::new());
    };
    files.sort_by(|a, b| a.relative.cmp(&b.relative));

    let mut output = String::new();
    for file in files {
        let relative = file.relative;
        let content = file.content;
        let content = String::from_utf8(content)?;
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str("== ");
        output.push_str(&relative.display().to_string());
        output.push_str(" ==\n");
        output.push_str(&content);
        if !output.ends_with('\n') {
            output.push('\n');
        }
    }

    Ok(output)
}

/// Returns the last `max_lines` from the lease log bundle.
///
/// # Errors
///
/// Returns an error if the log directory cannot be read or any file contents cannot be read.
pub fn tail_lease_logs(log_dir: &Path, max_lines: usize) -> Result<String, LeaseError> {
    let lines = collect_tailed_lines(log_dir, max_lines)?;
    Ok(render_tailed_lines(&lines))
}

fn collect_regular_log_files(log_dir: &Path) -> Result<Option<LogFiles>, LeaseError> {
    let root = match secure_fs::open_ambient_dir(log_dir) {
        Ok(root) => root,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    let mut files = Vec::new();
    collect_regular_files(&root, Path::new(""), &mut files)?;
    Ok(Some(files))
}

fn collect_regular_files(
    dir: &cap_std::fs::Dir,
    prefix: &Path,
    files: &mut LogFiles,
) -> Result<(), LeaseError> {
    for entry in dir.entries()? {
        let entry = entry?;
        let relative = prefix.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let child = entry.open_dir()?;
            collect_regular_files(&child, &relative, files)?;
        } else if file_type.is_file() {
            let mut file = entry.open()?;
            let modified = file.metadata()?.modified().ok().map(|time| time.into_std());
            let content = secure_fs::read_file_to_end(&mut file)?;
            files.push(LogFile {
                relative,
                content,
                modified,
            });
        }
    }
    Ok(())
}

fn collect_tailed_lines(log_dir: &Path, max_lines: usize) -> Result<Vec<LogLine>, LeaseError> {
    if max_lines == 0 {
        return Ok(Vec::new());
    }

    let Some(mut files) = collect_regular_log_files(log_dir)? else {
        return Ok(Vec::new());
    };
    files.sort_by(|a, b| a.relative.cmp(&b.relative));

    let mut lines = Vec::new();
    for file in files {
        let content = String::from_utf8(file.content)?;
        let timestamp = format_epoch_millis(file.modified);
        for line in content.lines() {
            lines.push(LogLine {
                timestamp: timestamp.clone(),
                source: file.relative.display().to_string(),
                line: line.to_string(),
            });
        }
    }

    if lines.len() > max_lines {
        lines = lines.split_off(lines.len() - max_lines);
    }
    Ok(lines)
}

fn render_tailed_lines(lines: &[LogLine]) -> String {
    if lines.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    for entry in lines {
        output.push('[');
        output.push_str(entry.timestamp.as_deref().unwrap_or("unknown"));
        output.push_str("] [");
        output.push_str(&entry.source);
        output.push_str("] ");
        output.push_str(&entry.line);
        output.push('\n');
    }
    output
}

#[derive(Debug, Clone)]
struct LogLine {
    timestamp: Option<String>,
    source: String,
    line: String,
}
