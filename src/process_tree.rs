use tokio::process::Child;
#[cfg(unix)]
use tokio::process::Command;
use tokio::time::{Duration, timeout};

const TERMINATION_GRACE: Duration = Duration::from_millis(500);

pub async fn kill_child_process_group(child: Option<Child>) {
    if let Some(mut child) = child {
        #[cfg(unix)]
        if let Some(pid) = child.id() {
            signal_process_group(pid, "TERM").await;
            let _ = timeout(TERMINATION_GRACE, child.wait()).await;

            if process_group_exists(pid).await {
                signal_process_group(pid, "KILL").await;
            }
        }

        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

#[cfg(unix)]
async fn signal_process_group(pid: u32, signal: &str) {
    let _ = Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(format!("-{pid}"))
        .stderr(std::process::Stdio::null())
        .status()
        .await;
}

#[cfg(unix)]
async fn process_group_exists(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(format!("-{pid}"))
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tokio::time::sleep;

    #[tokio::test]
    #[cfg(unix)]
    async fn kill_child_process_group_terminates_descendants() {
        let temp = tempfile::tempdir().expect("tempdir");
        let pid_file = temp.path().join("sleep.pid");
        let child = Command::new("/bin/sh")
            .arg("-c")
            .arg("sleep 30 & echo $! > \"$1\"; wait")
            .arg("sh")
            .arg(&pid_file)
            .process_group(0)
            .spawn()
            .expect("spawn shell");

        let sleep_pid = wait_for_pid_file(&pid_file).await;
        kill_child_process_group(Some(child)).await;

        for _ in 0..20 {
            if !process_exists(sleep_pid).await {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
        panic!("descendant process survived process-group cleanup: {sleep_pid}");
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn kill_child_process_group_kills_descendant_when_leader_exits_first() {
        let temp = tempfile::tempdir().expect("tempdir");
        let pid_file = temp.path().join("sleep.pid");
        let child = Command::new("/bin/sh")
            .arg("-c")
            .arg(
                "trap 'exit 0' TERM; /bin/sh -c 'trap \"\" TERM; sleep 30' & echo $! > \"$1\"; while :; do sleep 1; done",
            )
            .arg("sh")
            .arg(&pid_file)
            .process_group(0)
            .spawn()
            .expect("spawn shell");

        let sleep_pid = wait_for_pid_file(&pid_file).await;
        kill_child_process_group(Some(child)).await;

        for _ in 0..20 {
            if !process_exists(sleep_pid).await {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
        panic!("descendant survived after leader exited: {sleep_pid}");
    }

    #[cfg(unix)]
    async fn wait_for_pid_file(path: &std::path::Path) -> u32 {
        for _ in 0..20 {
            if let Ok(pid) = fs::read_to_string(path)
                && let Ok(pid) = pid.trim().parse()
            {
                return pid;
            }
            sleep(Duration::from_millis(50)).await;
        }
        panic!("pid file was not written: {}", path.display());
    }

    #[cfg(unix)]
    async fn process_exists(pid: u32) -> bool {
        tokio::process::Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .is_ok_and(|status| status.success())
    }
}
