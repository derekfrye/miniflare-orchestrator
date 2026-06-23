use serde_json::Value;

pub(super) fn assert_debug_isolated(
    alpha_debug: &Value,
    beta_debug: &Value,
    alpha_id: &str,
    beta_id: &str,
) {
    assert_eq!(
        alpha_debug["lease"]["id"].as_str().expect("alpha debug id"),
        alpha_id
    );
    assert_eq!(
        beta_debug["lease"]["id"].as_str().expect("beta debug id"),
        beta_id
    );
    assert_eq!(
        alpha_debug["lease"]["env_vars"]["ALPHA_TOKEN"],
        "alpha-secret"
    );
    assert_eq!(beta_debug["lease"]["env_vars"]["BETA_TOKEN"], "beta-secret");
    assert!(alpha_debug["lease"]["env_vars"].get("BETA_TOKEN").is_none());
    assert!(beta_debug["lease"]["env_vars"].get("ALPHA_TOKEN").is_none());
    assert!(
        alpha_debug["startup"]["config_file"]
            .as_str()
            .expect("alpha config")
            .contains(alpha_id)
    );
    assert!(
        beta_debug["startup"]["config_file"]
            .as_str()
            .expect("beta config")
            .contains(beta_id)
    );
    assert!(
        alpha_debug["startup"]["injected_env"]["ALPHA_TOKEN"]
            .as_str()
            .expect("alpha token")
            .contains("alpha-secret")
    );
    assert!(
        beta_debug["startup"]["injected_env"]["BETA_TOKEN"]
            .as_str()
            .expect("beta token")
            .contains("beta-secret")
    );
    assert!(
        alpha_debug["startup"]["injected_env"]
            .get("BETA_TOKEN")
            .is_none()
    );
    assert!(
        beta_debug["startup"]["injected_env"]
            .get("ALPHA_TOKEN")
            .is_none()
    );
}

pub(super) fn assert_snapshots_isolated(
    alpha_snapshot: &Value,
    beta_snapshot: &Value,
    alpha_id: &str,
    beta_id: &str,
) {
    assert!(
        alpha_snapshot["runtime_dir"]
            .as_str()
            .expect("alpha runtime")
            .contains(alpha_id)
    );
    assert!(
        beta_snapshot["runtime_dir"]
            .as_str()
            .expect("beta runtime")
            .contains(beta_id)
    );
    assert!(alpha_snapshot["entries"].to_string().contains("alpha.txt"));
    assert!(!alpha_snapshot["entries"].to_string().contains("beta.txt"));
    assert!(beta_snapshot["entries"].to_string().contains("beta.txt"));
    assert!(!beta_snapshot["entries"].to_string().contains("alpha.txt"));
}
