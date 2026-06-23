use super::support::{
    TestBackend, TransportHarness, delete_lease, provision_lease, restart_lease, wait_for_ready,
};

#[tokio::test]
async fn lease_env_vars_are_isolated_between_two_leases() {
    for backend in TestBackend::all() {
        lease_env_vars_are_isolated_between_two_leases_for_backend(backend).await;
    }
}

async fn lease_env_vars_are_isolated_between_two_leases_for_backend(backend: TestBackend) {
    let harness = TransportHarness::with_capacity(2);
    let router = harness.router;

    let (alpha_lease_id, alpha_created) = provision_lease(
        router.clone(),
        "alpha",
        "http",
        backend,
        serde_json::json!({ "ALPHA_TOKEN": "alpha-secret" }),
    )
    .await;
    let (beta_lease_id, beta_created) = provision_lease(
        router.clone(),
        "beta",
        "http",
        backend,
        serde_json::json!({ "BETA_TOKEN": "beta-secret" }),
    )
    .await;

    let alpha_restart = restart_lease(router.clone(), &alpha_lease_id).await;
    let beta_restart = restart_lease(router.clone(), &beta_lease_id).await;

    assert_created_state(&alpha_created, &beta_created, backend);
    assert_eq!(alpha_restart["status"]["state"], "starting");
    assert_eq!(beta_restart["status"]["state"], "starting");

    let alpha_ready = wait_for_ready(router.clone(), &alpha_lease_id).await;
    let beta_ready = wait_for_ready(router.clone(), &beta_lease_id).await;
    assert_eq!(alpha_ready["backend"], backend.name());
    assert_eq!(beta_ready["backend"], backend.name());

    let alpha_token = worker_env(&alpha_ready, "ALPHA_TOKEN", "alpha token").await;
    let beta_token = worker_env(&beta_ready, "BETA_TOKEN", "beta token").await;
    let alpha_cannot_see_beta = worker_env(&alpha_ready, "BETA_TOKEN", "alpha isolation").await;
    let beta_cannot_see_alpha = worker_env(&beta_ready, "ALPHA_TOKEN", "beta isolation").await;
    assert_eq!(alpha_token, "alpha-secret");
    assert_eq!(beta_token, "beta-secret");
    assert_eq!(alpha_cannot_see_beta, "");
    assert_eq!(beta_cannot_see_alpha, "");

    let alpha_deleted = delete_lease(router.clone(), &alpha_lease_id).await;
    let beta_deleted = delete_lease(router, &beta_lease_id).await;

    assert_eq!(alpha_deleted["status"]["state"], "stopped");
    assert_eq!(beta_deleted["status"]["state"], "stopped");
}

fn assert_created_state(
    alpha_created: &serde_json::Value,
    beta_created: &serde_json::Value,
    backend: TestBackend,
) {
    assert_eq!(alpha_created["status"]["state"], "created");
    assert_eq!(beta_created["status"]["state"], "created");
    assert_eq!(alpha_created["env_vars"]["ALPHA_TOKEN"], "alpha-secret");
    assert_eq!(beta_created["env_vars"]["BETA_TOKEN"], "beta-secret");
    assert_eq!(alpha_created["backend"], backend.name());
    assert_eq!(beta_created["backend"], backend.name());
}

async fn worker_env(lease: &serde_json::Value, name: &str, context: &str) -> String {
    reqwest::get(format!(
        "{}/env?name={name}",
        lease["base_url"].as_str().expect("base url")
    ))
    .await
    .unwrap_or_else(|error| panic!("{context} request: {error}"))
    .text()
    .await
    .unwrap_or_else(|error| panic!("{context} body: {error}"))
}
