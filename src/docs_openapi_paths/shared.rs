use serde_json::{Value, json};

pub fn json_request_path(
    summary: &str,
    request_schema: &str,
    response_schema: &str,
    error_responses: &[(&str, &str)],
) -> Value {
    let mut responses = serde_json::Map::new();
    responses.insert(
        "200".to_string(),
        json!({
            "description": summary,
            "content": {
                "application/json": {
                    "schema": {"$ref": format!("#/components/schemas/{response_schema}")}
                }
            }
        }),
    );
    for (status, description) in error_responses {
        responses.insert(status.to_string(), error_response(description));
    }
    json!({
        "summary": summary,
        "requestBody": {
            "required": true,
            "content": {
                "application/json": {
                    "schema": {"$ref": format!("#/components/schemas/{request_schema}")}
                }
            }
        },
        "responses": responses
    })
}

pub fn json_response_path(
    summary: &str,
    response_schema: &str,
    error_responses: &[(&str, &str)],
) -> Value {
    let mut responses = serde_json::Map::new();
    responses.insert(
        "200".to_string(),
        json!({
            "description": summary,
            "content": {
                "application/json": {
                    "schema": {"$ref": format!("#/components/schemas/{response_schema}")}
                }
            }
        }),
    );
    for (status, description) in error_responses {
        responses.insert(status.to_string(), error_response(description));
    }
    json!({
        "summary": summary,
        "responses": responses
    })
}

pub fn response_path(summary: &str, content_type: &str, example: &str) -> Value {
    let schema_type = if content_type.starts_with("text/") {
        "string"
    } else {
        "object"
    };
    let mut content = serde_json::Map::new();
    content.insert(
        content_type.to_string(),
        json!({
            "schema": {"type": schema_type},
            "example": if example.is_empty() { Value::Null } else { json!(example) }
        }),
    );
    json!({
        "summary": summary,
        "responses": {
            "200": {
                "description": summary,
                "content": Value::Object(content)
            }
        }
    })
}

pub fn text_response_path(summary: &str, content_type: &str, example: &str) -> Value {
    text_response_path_with_params(summary, content_type, example, &[])
}

pub fn text_response_path_with_params(
    summary: &str,
    content_type: &str,
    example: &str,
    params: &[(&str, &str, bool, &str)],
) -> Value {
    let mut value = response_path(summary, content_type, example);
    if !params.is_empty() {
        let parameters: Vec<Value> = params
            .iter()
            .map(|(name, location, required, description)| {
                json!({
                    "name": name,
                    "in": location,
                    "required": required,
                    "description": description,
                    "schema": {"type": "integer"}
                })
            })
            .collect();
        value["parameters"] = Value::Array(parameters);
    }
    value
}

fn error_response(description: &str) -> Value {
    json!({
        "description": description,
        "content": {
            "application/json": {
                "schema": {"$ref": "#/components/schemas/ErrorResponse"}
            }
        }
    })
}
