use ai_blame::models::*;
use chrono::Utc;

#[test]
fn test_file_history_first_last_edit() {
    let mut history = FileHistory {
        file_path: "test.txt".to_string(),
        events: vec![],
    };

    // Empty history
    assert!(history.first_edit().is_none());
    assert!(history.last_edit().is_none());

    // Add events
    let now = Utc::now();
    history.events.push(CurationEvent {
        timestamp: now - chrono::Duration::hours(2),
        model: Some("model-1".to_string()),
        action: Some(CurationAction::Created),
        description: None,
        agent_tool: Some("claude-code".to_string()),
        agent_version: None,
    });

    history.events.push(CurationEvent {
        timestamp: now,
        model: Some("model-2".to_string()),
        action: Some(CurationAction::Edited),
        description: None,
        agent_tool: Some("claude-code".to_string()),
        agent_version: None,
    });

    assert!(history.first_edit().is_some());
    assert!(history.last_edit().is_some());
    assert!(history.first_edit().unwrap() < history.last_edit().unwrap());
}

#[test]
fn test_filter_config_default() {
    let config = FilterConfig::default();
    assert!(!config.initial_and_recent_only);
    assert_eq!(config.min_change_size, 0);
    assert!(config.file_pattern.is_none());
}

#[test]
fn test_curation_action_serialization() {
    let action = CurationAction::Created;
    let serialized = serde_json::to_string(&action).unwrap();
    assert_eq!(serialized, "\"CREATED\"");

    let action = CurationAction::Edited;
    let serialized = serde_json::to_string(&action).unwrap();
    assert_eq!(serialized, "\"EDITED\"");
}
