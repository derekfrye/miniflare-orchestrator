use super::support::{bin, make_executable, make_temp_dir, prepend_path, wait_for, write_file};
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
fn worker_execs_wrangler_with_expected_args() {
    let temp = make_temp_dir();
    let fake_bin = temp.path().join("bin");
    let call_log = temp.path().join("calls/worker.txt");
    let runtime_dir = temp.path().join("runtime");
    let state_dir = temp.path().join("state");
    let log_dir = temp.path().join("logs");
    let config_file = runtime_dir.join("wrangler.toml");

    fs::create_dir_all(&fake_bin).expect("fake bin dir");
    write_file(&config_file, "name = \"oms-menus\"\n");
    make_executable(
        &fake_bin.join("wrangler"),
        &format!(
            r#"#!/bin/sh
set -eu
mkdir -p "$(dirname "{call_log}")"
printf '%s\n' "$0" "$@" > "{call_log}"
pwd > "{call_log}.cwd"
exit 0
"#,
            call_log = call_log.display()
        ),
    );

    let status = Command::new(bin("worker-runtime-host-worker"))
        .env("WORKER_RUNTIME_HOST_RUNTIME_DIR", &runtime_dir)
        .env("WORKER_RUNTIME_HOST_STATE_DIR", &state_dir)
        .env("WORKER_RUNTIME_HOST_LOG_DIR", &log_dir)
        .env("WORKER_RUNTIME_HOST_CONFIG_FILE", &config_file)
        .env("WORKER_RUNTIME_HOST_PORT", "8787")
        .env("WORKER_RUNTIME_HOST_INSPECTOR_PORT", "9100")
        .env("WORKER_RUNTIME_HOST_ENV", "dev")
        .env("WORKER_RUNTIME_HOST_PROTOCOL", "http")
        .env("WORKER_RUNTIME_HOST_LOG_LEVEL", "warn")
        .env("WORKER_RUNTIME_HOST_BACKEND", "wrangler_dev")
        .env(
            "WORKER_RUNTIME_HOST_WRANGLER_BIN",
            fake_bin.join("wrangler"),
        )
        .status()
        .expect("run worker");

    assert!(status.success());
    let args = fs::read_to_string(&call_log).expect("call log");
    assert!(args.contains("wrangler"));
    assert!(args.contains("dev"));
    assert!(args.contains("--local"));
    assert!(args.contains("--no-bundle"));
    assert!(args.contains("--config"));
    assert!(args.contains(config_file.to_string_lossy().as_ref()));
    assert!(args.contains("--port"));
    assert!(args.contains("8787"));
    assert!(args.contains("--inspector-ip"));
    assert!(args.contains("127.0.0.1"));
    assert!(args.contains("--inspector-port"));
    assert!(args.contains("9100"));
    assert_eq!(
        fs::read_to_string(format!("{}.cwd", call_log.display())).expect("cwd"),
        format!("{}\n", runtime_dir.display())
    );
}

#[test]
fn worker_defaults_to_miniflare_runner() {
    let temp = make_temp_dir();
    let fake_bin = temp.path().join("bin");
    let call_log = temp.path().join("calls/node.txt");
    let runtime_dir = temp.path().join("runtime");
    let state_dir = temp.path().join("state");
    let log_dir = temp.path().join("logs");
    let config_file = runtime_dir.join("wrangler.toml");

    fs::create_dir_all(&fake_bin).expect("fake bin dir");
    write_file(
        &config_file,
        "name = \"oms-menus\"\nmain = \"worker_entry.mjs\"\n",
    );
    make_executable(
        &fake_bin.join("node"),
        &format!(
            r#"#!/bin/sh
set -eu
mkdir -p "$(dirname "{call_log}")"
printf '%s\n' "$0" "$@" > "{call_log}"
printf '%s\n' "$WORKER_RUNTIME_HOST_BACKEND" > "{call_log}.backend"
test -f "$1"
exit 0
"#,
            call_log = call_log.display()
        ),
    );

    let status = Command::new(bin("worker-runtime-host-worker"))
        .env("PATH", prepend_path(&fake_bin))
        .env("WORKER_RUNTIME_HOST_RUNTIME_DIR", &runtime_dir)
        .env("WORKER_RUNTIME_HOST_STATE_DIR", &state_dir)
        .env("WORKER_RUNTIME_HOST_LOG_DIR", &log_dir)
        .env("WORKER_RUNTIME_HOST_CONFIG_FILE", &config_file)
        .env("WORKER_RUNTIME_HOST_PORT", "8787")
        .env("WORKER_RUNTIME_HOST_INSPECTOR_PORT", "9100")
        .env("WORKER_RUNTIME_HOST_ENV", "dev")
        .env("WORKER_RUNTIME_HOST_PROTOCOL", "http")
        .env("WORKER_RUNTIME_HOST_LOG_LEVEL", "warn")
        .env("WORKER_RUNTIME_HOST_NODE_BIN", fake_bin.join("node"))
        .status()
        .expect("run worker");

    assert!(status.success());
    let args = fs::read_to_string(&call_log).expect("call log");
    assert!(args.contains("node"));
    assert!(args.contains(".worker-runtime-host-miniflare-runner.mjs"));
    assert_eq!(
        fs::read_to_string(format!("{}.backend", call_log.display())).expect("backend"),
        "miniflare\n"
    );
}

#[test]
fn worker_passes_miniflare_debug_controls_to_runner() {
    let temp = make_temp_dir();
    let fake_bin = temp.path().join("bin");
    let call_log = temp.path().join("calls/node.txt");
    let runtime_dir = temp.path().join("runtime");
    let state_dir = temp.path().join("state");
    let log_dir = temp.path().join("logs");
    let config_file = runtime_dir.join("wrangler.toml");

    fs::create_dir_all(&fake_bin).expect("fake bin dir");
    write_file(
        &config_file,
        "name = \"oms-menus\"\nmain = \"worker_entry.mjs\"\n",
    );
    make_executable(
        &fake_bin.join("node"),
        &format!(
            r#"#!/bin/sh
set -eu
mkdir -p "$(dirname "{call_log}")"
printf '%s\n' "$WORKER_RUNTIME_HOST_MINIFLARE_MODULE" > "{call_log}.module"
printf '%s\n' "$WORKER_RUNTIME_HOST_MINIFLARE_VERBOSE" > "{call_log}.verbose"
printf '%s\n' "$WORKER_RUNTIME_HOST_MINIFLARE_WORKERD_CONFIG_DEBUG" > "{call_log}.config_debug"
printf '%s\n' "$WORKER_RUNTIME_HOST_MINIFLARE_DISABLE_INSPECTOR" > "{call_log}.disable_inspector"
printf '%s\n' "$MINIFLARE_WORKERD_PATH" > "{call_log}.workerd_path"
exit 0
"#,
            call_log = call_log.display()
        ),
    );

    let status = Command::new(bin("worker-runtime-host-worker"))
        .env("PATH", prepend_path(&fake_bin))
        .env("WORKER_RUNTIME_HOST_RUNTIME_DIR", &runtime_dir)
        .env("WORKER_RUNTIME_HOST_STATE_DIR", &state_dir)
        .env("WORKER_RUNTIME_HOST_LOG_DIR", &log_dir)
        .env("WORKER_RUNTIME_HOST_CONFIG_FILE", &config_file)
        .env("WORKER_RUNTIME_HOST_PORT", "8787")
        .env("WORKER_RUNTIME_HOST_INSPECTOR_PORT", "9100")
        .env("WORKER_RUNTIME_HOST_ENV", "dev")
        .env("WORKER_RUNTIME_HOST_PROTOCOL", "http")
        .env("WORKER_RUNTIME_HOST_LOG_LEVEL", "warn")
        .env("WORKER_RUNTIME_HOST_NODE_BIN", fake_bin.join("node"))
        .env(
            "WORKER_RUNTIME_HOST_MINIFLARE_MODULE",
            "/tmp/miniflare/dist/src/index.js",
        )
        .env("WORKER_RUNTIME_HOST_MINIFLARE_VERBOSE", "1")
        .env("WORKER_RUNTIME_HOST_MINIFLARE_WORKERD_CONFIG_DEBUG", "true")
        .env("WORKER_RUNTIME_HOST_MINIFLARE_DISABLE_INSPECTOR", "1")
        .env("MINIFLARE_WORKERD_PATH", "/tmp/workerd")
        .status()
        .expect("run worker");

    assert!(status.success());
    assert_eq!(
        fs::read_to_string(format!("{}.module", call_log.display())).expect("module"),
        "/tmp/miniflare/dist/src/index.js\n"
    );
    assert_eq!(
        fs::read_to_string(format!("{}.verbose", call_log.display())).expect("verbose"),
        "1\n"
    );
    assert_eq!(
        fs::read_to_string(format!("{}.config_debug", call_log.display())).expect("config debug"),
        "true\n"
    );
    assert_eq!(
        fs::read_to_string(format!("{}.disable_inspector", call_log.display()))
            .expect("disable inspector"),
        "1\n"
    );
    assert_eq!(
        fs::read_to_string(format!("{}.workerd_path", call_log.display())).expect("workerd path"),
        "/tmp/workerd\n"
    );
}

#[test]
fn watcher_restarts_worker_on_token_change() {
    let temp = make_temp_dir();
    let fake_bin = temp.path().join("bin");
    let reload_token = temp.path().join("runtime/.reload-token");
    let worker_service = temp.path().join("services/oms-menus");
    let restart_log = temp.path().join("calls/s6-svc.txt");

    fs::create_dir_all(&fake_bin).expect("fake bin dir");
    fs::create_dir_all(&worker_service).expect("worker service dir");
    make_executable(
        &fake_bin.join("s6-svc"),
        &format!(
            r#"#!/bin/sh
set -eu
mkdir -p "$(dirname "{restart_log}")"
printf '%s\n' "$0" "$@" > "{restart_log}"
exit 0
"#,
            restart_log = restart_log.display()
        ),
    );

    let mut child = Command::new(bin("worker-runtime-host-watch"))
        .env("PATH", prepend_path(&fake_bin))
        .env("WORKER_RUNTIME_HOST_RELOAD_TOKEN", &reload_token)
        .env("WORKER_RUNTIME_HOST_WORKER_SERVICE", &worker_service)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn watcher");

    assert!(wait_for(Duration::from_secs(2), || reload_token.is_file()));
    thread::sleep(Duration::from_millis(200));
    let staged = reload_token.with_extension("tmp");
    write_file(&staged, "trigger-1\n");
    fs::rename(&staged, &reload_token).expect("atomic reload token rewrite");
    assert!(wait_for(Duration::from_secs(5), || restart_log.is_file()));

    let log = fs::read_to_string(&restart_log).expect("restart log");
    assert!(log.contains("s6-svc"));
    assert!(log.contains("-r"));
    assert!(log.contains(worker_service.to_string_lossy().as_ref()));

    child.kill().expect("kill watcher");
    let _ = child.wait();
}
