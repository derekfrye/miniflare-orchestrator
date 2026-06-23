use crate::docs_model::{
    ClientContractExample, EndpointDocument, InstructionProject, InstructionsDocument,
};
use crate::lease_model::{
    LeaseBundleDiagnostic, LeaseBundleDiagnosticKind, LeaseBundleMetadata, LeaseBundleRequest,
    LeaseCreateRequest, LeaseDebugResponse, LeaseFailureReport, LeaseFile, LeaseFilesystemEntry,
    LeaseFilesystemEntryKind, LeaseFilesystemSnapshot, LeaseHealthProbeOutcome,
    LeaseHealthProbeReport, LeaseLaunchDetails, LeasePrebuiltBundleNotice, LeaseResponse,
    LeaseRestartRequest, LeaseStartupDiagnosticKind, LeaseStartupDiagnostics, LeaseStatus,
};
use crate::manifest::{Manifest, ProjectInput};
use crate::plan::{Plan, PlannedProject};
use schemars::JsonSchema;
use serde_json::{Map, Value, json};

#[must_use]
pub fn schema_components() -> Map<String, Value> {
    let mut schemas = Map::new();
    insert_schema::<InstructionsDocument>(&mut schemas);
    insert_schema::<InstructionProject>(&mut schemas);
    insert_schema::<EndpointDocument>(&mut schemas);
    insert_schema::<ClientContractExample>(&mut schemas);
    insert_schema::<Manifest>(&mut schemas);
    insert_schema::<ProjectInput>(&mut schemas);
    insert_schema::<Plan>(&mut schemas);
    insert_schema::<PlannedProject>(&mut schemas);
    insert_schema::<LeaseCreateRequest>(&mut schemas);
    insert_schema::<LeaseBundleRequest>(&mut schemas);
    insert_schema::<LeaseBundleMetadata>(&mut schemas);
    insert_schema::<LeasePrebuiltBundleNotice>(&mut schemas);
    insert_schema::<LeaseBundleDiagnostic>(&mut schemas);
    insert_schema::<LeaseBundleDiagnosticKind>(&mut schemas);
    insert_schema::<LeaseRestartRequest>(&mut schemas);
    insert_schema::<LeaseFile>(&mut schemas);
    insert_schema::<LeaseLaunchDetails>(&mut schemas);
    insert_schema::<LeaseHealthProbeReport>(&mut schemas);
    insert_schema::<LeaseHealthProbeOutcome>(&mut schemas);
    insert_schema::<LeaseFilesystemEntryKind>(&mut schemas);
    insert_schema::<LeaseFilesystemEntry>(&mut schemas);
    insert_schema::<LeaseFilesystemSnapshot>(&mut schemas);
    insert_schema::<LeaseResponse>(&mut schemas);
    insert_schema::<LeaseDebugResponse>(&mut schemas);
    insert_schema::<LeaseFailureReport>(&mut schemas);
    insert_schema::<LeaseStartupDiagnostics>(&mut schemas);
    insert_schema::<LeaseStartupDiagnosticKind>(&mut schemas);
    insert_schema::<LeaseStatus>(&mut schemas);
    schemas.insert(
        "ErrorResponse".to_string(),
        json!({
            "type": "object",
            "description": "Standard JSON error payload returned by lease endpoints.",
            "properties": {"error": {"type": "string"}},
            "required": ["error"]
        }),
    );
    schemas
}

fn insert_schema<T: JsonSchema>(schemas: &mut Map<String, Value>) {
    let mut root = serde_json::to_value(schemars::schema_for!(T)).expect("schema serializes");
    if let Some(object) = root.as_object_mut() {
        object.remove("$schema");
        let definitions = object
            .remove("definitions")
            .or_else(|| object.remove("$defs"))
            .and_then(|value| match value {
                Value::Object(values) => Some(values),
                _ => None,
            })
            .unwrap_or_default();

        rewrite_refs(&mut root);
        schemas.insert(schema_name::<T>(), root);

        for (name, mut schema) in definitions {
            rewrite_refs(&mut schema);
            schemas.entry(name).or_insert(schema);
        }
    }
}

fn schema_name<T>() -> String {
    std::any::type_name::<T>()
        .rsplit("::")
        .next()
        .expect("type name has a final segment")
        .to_string()
}

fn rewrite_refs(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get_mut("$ref") {
                if let Some(name) = reference.strip_prefix("#/definitions/") {
                    *reference = format!("#/components/schemas/{name}");
                } else if let Some(name) = reference.strip_prefix("#/$defs/") {
                    *reference = format!("#/components/schemas/{name}");
                }
            }
            for value in object.values_mut() {
                rewrite_refs(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                rewrite_refs(value);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}
