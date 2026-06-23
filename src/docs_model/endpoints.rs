use super::{EndpointDocument, InstructionProject};

pub fn build_endpoints() -> Vec<EndpointDocument> {
    let mut endpoints = Vec::new();
    endpoints.extend(documentation_endpoints());
    endpoints.extend(project_endpoints());
    endpoints.extend(lease_endpoints());
    endpoints.extend(openapi_endpoints());
    endpoints
}

pub fn build_project(project: &crate::plan::PlannedProject) -> InstructionProject {
    InstructionProject {
        name: project.name.clone(),
        runtime_dir: project.runtime_dir.display().to_string(),
        static_dir: project.static_dir.display().to_string(),
        state_dir: project.state_dir.display().to_string(),
        log_dir: project.log_dir.display().to_string(),
        config_file: project.config_file.display().to_string(),
        reload_token: project.reload_token.display().to_string(),
        port: project.port,
        inspector_port: project.inspector_port,
        health_url: format!("http://127.0.0.1:{}{}", project.port, project.health_path),
        env: project.env.clone(),
        protocol: project.protocol.clone(),
    }
}

fn documentation_endpoints() -> Vec<EndpointDocument> {
    vec![
        endpoint("GET", "/healthz", "container health probe", &["text/plain"]),
        endpoint("GET", "/docs.css", "docs stylesheet", &["text/css"]),
        endpoint(
            "GET",
            "/instructions",
            "html instructions page",
            &["text/html"],
        ),
        endpoint(
            "GET",
            "/instructions.json",
            "machine-readable instructions",
            &["application/json"],
        ),
        endpoint(
            "GET",
            "/instructions.html",
            "human-readable instructions",
            &["text/html"],
        ),
        endpoint(
            "GET",
            "/projects.json",
            "resolved project plan",
            &["application/json"],
        ),
    ]
}

fn project_endpoints() -> Vec<EndpointDocument> {
    vec![endpoint(
        "GET",
        "/project/{name}.json",
        "one resolved project",
        &["application/json"],
    )]
}

fn lease_endpoints() -> Vec<EndpointDocument> {
    vec![
        endpoint(
            "POST",
            "/leases",
            "allocate a test lease",
            &["application/json"],
        ),
        endpoint(
            "GET",
            "/leases/{id}",
            "inspect one lease",
            &["application/json"],
        ),
        endpoint(
            "GET",
            "/leases/{id}/debug",
            "inspect one lease debug snapshot",
            &["application/json"],
        ),
        endpoint(
            "GET",
            "/leases/{id}/failure-report",
            "inspect one retained lease failure report (kept for up to 1 day by default, with a maximum of 100 deleted failure reports)",
            &["application/json"],
        ),
        endpoint(
            "GET",
            "/leases/{id}/probe",
            "probe one lease health endpoint",
            &["application/json"],
        ),
        endpoint(
            "GET",
            "/leases/{id}/filesystem-snapshot",
            "inspect one lease filesystem snapshot",
            &["application/json"],
        ),
        endpoint(
            "DELETE",
            "/leases/{id}",
            "release one lease",
            &["application/json"],
        ),
        endpoint(
            "POST",
            "/leases/{id}/bundle",
            "upload lease bundle files",
            &["application/json"],
        ),
        endpoint(
            "POST",
            "/leases/{id}/restart",
            "start or restart a lease",
            &["application/json"],
        ),
        endpoint(
            "GET",
            "/leases/{id}/logs",
            "fetch one lease log bundle",
            &["text/plain"],
        ),
        endpoint(
            "GET",
            "/leases/{id}/logs/tail",
            "tail one lease log bundle",
            &["text/plain"],
        ),
    ]
}

fn openapi_endpoints() -> Vec<EndpointDocument> {
    vec![
        endpoint(
            "GET",
            "/openapi.json",
            "OpenAPI document in JSON",
            &["application/json"],
        ),
        endpoint(
            "GET",
            "/openapi.yaml",
            "OpenAPI document in YAML",
            &["application/yaml", "text/yaml"],
        ),
        endpoint(
            "GET",
            "/",
            "alias for the instructions page",
            &["text/html"],
        ),
    ]
}

fn endpoint(
    method: &str,
    path: &str,
    description: &str,
    content_types: &[&str],
) -> EndpointDocument {
    EndpointDocument {
        method: method.to_string(),
        path: path.to_string(),
        description: description.to_string(),
        content_types: content_types
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    }
}
