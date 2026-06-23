#[path = "runtime_host/worker.rs"]
mod worker;

#[path = "runtime_host_support/mod.rs"]
mod support;

use support::{bin, make_temp_dir, write_file};

use std::fs;
use std::process::Command;

#[test]
fn init_renders_services_and_env_files() {
    let temp = make_temp_dir();
    let host_root = temp.path().join("work/host");
    let services_dir = temp.path().join("etc/s6-overlay/s6-rc.d");
    let plan_file = temp.path().join("work/host/config/projects.plan.json");
    let service_root = temp.path().join("run/service");
    let manifest = host_root.join("config/projects.json");
    let runtime_dir = host_root.join("projects/oms-menus/runtime");
    let state_dir = host_root.join("projects/oms-menus/state");
    let log_dir = host_root.join("projects/oms-menus/logs");
    let static_dir = host_root.join("projects/oms-menus/static");

    write_file(
        &manifest,
        &format!(
            r#"{{
  "projects": [
    {{
      "name": "oms-menus",
      "runtime_dir": "{runtime}",
      "state_dir": "{state}",
      "log_dir": "{log}",
      "static_dir": "{static}",
      "config_file": "{runtime}/wrangler.toml",
      "reload_token": "{runtime}/.reload-token",
      "health_path": "/health",
      "port": 8787,
      "env": "dev",
      "protocol": "http"
    }}
  ]
}}"#,
            runtime = runtime_dir.display(),
            state = state_dir.display(),
            log = log_dir.display(),
            static = static_dir.display(),
        ),
    );
    write_file(&runtime_dir.join("wrangler.toml"), "name = \"oms-menus\"\n");

    let status = Command::new(bin("worker-runtime-host-init"))
        .env("WORKER_RUNTIME_HOST_ROOT", &host_root)
        .env("WORKER_RUNTIME_HOST_MODE", "manifest_and_leases")
        .env("WORKER_RUNTIME_HOST_MANIFEST", &manifest)
        .env("WORKER_RUNTIME_HOST_SERVICES_DIR", &services_dir)
        .env("WORKER_RUNTIME_HOST_PLAN_FILE", &plan_file)
        .env("WORKER_RUNTIME_HOST_SERVICE_ROOT", &service_root)
        .status()
        .expect("run init");

    assert!(status.success());
    assert!(services_dir.join("worker-runtime-host-docs").exists());
    assert!(services_dir.join("oms-menus").exists());
    assert!(services_dir.join("oms-menus-watch").exists());
    assert!(static_dir.is_dir());
    assert_eq!(
        fs::read_to_string(services_dir.join("oms-menus/type"))
            .expect("worker type")
            .trim(),
        "longrun"
    );
    assert!(
        fs::read_to_string(services_dir.join("oms-menus/run"))
            .expect("worker run")
            .contains("/usr/local/bin/worker-runtime-host-worker")
    );
    assert!(
        fs::read_to_string(services_dir.join("oms-menus-watch/run"))
            .expect("watcher run")
            .contains("/usr/local/bin/worker-runtime-host-watch")
    );
    assert!(
        services_dir
            .join("oms-menus-watch/dependencies.d/oms-menus")
            .exists()
    );
    assert!(
        services_dir
            .join("user/contents.d/worker-runtime-host-docs")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(services_dir.join("oms-menus/env/WORKER_RUNTIME_HOST_PORT"))
            .expect("port env")
            .trim(),
        "8787"
    );
    assert_eq!(
        fs::read_to_string(
            services_dir.join("oms-menus-watch/env/WORKER_RUNTIME_HOST_RELOAD_TOKEN")
        )
        .expect("reload token env")
        .trim(),
        runtime_dir.join(".reload-token").display().to_string()
    );
    assert!(plan_file.is_file());
}

#[test]
fn init_defaults_to_leases_only_without_manifest() {
    let temp = make_temp_dir();
    let host_root = temp.path().join("work/host");
    let services_dir = temp.path().join("etc/s6-overlay/s6-rc.d");
    let plan_file = temp.path().join("work/host/config/projects.plan.json");
    let service_root = temp.path().join("run/service");

    let status = Command::new(bin("worker-runtime-host-init"))
        .env("WORKER_RUNTIME_HOST_ROOT", &host_root)
        .env("WORKER_RUNTIME_HOST_SERVICES_DIR", &services_dir)
        .env("WORKER_RUNTIME_HOST_PLAN_FILE", &plan_file)
        .env("WORKER_RUNTIME_HOST_SERVICE_ROOT", &service_root)
        .status()
        .expect("run init");

    assert!(status.success());
    assert!(services_dir.join("worker-runtime-host-docs").exists());
    assert!(!services_dir.join("oms-menus").exists());
    assert!(plan_file.is_file());
}
