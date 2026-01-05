//! Batch git operations using git cat-file --batch for performance
//!
//! This module provides a thread-safe wrapper around git's native batch mode,
//! allowing multiple file reads from a single long-lived git process instead of
//! spawning a new process for each file. This dramatically reduces overhead when
//! reading hundreds or thousands of files.

use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

/// Thread-safe wrapper for git cat-file --batch process
///
/// This struct manages a single long-lived `git cat-file --batch` process and
/// provides thread-safe access to read file contents from specific commits.
/// Multiple threads can safely share the same BatchGitReader.
///
/// # Example
/// ```ignore
/// let batch_reader = BatchGitReader::new("/path/to/repo")?;
/// let content = batch_reader.get_file_content("abc123def", "src/main.rs")?;
/// ```
#[derive(Clone)]
pub struct BatchGitReader {
    inner: Arc<Mutex<BatchGitReaderInner>>,
}

struct BatchGitReaderInner {
    process: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl BatchGitReader {
    /// Create a new batch reader for the given repository
    ///
    /// This spawns a `git cat-file --batch` process that remains alive for
    /// the lifetime of the BatchGitReader. The process is automatically cleaned
    /// up when the BatchGitReader is dropped.
    pub fn new(repo_root: &Path) -> Result<Self> {
        let mut process = Command::new("git")
            .arg("cat-file")
            .arg("--batch")
            .current_dir(repo_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn git cat-file --batch")?;

        let stdin = BufWriter::new(process.stdin.take().context("Failed to open git stdin")?);
        let stdout = BufReader::new(process.stdout.take().context("Failed to open git stdout")?);

        Ok(Self {
            inner: Arc::new(Mutex::new(BatchGitReaderInner {
                process,
                stdin,
                stdout,
            })),
        })
    }

    /// Read file content from git at specific commit (thread-safe)
    ///
    /// Sends a request to the batch git process in the form "commit:path\n"
    /// and reads the response.
    ///
    /// # Returns
    /// - `Ok(String)` containing the file content if found
    /// - `Err` if the object doesn't exist or there's an I/O error
    pub fn get_file_content(&self, commit_id: &str, file_path: &str) -> Result<String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?;

        // Write request: "commit:path\n"
        writeln!(inner.stdin, "{}:{}", commit_id, file_path)?;
        inner.stdin.flush()?;

        // Read response header: "<sha> <type> <size>\n"
        let mut header = String::new();
        inner.stdout.read_line(&mut header)?;

        // Parse header
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() < 3 {
            anyhow::bail!("Invalid git cat-file response: {}", header.trim());
        }

        // Check for "missing" response
        if parts[1] == "missing" {
            anyhow::bail!("Object not found in git: {}:{}", commit_id, file_path);
        }

        // Parse size
        let size: usize = parts[2]
            .parse()
            .context("Failed to parse content size from git response")?;

        // Read content (exactly 'size' bytes)
        let mut content = vec![0u8; size];
        std::io::Read::read_exact(&mut inner.stdout, &mut content)
            .context("Failed to read object content from git")?;

        // Read trailing newline that git adds after each object
        let mut newline = [0u8; 1];
        std::io::Read::read_exact(&mut inner.stdout, &mut newline)
            .context("Failed to read trailing newline from git")?;

        Ok(String::from_utf8_lossy(&content).to_string())
    }
}

impl Drop for BatchGitReaderInner {
    fn drop(&mut self) {
        // Ensure process cleanup
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_test_repo() -> Result<(TempDir, String)> {
        let dir = TempDir::new()?;
        let repo_path = dir.path().to_string_lossy().to_string();

        // Initialize git repo
        Command::new("git")
            .arg("init")
            .current_dir(&repo_path)
            .output()?;

        // Configure git
        Command::new("git")
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .current_dir(&repo_path)
            .output()?;

        Command::new("git")
            .arg("config")
            .arg("user.name")
            .arg("Test User")
            .current_dir(&repo_path)
            .output()?;

        Ok((dir, repo_path))
    }

    #[test]
    fn test_batch_reader_valid_file() -> Result<()> {
        let (_dir, repo_path) = init_test_repo()?;

        // Create a test file
        std::fs::write(
            std::path::Path::new(&repo_path).join("test.txt"),
            "hello world",
        )?;

        // Commit it
        Command::new("git")
            .arg("add")
            .arg("test.txt")
            .current_dir(&repo_path)
            .output()?;

        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("initial commit")
            .current_dir(&repo_path)
            .output()?;

        let commit_sha = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(&repo_path)
            .output()?;

        let commit_id = String::from_utf8(commit_sha.stdout)?.trim().to_string();

        // Test batch reader
        let batch_reader = BatchGitReader::new(Path::new(&repo_path))?;
        let content = batch_reader.get_file_content(&commit_id, "test.txt")?;

        assert_eq!(content, "hello world");
        Ok(())
    }

    #[test]
    fn test_batch_reader_missing_object() -> Result<()> {
        let (_dir, repo_path) = init_test_repo()?;

        let batch_reader = BatchGitReader::new(Path::new(&repo_path))?;
        let result = batch_reader.get_file_content(
            "0000000000000000000000000000000000000000",
            "nonexistent.txt",
        );

        assert!(result.is_err());
        Ok(())
    }
}
