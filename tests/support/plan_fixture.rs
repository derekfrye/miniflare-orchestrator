use worker_runtime_host_gen::plan::{Plan, PlannedProject};

#[must_use]
pub fn test_plan() -> Plan {
    Plan {
        manifest: "/work/host/config/projects.json".into(),
        output_dir: "/etc/s6-overlay/s6-rc.d".into(),
        debug_plan_file: "/work/host/config/projects.plan.json".into(),
        service_root: "/run/service".into(),
        dry_run: false,
        projects: vec![PlannedProject {
            name: "food-tracker".to_string(),
            runtime_dir: "/work/host/projects/food-tracker/runtime".into(),
            state_dir: "/work/host/projects/food-tracker/state".into(),
            log_dir: "/work/host/projects/food-tracker/logs".into(),
            static_dir: "/work/host/projects/food-tracker/static".into(),
            config_file: "/work/host/projects/food-tracker/runtime/wrangler.toml".into(),
            reload_token: "/work/host/projects/food-tracker/runtime/.reload-token".into(),
            health_path: "/health".to_string(),
            port: 8788,
            inspector_port: 9100,
            env: "dev".to_string(),
            protocol: "http".to_string(),
            worker_service: "/etc/s6-overlay/s6-rc.d/food-tracker".into(),
            watcher_service: "/etc/s6-overlay/s6-rc.d/food-tracker-watch".into(),
            worker_run: "/etc/s6-overlay/s6-rc.d/food-tracker/run".into(),
            watcher_run: "/etc/s6-overlay/s6-rc.d/food-tracker-watch/run".into(),
        }],
    }
}
