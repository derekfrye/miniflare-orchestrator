use std::time::SystemTime;

#[must_use]
pub fn format_epoch_millis(time: Option<SystemTime>) -> Option<String> {
    let time = time?;
    let duration = time.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    let seconds = duration.as_secs();
    let millis = duration.subsec_millis();
    Some(format!("{seconds}.{millis:03}Z"))
}
