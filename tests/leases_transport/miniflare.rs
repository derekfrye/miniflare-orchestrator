use super::support::{TransportHarness, delete_lease, restart_lease, wait_for_ready};
use crate::lease_redirect_fixtures::redirect_lease_bundle;
use crate::lease_shared_fixtures::lease_bundle;
use crate::request_json::request;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use tower::ServiceExt;

#[tokio::test]
async fn miniflare_preserves_worker_redirect_responses() {
    let harness = TransportHarness::with_capacity(1);
    let router = harness.router;
    let lease_id = create_miniflare_lease(router.clone(), "miniflare-redirect", "https").await;

    let _ = request(
        router.clone(),
        axum::http::Method::POST,
        &format!("/leases/{lease_id}/bundle"),
        redirect_lease_bundle(),
    )
    .await;
    let restart = restart_lease(router.clone(), &lease_id).await;
    assert_eq!(restart["status"]["state"], "starting");

    let ready = wait_for_ready(router.clone(), &lease_id).await;
    assert_redirect_response(&ready).await;

    let deleted = delete_lease(router, &lease_id).await;
    assert_eq!(deleted["status"]["state"], "stopped");
}

#[tokio::test]
async fn lease_can_run_against_miniflare_backend() {
    let harness = TransportHarness::with_capacity(1);
    let router = harness.router;
    let lease_id = create_miniflare_lease(router.clone(), "miniflare-lease", "http").await;

    let _ = request(
        router.clone(),
        axum::http::Method::POST,
        &format!("/leases/{lease_id}/bundle"),
        lease_bundle(),
    )
    .await;
    let restart = restart_lease(router.clone(), &lease_id).await;
    assert_eq!(restart["status"]["state"], "starting");
    assert_eq!(restart["backend"], "miniflare");

    let ready = wait_for_ready(router.clone(), &lease_id).await;
    assert_miniflare_env(&ready).await;

    let debug = request(
        router.clone(),
        axum::http::Method::GET,
        &format!("/leases/{lease_id}/debug"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(debug["startup"]["backend"], "miniflare");

    let deleted = delete_lease(router, &lease_id).await;
    assert_eq!(deleted["status"]["state"], "stopped");
}

#[tokio::test]
async fn lease_can_run_against_miniflare_r2_binding() {
    let harness = TransportHarness::with_capacity(1);
    let router = harness.router;
    let lease_id = create_miniflare_lease(router.clone(), "miniflare-r2-lease", "http").await;

    let _ = request(
        router.clone(),
        axum::http::Method::POST,
        &format!("/leases/{lease_id}/bundle"),
        r2_lease_bundle(),
    )
    .await;
    let restart = restart_lease(router.clone(), &lease_id).await;
    assert_eq!(restart["status"]["state"], "starting");

    let ready = wait_for_ready(router.clone(), &lease_id).await;
    assert_r2_round_trip(&ready).await;

    let debug = request(
        router.clone(),
        axum::http::Method::GET,
        &format!("/leases/{lease_id}/debug"),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(
        debug["startup"]["effective_bindings"]["backend"],
        "miniflare"
    );
    assert_eq!(debug["startup"]["effective_bindings"]["env"], "dev");
    assert_eq!(
        debug["startup"]["effective_bindings"]["r2_buckets"],
        serde_json::json!(["SCORES_R2"])
    );
    assert_eq!(
        debug["startup"]["effective_bindings"]["kv_namespaces"],
        serde_json::json!(["SCORES_KV"])
    );
    assert_eq!(
        debug["startup"]["effective_bindings"]["vars"],
        serde_json::json!(["LEASE_TEST_VALUE"])
    );

    let logs = text_request(
        router.clone(),
        &format!("/leases/{lease_id}/logs/tail?lines=20"),
    )
    .await;
    assert!(logs.contains("worker-runtime-host-miniflare: effective config"));
    assert!(logs.contains("\"r2Buckets\":[\"SCORES_R2\"]"));

    let deleted = delete_lease(router, &lease_id).await;
    assert_eq!(deleted["status"]["state"], "stopped");
}

async fn create_miniflare_lease(router: axum::Router, name: &str, protocol: &str) -> String {
    let created = request(
        router,
        axum::http::Method::POST,
        "/leases",
        serde_json::json!({
            "name": name,
            "health_path": "/health",
            "env": "dev",
            "protocol": protocol,
            "log_level": "warn",
            "backend": "miniflare",
            "env_vars": { "LEASE_TEST_VALUE": "from-create" }
        }),
    )
    .await;
    assert_eq!(created["backend"], "miniflare");
    created["id"].as_str().expect("lease id").to_string()
}

async fn assert_r2_round_trip(lease: &serde_json::Value) {
    let base_url = lease["base_url"].as_str().expect("base url");
    let client = reqwest::Client::new();
    let put = client
        .post(format!("{base_url}/r2-put?key=round-trip"))
        .body("score=72")
        .send()
        .await
        .expect("r2 put");
    assert_eq!(put.status(), reqwest::StatusCode::OK);

    let body = client
        .get(format!("{base_url}/r2-get?key=round-trip"))
        .send()
        .await
        .expect("r2 get")
        .text()
        .await
        .expect("r2 body");
    assert_eq!(body, "score=72");
}

async fn text_request(router: axum::Router, uri: &str) -> String {
    let response = router
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let body = String::from_utf8(body.to_vec()).expect("utf8 body");
    assert!(status.is_success(), "status={status} body={body:?}");
    body
}

fn r2_lease_bundle() -> serde_json::Value {
    serde_json::json!({
        "runtime_files": [
            {
                "path": "wrangler.toml",
                "content_b64": STANDARD.encode(
                    r#"name = "r2-lease"
main = "worker_entry.mjs"
compatibility_date = "2026-03-23"

[env.dev]
kv_namespaces = [
  { binding = "SCORES_KV", id = "scores-kv" },
]

[[env.dev.r2_buckets]]
binding = "SCORES_R2"
bucket_name = "scores-r2"
"#
                )
            },
            {
                "path": "worker_entry.mjs",
                "content_b64": STANDARD.encode(
                    r#"export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname === "/health") return new Response("ok");
    if (url.pathname === "/r2-put") {
      const key = url.searchParams.get("key") || "default";
      await env.SCORES_R2.put(key, await request.text());
      await env.SCORES_KV.put("last-key", key);
      return new Response("stored");
    }
    if (url.pathname === "/r2-get") {
      const key = url.searchParams.get("key") || await env.SCORES_KV.get("last-key");
      const object = await env.SCORES_R2.get(key);
      return new Response(object ? await object.text() : "", { status: object ? 200 : 404 });
    }
    return new Response("not found", { status: 404 });
  }
};
"#,
                )
            }
        ]
    })
}

async fn assert_redirect_response(lease: &serde_json::Value) {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("https client");
    let response = client
        .get(format!(
            "{}/protected",
            lease["base_url"].as_str().expect("base url")
        ))
        .send()
        .await
        .expect("protected request");

    assert_eq!(response.status(), reqwest::StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok()),
        Some("/recover?next=%2Fprotected")
    );
    assert_eq!(response.text().await.expect("redirect body"), "");
}

async fn assert_miniflare_env(lease: &serde_json::Value) {
    let body = reqwest::get(format!(
        "{}/env?name=LEASE_TEST_VALUE",
        lease["base_url"].as_str().expect("base url")
    ))
    .await
    .expect("worker request")
    .text()
    .await
    .expect("worker body");
    assert_eq!(body, "from-create");
}
