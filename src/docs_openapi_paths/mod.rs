use serde_json::Value;

#[path = "routes.rs"]
mod routes;
#[path = "shared.rs"]
mod shared;

#[must_use]
pub fn build_paths() -> Value {
    routes::build_paths()
}
