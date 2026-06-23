use axum::body::{Body, to_bytes};
use axum::http::Request;
use serde_json::Value;
use tower::ServiceExt;

pub use super::request_json::request;

/// Waits until the lease reaches the ready state.
///
/// # Panics
///
/// Panics if the lease never becomes ready or the router returns invalid JSON.
#[must_use]
pub async fn wait_for_ready(router: axum::Router, lease_id: &str) -> Value {
    let mut last_response = None;
    for _ in 0..240 {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/leases/{lease_id}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: Value = serde_json::from_slice(&body).expect("json");
        last_response = Some(json.clone());
        if json["status"]["state"] == "ready" {
            return json;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    panic!("lease did not become ready: {last_response:?}");
}
