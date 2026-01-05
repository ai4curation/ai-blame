/// Utility functions for safe string and path operations.
/// Safely truncate a string to a maximum number of characters, respecting UTF-8 boundaries.
/// Returns the truncated string with "..." appended if truncation occurred.
pub fn safe_truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Safely truncate a string to a maximum number of characters, respecting UTF-8 boundaries.
/// Returns the truncated string with the specified suffix if truncation occurred.
pub fn safe_truncate_with_suffix(s: &str, max_chars: usize, suffix: &str) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let suffix_len = suffix.chars().count();
        let max_content = max_chars.saturating_sub(suffix_len);
        let truncated: String = s.chars().take(max_content).collect();
        format!("{}{}", truncated, suffix)
    }
}

/// Validate that a path is safe (relative, doesn't contain path traversal)
pub fn validate_safe_path(path: &str) -> Result<std::path::PathBuf, String> {
    let path_buf = std::path::PathBuf::from(path);

    // Check for absolute paths
    if path_buf.is_absolute() {
        return Err("Absolute paths are not allowed".to_string());
    }

    // Check for path traversal attempts
    if path.contains("..") {
        return Err("Path traversal is not allowed".to_string());
    }

    // Check for invalid characters that could be used maliciously
    if path.contains('\0') {
        return Err("Null characters in paths are not allowed".to_string());
    }

    Ok(path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_truncate() {
        assert_eq!(safe_truncate("hello", 10), "hello");
        assert_eq!(safe_truncate("hello world", 5), "he...");
        assert_eq!(safe_truncate("hello", 5), "hello");
        assert_eq!(safe_truncate("hello", 3), "...");
    }

    #[test]
    fn test_safe_truncate_with_utf8() {
        // Test with multi-byte UTF-8 characters
        assert_eq!(safe_truncate("🚀🌟💫", 2), "...");
        assert_eq!(safe_truncate("🚀🌟💫", 3), "🚀🌟💫");
        assert_eq!(safe_truncate("hello🚀world", 8), "hello...");
    }

    #[test]
    fn test_validate_safe_path() {
        // Valid paths
        assert!(validate_safe_path("hello.txt").is_ok());
        assert!(validate_safe_path("dir/file.txt").is_ok());

        // Invalid paths
        assert!(validate_safe_path("/etc/passwd").is_err());
        assert!(validate_safe_path("../../../etc/passwd").is_err());
        assert!(validate_safe_path("dir/../../../etc/passwd").is_err());
        assert!(validate_safe_path("file\0.txt").is_err());
    }
}
