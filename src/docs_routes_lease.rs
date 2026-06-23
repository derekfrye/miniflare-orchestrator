use crate::docs_routes::DocsState;
use crate::lease_manager::LeaseError;
use crate::lease_model::{LeaseBundleRequest, LeaseCreateRequest, LeaseRestartRequest};
use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use std::collections::HashMap;
use std::sync::Arc;

const LEASE_BUNDLE_BODY_LIMIT_BYTES: usize = 50 * 1024 * 1024;

pub fn router() -> axum::Router<Arc<DocsState>> {
    axum::Router::new()
        .route("/", post(create_lease))
        .route("/{id}", get(get_lease).delete(delete_lease))
        .route("/{id}/debug", get(debug_lease))
        .route("/{id}/failure-report", get(failure_report))
        .route("/{id}/probe", get(probe_lease))
        .route("/{id}/filesystem-snapshot", get(filesystem_snapshot))
        .route(
            "/{id}/bundle",
            post(bundle_lease).layer(DefaultBodyLimit::max(LEASE_BUNDLE_BODY_LIMIT_BYTES)),
        )
        .route("/{id}/restart", post(restart_lease))
        .route("/{id}/logs", get(logs))
        .route("/{id}/logs/tail", get(tail_logs))
}

async fn create_lease(
    State(state): State<Arc<DocsState>>,
    payload: Result<Json<LeaseCreateRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    lease_json_result(state.leases.create(request).await)
}

async fn get_lease(State(state): State<Arc<DocsState>>, Path(id): Path<String>) -> Response {
    lease_json_result(state.leases.get(&id).await)
}

async fn debug_lease(State(state): State<Arc<DocsState>>, Path(id): Path<String>) -> Response {
    lease_json_result(state.leases.debug(&id).await)
}

async fn failure_report(State(state): State<Arc<DocsState>>, Path(id): Path<String>) -> Response {
    lease_json_result(state.leases.failure_report(&id).await)
}

async fn probe_lease(State(state): State<Arc<DocsState>>, Path(id): Path<String>) -> Response {
    lease_json_result(state.leases.probe(&id).await)
}

async fn filesystem_snapshot(
    State(state): State<Arc<DocsState>>,
    Path(id): Path<String>,
) -> Response {
    lease_json_result(state.leases.filesystem_snapshot(&id).await)
}

async fn delete_lease(State(state): State<Arc<DocsState>>, Path(id): Path<String>) -> Response {
    lease_json_result(state.leases.delete(&id).await)
}

async fn bundle_lease(
    State(state): State<Arc<DocsState>>,
    Path(id): Path<String>,
    payload: Result<Json<LeaseBundleRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    lease_json_result(state.leases.bundle(&id, request).await)
}

async fn restart_lease(
    State(state): State<Arc<DocsState>>,
    Path(id): Path<String>,
    payload: Result<Json<LeaseRestartRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, &error.to_string()),
    };
    lease_json_result(state.leases.restart(&id, request).await)
}

async fn logs(State(state): State<Arc<DocsState>>, Path(id): Path<String>) -> Response {
    match state.leases.logs(&id).await {
        Ok(response) => text_response(response),
        Err(error) => lease_error_response(error),
    }
}

async fn tail_logs(
    State(state): State<Arc<DocsState>>,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let lines = query
        .get("lines")
        .and_then(|value| value.parse().ok())
        .unwrap_or(200);
    match state.leases.tail_logs(&id, lines).await {
        Ok(response) => text_response(response),
        Err(error) => lease_error_response(error),
    }
}

fn lease_json_result<T: serde::Serialize>(result: Result<T, LeaseError>) -> Response {
    match result {
        Ok(response) => Json(response).into_response(),
        Err(error) => lease_error_response(error),
    }
}

fn text_response(response: String) -> Response {
    ([(CONTENT_TYPE, "text/plain; charset=utf-8")], response).into_response()
}

fn json_error(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

fn lease_error_response(error: LeaseError) -> Response {
    let (status, message) = match error {
        LeaseError::Usage(message) => (StatusCode::BAD_REQUEST, message),
        LeaseError::Json(message) => (StatusCode::BAD_REQUEST, message.to_string()),
        LeaseError::Base64(message) => (StatusCode::BAD_REQUEST, message.to_string()),
        LeaseError::NotFound(message) => (StatusCode::NOT_FOUND, message),
        LeaseError::Conflict(message) => (StatusCode::CONFLICT, message),
        LeaseError::Unavailable(message) => (StatusCode::SERVICE_UNAVAILABLE, message),
        LeaseError::Process(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        LeaseError::Io(message) => (StatusCode::INTERNAL_SERVER_ERROR, message.to_string()),
        LeaseError::Utf8(message) => (StatusCode::INTERNAL_SERVER_ERROR, message.to_string()),
    };

    json_error(status, &message)
}
