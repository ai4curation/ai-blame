use ai_blame::{config::*, models::*};

#[test]
fn test_default_config() {
    let config = get_default_config();

    // Check that defaults exist
    assert!(config.defaults.is_some());
    let defaults = config.defaults.as_ref().unwrap();
    assert_eq!(defaults.policy, OutputPolicy::Sidecar);

    // Check that yaml rule exists
    let yaml_rule = config.get_rule_for_file("test.yaml");
    assert!(yaml_rule.is_some());
    assert_eq!(yaml_rule.unwrap().policy, OutputPolicy::Append);

    // Check that json rule exists
    let json_rule = config.get_rule_for_file("test.json");
    assert!(json_rule.is_some());
    let jr = json_rule.unwrap();
    assert_eq!(jr.policy, OutputPolicy::Append);
    assert_eq!(jr.format, "json");
}

#[test]
fn test_resolve_sidecar_path() {
    use std::path::Path;

    let source = Path::new("src/foo.py");
    let pattern = "{stem}.history.yaml";
    let result = resolve_sidecar_path(source, pattern);

    assert_eq!(
        result.file_name().unwrap().to_str().unwrap(),
        "foo.history.yaml"
    );
}

#[test]
fn test_output_policy_deserialization() {
    let yaml = r#"
defaults:
  policy: sidecar
  sidecar_pattern: "{stem}.history.yaml"
rules:
  - pattern: "*.yaml"
    policy: append
  - pattern: "*.py"
    policy: comment
    comment_syntax: hash
"#;

    let config: OutputConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(config.defaults.is_some());
    assert_eq!(config.rules.len(), 2);
    assert_eq!(config.rules[0].policy, OutputPolicy::Append);
    assert_eq!(config.rules[1].policy, OutputPolicy::Comment);
}

#[test]
fn test_seed_config_writes_file_and_refuses_overwrite_without_force() {
    let dir = tempfile::tempdir().unwrap();

    let path = write_seed_config(dir.path(), SeedFlavor::Sidecar, false).unwrap();
    assert!(path.ends_with(CONFIG_FILENAME));
    let contents = std::fs::read_to_string(&path).unwrap();
    assert_eq!(contents, seed_config_contents(SeedFlavor::Sidecar));

    // Second write without force should fail.
    let err = write_seed_config(dir.path(), SeedFlavor::Sidecar, false).unwrap_err();
    assert!(format!("{err:?}").contains("Config already exists"));

    // With force it should succeed.
    let _path2 = write_seed_config(dir.path(), SeedFlavor::InPlace, true).unwrap();
}
