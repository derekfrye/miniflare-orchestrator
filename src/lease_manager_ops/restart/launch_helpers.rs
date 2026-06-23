use crate::lease_model::{LeaseEffectiveBindings, LeaseLaunchDetails};
use crate::lease_runtime::{LeaseLaunchConfig, lease_launch_env};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(super) fn startup_details(
    launch: &LeaseLaunchConfig,
    worker_bin: &Path,
    wrangler_bin: &Path,
) -> LeaseLaunchDetails {
    LeaseLaunchDetails {
        backend: launch.backend,
        worker_bin: worker_bin.display().to_string(),
        wrangler_bin: wrangler_bin.display().to_string(),
        config_file: launch.config_file.display().to_string(),
        runtime_dir: launch.runtime_dir.display().to_string(),
        static_dir: launch.static_dir.display().to_string(),
        state_dir: launch.state_dir.display().to_string(),
        log_dir: launch.log_dir.display().to_string(),
        port: launch.port,
        inspector_port: launch.inspector_port,
        env: launch.env_name.clone(),
        protocol: launch.protocol.clone(),
        log_level: launch.log_level.clone(),
        persist_state: launch.persist_state,
        env_vars: launch.env_vars.clone(),
        injected_env: lease_launch_env(launch, wrangler_bin),
        effective_bindings: effective_bindings(launch),
    }
}

fn effective_bindings(launch: &LeaseLaunchConfig) -> LeaseEffectiveBindings {
    let mut bindings = parse_wrangler_bindings(
        &fs::read_to_string(&launch.config_file).unwrap_or_default(),
        &launch.env_name,
    );
    bindings.backend = launch.backend;
    bindings.env = launch.env_name.clone();
    bindings.vars.extend(launch.env_vars.keys().cloned());
    bindings.vars.sort();
    bindings.vars.dedup();
    bindings
}

#[derive(Default)]
struct ParsedWranglerBindings {
    vars: BTreeSet<String>,
    kv_namespaces: BTreeSet<String>,
    r2_buckets: BTreeSet<String>,
    durable_objects: BTreeSet<String>,
    assets: Option<String>,
}

fn parse_wrangler_bindings(source: &str, env_name: &str) -> LeaseEffectiveBindings {
    let mut parsed = ParsedWranglerBindings::default();
    let mut section = String::new();
    let mut array_table = String::new();
    let mut current_object = std::collections::BTreeMap::new();
    let mut collecting: Option<Collecting> = None;

    for raw_line in source.lines() {
        let line = strip_comment(raw_line).trim().to_string();
        if line.is_empty() {
            continue;
        }

        if let Some(table) = array_header(&line) {
            finish_object(&mut parsed, &array_table, &current_object, env_name);
            section.clear();
            array_table = table.to_string();
            current_object.clear();
            collecting = None;
            continue;
        }

        if let Some(header) = header(&line) {
            finish_object(&mut parsed, &array_table, &current_object, env_name);
            section = header.to_string();
            array_table.clear();
            current_object.clear();
            collecting = None;
            continue;
        }

        if let Some(kind) = collecting {
            if line == "]" {
                collecting = None;
                continue;
            }
            let object = parse_inline_object(line.trim_end_matches(','));
            apply_collected_object(&mut parsed, kind, &object);
            continue;
        }

        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let raw_value = raw_value.trim();

        if raw_value == "[" {
            collecting = collection_for(&section, key, env_name);
            continue;
        }

        if !array_table.is_empty() {
            current_object.insert(key.to_string(), parse_toml_string(raw_value));
            continue;
        }

        apply_scalar(&mut parsed, &section, key, raw_value, env_name);
    }
    finish_object(&mut parsed, &array_table, &current_object, env_name);

    LeaseEffectiveBindings {
        backend: Default::default(),
        env: String::new(),
        vars: parsed.vars.into_iter().collect(),
        kv_namespaces: parsed.kv_namespaces.into_iter().collect(),
        r2_buckets: parsed.r2_buckets.into_iter().collect(),
        durable_objects: parsed.durable_objects.into_iter().collect(),
        assets: parsed.assets,
    }
}

#[derive(Clone, Copy)]
enum Collecting {
    KvNamespace,
    R2Bucket,
    DurableObject,
}

fn collection_for(section: &str, key: &str, env_name: &str) -> Option<Collecting> {
    if key == "kv_namespaces" && (section.is_empty() || section == format!("env.{env_name}")) {
        return Some(Collecting::KvNamespace);
    }
    if key == "r2_buckets" && (section.is_empty() || section == format!("env.{env_name}")) {
        return Some(Collecting::R2Bucket);
    }
    if key == "bindings"
        && (section == "durable_objects" || section == format!("env.{env_name}.durable_objects"))
    {
        return Some(Collecting::DurableObject);
    }
    None
}

fn apply_scalar(
    parsed: &mut ParsedWranglerBindings,
    section: &str,
    key: &str,
    raw_value: &str,
    env_name: &str,
) {
    if section == "vars" || section == format!("env.{env_name}.vars") {
        parsed.vars.insert(key.to_string());
    }
    if (section == "assets" || section == format!("env.{env_name}.assets")) && key == "binding" {
        parsed.assets = Some(parse_toml_string(raw_value));
    }
}

fn apply_collected_object(
    parsed: &mut ParsedWranglerBindings,
    kind: Collecting,
    object: &std::collections::BTreeMap<String, String>,
) {
    match kind {
        Collecting::KvNamespace => {
            if let Some(binding) = object.get("binding") {
                parsed.kv_namespaces.insert(binding.clone());
            }
        }
        Collecting::R2Bucket => {
            if let Some(binding) = object.get("binding") {
                parsed.r2_buckets.insert(binding.clone());
            }
        }
        Collecting::DurableObject => {
            if let Some(name) = object.get("name") {
                parsed.durable_objects.insert(name.clone());
            }
        }
    }
}

fn finish_object(
    parsed: &mut ParsedWranglerBindings,
    table: &str,
    object: &std::collections::BTreeMap<String, String>,
    env_name: &str,
) {
    if table == "r2_buckets" || table == format!("env.{env_name}.r2_buckets") {
        apply_collected_object(parsed, Collecting::R2Bucket, object);
    }
    if table == "durable_objects.bindings"
        || table == format!("env.{env_name}.durable_objects.bindings")
    {
        apply_collected_object(parsed, Collecting::DurableObject, object);
    }
}

fn parse_inline_object(value: &str) -> std::collections::BTreeMap<String, String> {
    let mut object = std::collections::BTreeMap::new();
    let body = value
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim();
    for part in body.split(',') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        object.insert(key.trim().to_string(), parse_toml_string(value.trim()));
    }
    object
}

fn parse_toml_string(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(',')
        .trim_matches('"')
        .replace("\\\"", "\"")
}

fn array_header(line: &str) -> Option<&str> {
    line.strip_prefix("[[")?.strip_suffix("]]").map(str::trim)
}

fn header(line: &str) -> Option<&str> {
    if line.starts_with("[[") {
        return None;
    }
    line.strip_prefix('[')?.strip_suffix(']').map(str::trim)
}

fn strip_comment(line: &str) -> String {
    let mut in_string = false;
    for (index, char) in line.char_indices() {
        if char == '"' && !line[..index].ends_with('\\') {
            in_string = !in_string;
        }
        if char == '#' && !in_string {
            return line[..index].to_string();
        }
    }
    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::parse_wrangler_bindings;

    #[test]
    fn parses_effective_bindings_for_selected_env() {
        let source = r#"
name = "example"

kv_namespaces = [
  { binding = "ROOT_KV", id = "root-kv" },
]

[[r2_buckets]]
binding = "ROOT_R2"
bucket_name = "root-r2"

[vars]
ROOT_VAR = "root"

[env.dev]
kv_namespaces = [
  { binding = "DEV_KV", id = "dev-kv" },
]
r2_buckets = [
  { binding = "DEV_R2", bucket_name = "dev-r2" },
]

[env.dev.vars]
DEV_VAR = "dev"

[env.dev.assets]
binding = "ASSETS"

[[env.dev.r2_buckets]]
binding = "DEV_TABLE_R2"
bucket_name = "dev-table-r2"

[[env.dev.durable_objects.bindings]]
name = "SESSION"
class_name = "Session"
"#;

        let bindings = parse_wrangler_bindings(source, "dev");

        assert_eq!(bindings.kv_namespaces, ["DEV_KV", "ROOT_KV"]);
        assert_eq!(bindings.r2_buckets, ["DEV_R2", "DEV_TABLE_R2", "ROOT_R2"]);
        assert_eq!(bindings.durable_objects, ["SESSION"]);
        assert_eq!(bindings.vars, ["DEV_VAR", "ROOT_VAR"]);
        assert_eq!(bindings.assets.as_deref(), Some("ASSETS"));
    }
}
