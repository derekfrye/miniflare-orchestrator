use axum::body::{Body, to_bytes};
use axum::http::Request;
use serde_json::Value;
use tower::ServiceExt;

/// Sends an HTTP request through the test router.
///
/// # Panics
///
/// Panics if the router rejects the request or the body is not valid JSON.
#[must_use]
pub async fn request(
    router: axum::Router,
    method: axum::http::Method,
    uri: &str,
    body: Value,
) -> Value {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request");
    let response = router.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let body_text = String::from_utf8_lossy(&body);
    serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("status={status} body={body_text:?} json={error}"))
}
