use crate::docs_html::{DOCS_CSS, render};
use crate::docs_model::{InstructionsDocument, build_instructions};
use crate::docs_openapi::{openapi_json, openapi_yaml};
use crate::error::Result;
use crate::lease_manager::{LeaseManager, LeaseManagerConfig};
use crate::lease_runtime::real_lease_spawner;
use crate::plan::Plan;
use crate::service::DocsServiceConfig;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json, Response};
use axum::routing::get;
use std::fs;
use std::sync::Arc;

pub fn router(state: Arc<DocsState>) -> Router {
    Router::new()
        .route("/", get(instructions_html))
        .route("/healthz", get(healthz))
        .route("/docs.css", get(docs_css))
        .route("/instructions", get(instructions_html))
        .route("/instructions.json", get(instructions_json))
        .route("/instructions.html", get(instructions_html))
        .route("/projects.json", get(projects_json))
        .route("/project/{*path}", get(project_json))
        .route("/openapi.json", get(openapi_json_handler))
        .route("/openapi.yaml", get(openapi_yaml_handler))
        .nest("/leases", crate::docs_routes_lease::router())
        .with_state(state)
}

pub struct DocsState {
    pub config: DocsServiceConfig,
    plan: Plan,
    instructions: InstructionsDocument,
    openapi_json: serde_json::Value,
    openapi_yaml: String,
    pub(crate) leases: LeaseManager,
}

impl DocsState {
    #[must_use]
    pub fn new(config: DocsServiceConfig, plan: Plan) -> Self {
        Self::new_with_spawner(config, plan, real_lease_spawner())
    }

    #[must_use]
    pub fn new_with_spawner(
        config: DocsServiceConfig,
        plan: Plan,
        spawner: Arc<dyn crate::lease_runtime::LeaseSpawner>,
    ) -> Self {
        let instructions = build_instructions(&config, &plan);
        let openapi_json = openapi_json(&config, &plan, &instructions);
        let openapi_yaml = openapi_yaml(&openapi_json)
            .unwrap_or_else(|error| format!("openapi serialization failed: {error}"));
        let leases = LeaseManager::new(LeaseManagerConfig {
            lease_root: config.lease_root.clone(),
            worker_bin: config.worker_bin.clone(),
            wrangler_bin: config.wrangler_bin.clone(),
            port_start: config.lease_port_start,
            port_end: config.lease_port_end,
            inspector_port_start: config.lease_inspector_port_start,
            inspector_port_end: config.lease_inspector_port_end,
            failure_report_ttl_secs: config.failure_report_ttl_secs,
            failure_report_max_entries: config.failure_report_max_entries,
            spawner,
        });
        Self {
            config,
            plan,
            instructions,
            openapi_json,
            openapi_yaml,
            leases,
        }
    }
}

/// Loads a plan file for the docs service.
///
/// # Errors
///
/// Returns an error if the plan cannot be read or parsed.
pub fn load_plan(path: &std::path::Path) -> Result<Plan> {
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn app_with_spawner(
    config: DocsServiceConfig,
    plan: Plan,
    spawner: Arc<dyn crate::lease_runtime::LeaseSpawner>,
) -> Router {
    router(Arc::new(DocsState::new_with_spawner(config, plan, spawner)))
}

async fn healthz() -> &'static str {
    "ok"
}

async fn docs_css() -> Response {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8")],
        DOCS_CSS,
    )
        .into_response()
}

async fn instructions_json(State(state): State<Arc<DocsState>>) -> Json<InstructionsDocument> {
    Json(state.instructions.clone())
}

async fn instructions_html(State(state): State<Arc<DocsState>>) -> Html<String> {
    Html(render(&state.instructions).into_string())
}

async fn projects_json(State(state): State<Arc<DocsState>>) -> Json<Plan> {
    Json(state.plan.clone())
}

async fn project_json(Path(path): Path<String>, State(state): State<Arc<DocsState>>) -> Response {
    let Some(name) = path.strip_suffix(".json") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match state
        .plan
        .projects
        .iter()
        .find(|project| project.name == name)
    {
        Some(project) => Json(project).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn openapi_json_handler(State(state): State<Arc<DocsState>>) -> Json<serde_json::Value> {
    Json(state.openapi_json.clone())
}

async fn openapi_yaml_handler(State(state): State<Arc<DocsState>>) -> Response {
    (
        [(axum::http::header::CONTENT_TYPE, "application/yaml")],
        state.openapi_yaml.clone(),
    )
        .into_response()
}
