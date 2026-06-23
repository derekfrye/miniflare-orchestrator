use serde_json::{Map, Value, json};

use super::shared::{
    json_request_path, json_response_path, response_path, text_response_path,
    text_response_path_with_params,
};

pub fn build_paths() -> Value {
    let mut paths = Map::new();
    paths.extend(doc_paths());
    paths.extend(project_paths());
    paths.extend(lease_paths());
    paths.extend(openapi_paths());
    Value::Object(paths)
}

fn doc_paths() -> Map<String, Value> {
    Map::from_iter([
        (
            "/healthz".to_string(),
            json!({ "get": response_path("container health probe", "text/plain", "ok") }),
        ),
        (
            "/docs.css".to_string(),
            json!({ "get": response_path("docs stylesheet", "text/css", "") }),
        ),
        (
            "/instructions".to_string(),
            json!({ "get": response_path("instructions page", "text/html", "") }),
        ),
        (
            "/instructions.json".to_string(),
            json!({ "get": response_path("machine-readable instructions", "application/json", "") }),
        ),
        (
            "/instructions.html".to_string(),
            json!({ "get": response_path("human-readable instructions", "text/html", "") }),
        ),
        (
            "/projects.json".to_string(),
            json!({ "get": response_path("resolved project plan", "application/json", "") }),
        ),
    ])
}

fn project_paths() -> Map<String, Value> {
    Map::from_iter([(
        "/project/{name}.json".to_string(),
        json!({
            "get": {
                "summary": "one resolved project",
                "responses": {
                    "200": {
                        "description": "project found",
                        "content": {"application/json": {"schema": {"$ref": "#/components/schemas/PlannedProject"}}}
                    },
                    "404": {"description": "project not found"}
                },
                "parameters": [{
                    "name": "name",
                    "in": "path",
                    "required": true,
                    "schema": {"type": "string"}
                }]
            }
        }),
    )])
}

fn lease_paths() -> Map<String, Value> {
    Map::from_iter([
        (
            "/leases".to_string(),
            json!({ "post": json_request_path("allocate a test lease with isolated Worker and inspector ports", "LeaseCreateRequest", "LeaseResponse", &[("400", "bad request"), ("500", "unexpected internal error"), ("503", "lease worker or inspector ports unavailable")]) }),
        ),
        (
            "/leases/{id}".to_string(),
            json!({
                "get": json_response_path("inspect one lease", "LeaseResponse", &[("404", "lease not found")]),
                "delete": json_response_path("release one lease", "LeaseResponse", &[("404", "lease not found"), ("500", "unexpected internal error")])
            }),
        ),
        (
            "/leases/{id}/debug".to_string(),
            json!({
                "get": json_response_path("inspect one lease debug snapshot", "LeaseDebugResponse", &[("404", "lease not found")])
            }),
        ),
        (
            "/leases/{id}/failure-report".to_string(),
            json!({
                "get": json_response_path("inspect one retained lease failure report (kept for up to 1 day by default, with a maximum of 100 deleted failure reports)", "LeaseFailureReport", &[("404", "lease not found")])
            }),
        ),
        (
            "/leases/{id}/probe".to_string(),
            json!({
                "get": json_response_path("probe one lease health endpoint", "LeaseHealthProbeReport", &[("404", "lease not found"), ("500", "unexpected internal error")])
            }),
        ),
        (
            "/leases/{id}/filesystem-snapshot".to_string(),
            json!({
                "get": json_response_path("inspect one lease filesystem snapshot", "LeaseFilesystemSnapshot", &[("404", "lease not found"), ("500", "unexpected internal error")])
            }),
        ),
        (
            "/leases/{id}/bundle".to_string(),
            json!({ "post": json_request_path("upload prebuilt lease bundle files and optional provenance metadata", "LeaseBundleRequest", "LeaseResponse", &[("400", "bad request"), ("404", "lease not found"), ("500", "unexpected internal error")]) }),
        ),
        (
            "/leases/{id}/restart".to_string(),
            json!({ "post": json_request_path("start or restart a lease using the already-uploaded prebuilt bundle", "LeaseRestartRequest", "LeaseResponse", &[("400", "bad request"), ("404", "lease not found"), ("500", "unexpected internal error"), ("503", "lease startup unavailable")]) }),
        ),
        (
            "/leases/{id}/logs".to_string(),
            json!({ "get": text_response_path("fetch one lease log bundle", "text/plain; charset=utf-8", "== stdout.log ==\\n...") }),
        ),
        (
            "/leases/{id}/logs/tail".to_string(),
            json!({ "get": text_response_path_with_params("tail one lease log bundle", "text/plain; charset=utf-8", "[1713878400.000Z] [stdout.log] ...", &[("lines", "query", true, "optional number of trailing lines to return")]) }),
        ),
    ])
}

fn openapi_paths() -> Map<String, Value> {
    Map::from_iter([
        (
            "/openapi.json".to_string(),
            json!({ "get": response_path("openapi document in json", "application/json", "") }),
        ),
        (
            "/openapi.yaml".to_string(),
            json!({ "get": response_path("openapi document in yaml", "application/yaml", "") }),
        ),
    ])
}
