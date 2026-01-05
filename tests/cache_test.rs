use ai_blame::cache::CacheManager;
use ai_blame::models::EditRecord;
use chrono::Utc;
use std::fs;
use tempfile::TempDir;

/// Test basic cache creation and initialization
#[test]
fn test_cache_creation() {
    let temp = TempDir::new().unwrap();
    let cache = CacheManager::open(temp.path()).unwrap();

    // Cache should be created
    assert!(cache.db_path().exists());
    assert_eq!(cache.db_path().file_name().unwrap(), ".ai-blame.ddb");
}

/// Test storing and retrieving edits
#[test]
fn test_cache_round_trip() {
    let temp = TempDir::new().unwrap();
    let cache = CacheManager::open(temp.path()).unwrap();

    // Create a trace file
    let trace_file = temp.path().join("test.jsonl");
    fs::write(&trace_file, "{}").unwrap();

    // Create edits to store
    let edits = vec![
        EditRecord {
            file_path: "/test/file1.rs".to_string(),
            timestamp: Utc::now(),
            model: "claude-opus".to_string(),
            session_id: "session-1".to_string(),
            is_create: true,
            change_size: 100,
            agent_tool: "claude-code".to_string(),
            agent_version: None,
            old_string: None,
            new_string: None,
            structured_patch: None,
            create_content: Some("fn main() {}".to_string()),
        },
        EditRecord {
            file_path: "/test/file2.rs".to_string(),
            timestamp: Utc::now(),
            model: "claude-opus".to_string(),
            session_id: "session-1".to_string(),
            is_create: false,
            change_size: 50,
            agent_tool: "claude-code".to_string(),
            agent_version: Some("1.0".to_string()),
            old_string: Some("old code".to_string()),
            new_string: Some("new code".to_string()),
            structured_patch: Some("--- a\n+++ b".to_string()),
            create_content: None,
        },
    ];

    // Store edits
    cache
        .store_edits(&trace_file, "claude", &edits, 50)
        .unwrap();

    // Retrieve edits
    let retrieved = cache.get_cached_edits(&trace_file).unwrap().unwrap();
    assert_eq!(retrieved.len(), 2);
    assert_eq!(retrieved[0].file_path, "/test/file1.rs");
    assert_eq!(retrieved[1].file_path, "/test/file2.rs");
    assert!(retrieved[0].is_create);
    assert!(!retrieved[1].is_create);
}

/// Test file metadata tracking
#[test]
fn test_metadata_tracking() {
    let temp = TempDir::new().unwrap();
    let cache = CacheManager::open(temp.path()).unwrap();

    let trace_file = temp.path().join("test.jsonl");
    fs::write(&trace_file, "test content").unwrap();

    // No metadata initially
    assert!(cache.get_file_metadata(&trace_file).unwrap().is_none());

    // Store edits
    cache.store_edits(&trace_file, "claude", &[], 0).unwrap();

    // Metadata should exist
    let meta = cache.get_file_metadata(&trace_file).unwrap().unwrap();
    assert!(meta.file_mtime_ns > 0);
    assert_eq!(meta.file_size_bytes, 12); // "test content" = 12 bytes
}

/// Test staleness detection - file modification
#[test]
fn test_staleness_file_modification() {
    use std::thread;
    use std::time::Duration;

    let temp = TempDir::new().unwrap();
    let trace_file = temp.path().join("test.jsonl");
    fs::write(&trace_file, "initial").unwrap();

    let cache = CacheManager::open(temp.path()).unwrap();

    // Store initial edits
    cache.store_edits(&trace_file, "claude", &[], 0).unwrap();
    let meta1 = cache.get_file_metadata(&trace_file).unwrap().unwrap();

    // Wait to ensure timestamp changes (filesystem granularity)
    thread::sleep(Duration::from_millis(1500));

    // Modify file and update cache
    fs::write(&trace_file, "modified content").unwrap();
    cache.store_edits(&trace_file, "claude", &[], 0).unwrap();

    let meta2 = cache.get_file_metadata(&trace_file).unwrap().unwrap();

    // Modification time should differ
    assert_ne!(meta1.file_mtime_ns, meta2.file_mtime_ns);
    assert_ne!(meta1.file_size_bytes, meta2.file_size_bytes);
}

/// Test multiple files in cache
#[test]
fn test_multiple_files() {
    let temp = TempDir::new().unwrap();
    let cache = CacheManager::open(temp.path()).unwrap();

    let file1 = temp.path().join("file1.jsonl");
    let file2 = temp.path().join("file2.jsonl");
    let file3 = temp.path().join("file3.jsonl");

    fs::write(&file1, "{}").unwrap();
    fs::write(&file2, "{}").unwrap();
    fs::write(&file3, "{}").unwrap();

    // Store edits for each file
    let edits1 = vec![EditRecord {
        file_path: "/src/a.rs".to_string(),
        timestamp: Utc::now(),
        model: "claude".to_string(),
        session_id: "s1".to_string(),
        is_create: true,
        change_size: 10,
        agent_tool: "claude-code".to_string(),
        agent_version: None,
        old_string: None,
        new_string: None,
        structured_patch: None,
        create_content: Some("a".to_string()),
    }];

    let edits2 = vec![EditRecord {
        file_path: "/src/b.rs".to_string(),
        timestamp: Utc::now(),
        model: "claude".to_string(),
        session_id: "s2".to_string(),
        is_create: true,
        change_size: 20,
        agent_tool: "claude-code".to_string(),
        agent_version: None,
        old_string: None,
        new_string: None,
        structured_patch: None,
        create_content: Some("bb".to_string()),
    }];

    cache.store_edits(&file1, "claude", &edits1, 10).unwrap();
    cache.store_edits(&file2, "claude", &edits2, 20).unwrap();
    cache.store_edits(&file3, "claude", &[], 0).unwrap();

    // Verify retrieval
    assert_eq!(cache.get_cached_edits(&file1).unwrap().unwrap().len(), 1);
    assert_eq!(cache.get_cached_edits(&file2).unwrap().unwrap().len(), 1);
    assert_eq!(cache.get_cached_edits(&file3).unwrap().unwrap().len(), 0);

    // File not in cache should return None
    let nonexistent = temp.path().join("nonexistent.jsonl");
    assert!(cache.get_cached_edits(&nonexistent).unwrap().is_none());
}

/// Test file invalidation
#[test]
fn test_file_invalidation() {
    let temp = TempDir::new().unwrap();
    let cache = CacheManager::open(temp.path()).unwrap();

    let file1 = temp.path().join("file1.jsonl");
    let file2 = temp.path().join("file2.jsonl");
    fs::write(&file1, "{}").unwrap();
    fs::write(&file2, "{}").unwrap();

    let edits = vec![EditRecord {
        file_path: "/test.rs".to_string(),
        timestamp: Utc::now(),
        model: "claude".to_string(),
        session_id: "s".to_string(),
        is_create: true,
        change_size: 10,
        agent_tool: "claude-code".to_string(),
        agent_version: None,
        old_string: None,
        new_string: None,
        structured_patch: None,
        create_content: Some("test".to_string()),
    }];

    cache.store_edits(&file1, "claude", &edits, 10).unwrap();
    cache.store_edits(&file2, "claude", &edits, 10).unwrap();

    // Verify both cached
    assert!(cache.get_cached_edits(&file1).unwrap().is_some());
    assert!(cache.get_cached_edits(&file2).unwrap().is_some());

    // Invalidate file1
    cache
        .invalidate_files(std::slice::from_ref(&file1))
        .unwrap();

    // Only file2 should remain
    assert!(cache.get_cached_edits(&file1).unwrap().is_none());
    assert!(cache.get_cached_edits(&file2).unwrap().is_some());
}

/// Test cache with special characters in paths
#[test]
fn test_special_chars_in_paths() {
    let temp = TempDir::new().unwrap();
    let cache = CacheManager::open(temp.path()).unwrap();

    let trace_file = temp.path().join("test.jsonl");
    fs::write(&trace_file, "{}").unwrap();

    let edits = vec![
        EditRecord {
            file_path: "/path/with spaces/file.rs".to_string(),
            timestamp: Utc::now(),
            model: "claude".to_string(),
            session_id: "s".to_string(),
            is_create: true,
            change_size: 10,
            agent_tool: "claude-code".to_string(),
            agent_version: None,
            old_string: None,
            new_string: None,
            structured_patch: None,
            create_content: Some("test".to_string()),
        },
        EditRecord {
            file_path: "/path/with'quotes/file.rs".to_string(),
            timestamp: Utc::now(),
            model: "claude".to_string(),
            session_id: "s".to_string(),
            is_create: true,
            change_size: 10,
            agent_tool: "claude-code".to_string(),
            agent_version: None,
            old_string: None,
            new_string: None,
            structured_patch: None,
            create_content: Some("test".to_string()),
        },
    ];

    cache
        .store_edits(&trace_file, "claude", &edits, 10)
        .unwrap();
    let retrieved = cache.get_cached_edits(&trace_file).unwrap().unwrap();

    assert_eq!(retrieved.len(), 2);
    assert_eq!(retrieved[0].file_path, "/path/with spaces/file.rs");
    assert_eq!(retrieved[1].file_path, "/path/with'quotes/file.rs");
}

/// Test cache persistence across instances
#[test]
fn test_persistence() {
    let temp = TempDir::new().unwrap();
    let trace_file = temp.path().join("test.jsonl");
    fs::write(&trace_file, "{}").unwrap();

    let edits = vec![EditRecord {
        file_path: "/test.rs".to_string(),
        timestamp: Utc::now(),
        model: "claude".to_string(),
        session_id: "s".to_string(),
        is_create: true,
        change_size: 10,
        agent_tool: "claude-code".to_string(),
        agent_version: None,
        old_string: None,
        new_string: None,
        structured_patch: None,
        create_content: Some("test".to_string()),
    }];

    // Create cache and store data
    {
        let cache = CacheManager::open(temp.path()).unwrap();
        cache
            .store_edits(&trace_file, "claude", &edits, 10)
            .unwrap();
    }

    // Create new cache instance and verify data is still there
    {
        let cache = CacheManager::open(temp.path()).unwrap();
        let retrieved = cache.get_cached_edits(&trace_file).unwrap().unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].file_path, "/test.rs");
    }
}

/// Test cache with large number of edits
#[test]
fn test_large_edit_count() {
    let temp = TempDir::new().unwrap();
    let cache = CacheManager::open(temp.path()).unwrap();

    let trace_file = temp.path().join("test.jsonl");
    fs::write(&trace_file, "{}").unwrap();

    // Create 1000 edits
    let mut edits = Vec::new();
    for i in 0..1000 {
        edits.push(EditRecord {
            file_path: format!("/file{}.rs", i % 10), // 10 different files
            timestamp: Utc::now(),
            model: "claude".to_string(),
            session_id: format!("s{}", i % 5), // 5 different sessions
            is_create: i % 10 == 0,
            change_size: 10 + (i % 100),
            agent_tool: "claude-code".to_string(),
            agent_version: None,
            old_string: None,
            new_string: None,
            structured_patch: None,
            create_content: if i % 10 == 0 {
                Some(format!("content {}", i))
            } else {
                None
            },
        });
    }

    cache
        .store_edits(&trace_file, "claude", &edits, 100)
        .unwrap();
    let retrieved = cache.get_cached_edits(&trace_file).unwrap().unwrap();

    assert_eq!(retrieved.len(), 1000);
}
