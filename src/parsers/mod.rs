use crate::cache::{FileMetadata, StalenessReport};
use crate::git_batch::BatchGitReader;
use crate::models::EditRecord;
use anyhow::Result;
use std::path::Path;

pub mod claude;
pub mod codex;
pub mod common;

/// Metadata about a trace parser
#[derive(Debug, Clone)]
pub struct ParserInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub file_extensions: Vec<&'static str>,
}

/// Trait for parsing agent trace files into EditRecords
///
/// Implementations should perform single-pass file reading to maximize performance.
pub trait TraceParser: Send + Sync {
    /// Return metadata about this parser
    fn info(&self) -> ParserInfo;

    /// Check if this parser can handle the given file
    ///
    /// Returns:
    /// - `Some(true)` if this parser can definitely handle the file
    /// - `Some(false)` if this parser definitely cannot handle the file
    /// - `None` if unable to determine
    fn can_parse(&self, path: &Path) -> Result<Option<bool>>;

    /// Parse a single trace file into EditRecords (SINGLE PASS)
    ///
    /// Implementations should read the file only once.
    fn parse_file(&self, path: &Path, file_pattern: &str) -> Result<Vec<EditRecord>>;

    /// Parse a single trace file with context (repo root, etc)
    ///
    /// Default implementation delegates to parse_file() and ignores context.
    /// Parsers can override to use context information.
    fn parse_file_with_context(
        &self,
        path: &Path,
        file_pattern: &str,
        repo_root: Option<&Path>,
    ) -> Result<Vec<EditRecord>> {
        let _ = repo_root; // Suppress unused warning by default
        self.parse_file(path, file_pattern)
    }

    /// Parse all trace files in a directory tree with verbose progress reporting
    ///
    /// Default implementation uses collect_trace_files() + parse_file_with_context().
    /// Parsers can override for custom directory handling (e.g., cross-file UUID resolution).
    /// Verbose levels: 0 = silent, 1 = basic, 2+ = detailed per-file progress
    fn parse_directory_with_context_verbose(
        &self,
        dir: &Path,
        file_pattern: &str,
        repo_root: Option<&Path>,
        verbose: u8,
    ) -> Result<Vec<EditRecord>> {
        let mut all_edits = Vec::new();
        let files = self.collect_trace_files(dir)?;

        for (idx, file) in files.iter().enumerate() {
            if verbose >= 3 {
                eprintln!(
                    "[*] Parsing {} file {}/{}: {:?}",
                    self.info().name,
                    idx + 1,
                    files.len(),
                    file.file_name().unwrap_or_default()
                );
            }

            match self.parse_file_with_context(file, file_pattern, repo_root) {
                Ok(mut edits) => all_edits.append(&mut edits),
                Err(e) => {
                    if verbose >= 1 {
                        eprintln!("Warning: Failed to parse {:?}: {}", file, e);
                    }
                }
            }
        }

        Ok(all_edits)
    }

    /// Parse all trace files in a directory tree
    ///
    /// Default implementation uses collect_trace_files() + parse_file_with_context().
    /// Parsers can override for custom directory handling (e.g., cross-file UUID resolution).
    fn parse_directory_with_context(
        &self,
        dir: &Path,
        file_pattern: &str,
        repo_root: Option<&Path>,
    ) -> Result<Vec<EditRecord>> {
        let mut all_edits = Vec::new();
        let files = self.collect_trace_files(dir)?;

        for file in files {
            match self.parse_file_with_context(&file, file_pattern, repo_root) {
                Ok(mut edits) => all_edits.append(&mut edits),
                Err(e) => {
                    eprintln!("Warning: Failed to parse {:?}: {}", file, e);
                }
            }
        }

        Ok(all_edits)
    }

    /// Parse all trace files in a directory tree
    ///
    /// Deprecated: use parse_directory_with_context instead
    fn parse_directory(&self, dir: &Path, file_pattern: &str) -> Result<Vec<EditRecord>> {
        self.parse_directory_with_context(dir, file_pattern, None)
    }

    /// Parse a single trace file with batch git reader for performance
    ///
    /// Default implementation ignores the batch_reader and delegates to parse_file_with_context().
    /// Parsers that need git operations (e.g., Codex) can override to use the batch reader.
    fn parse_file_with_batch(
        &self,
        path: &Path,
        file_pattern: &str,
        repo_root: Option<&Path>,
        batch_reader: Option<&BatchGitReader>,
    ) -> Result<Vec<EditRecord>> {
        let _ = batch_reader; // Default: ignore batch_reader
        self.parse_file_with_context(path, file_pattern, repo_root)
    }

    /// Parse all trace files in a directory tree with batch git reader
    ///
    /// Default implementation creates a batch reader if repo_root is provided,
    /// then calls parse_file_with_batch for each file.
    fn parse_directory_with_batch(
        &self,
        dir: &Path,
        file_pattern: &str,
        repo_root: Option<&Path>,
    ) -> Result<Vec<EditRecord>> {
        // Create batch reader if repo_root available
        let batch_reader = repo_root.and_then(|root| BatchGitReader::new(root).ok());

        let files = self.collect_trace_files(dir)?;
        let mut all_edits = Vec::new();

        for file in files {
            match self.parse_file_with_batch(&file, file_pattern, repo_root, batch_reader.as_ref())
            {
                Ok(mut edits) => all_edits.append(&mut edits),
                Err(e) => eprintln!("Warning: Failed to parse {:?}: {}", file, e),
            }
        }

        Ok(all_edits)
    }

    /// Collect trace files from a directory tree
    ///
    /// Default implementation recursively finds .jsonl files.
    /// Parsers can override for custom file patterns.
    fn collect_trace_files(&self, dir: &Path) -> Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();
        crate::extractor::collect_jsonl_files(dir, &mut files)?;
        Ok(files)
    }

    /// Helper to filter a list of trace files down to those this parser can handle
    ///
    /// This is useful for parsers that need to filter files based on can_parse() results
    /// to avoid tracking metadata for files they cannot parse.
    ///
    /// Note: This calls can_parse() for each file, which may read the first 10 lines.
    /// For large directories, consider caching results or using more efficient filtering
    /// if this becomes a performance bottleneck.
    fn filter_parseable_files(
        &self,
        files: Vec<std::path::PathBuf>,
    ) -> Result<Vec<std::path::PathBuf>> {
        let mut parseable_files = Vec::new();
        for file in files {
            if matches!(self.can_parse(&file)?, Some(true)) {
                parseable_files.push(file);
            }
        }
        Ok(parseable_files)
    }

    /// Check if a single trace file is stale (needs reparsing)
    ///
    /// Default implementation compares modification time and file size.
    /// Parsers can override for provider-specific staleness detection.
    fn is_stale(&self, path: &Path, cached_meta: Option<&FileMetadata>) -> Result<bool> {
        match cached_meta {
            None => Ok(true), // Not in cache = stale
            Some(meta) => {
                // Compare mtime and size
                let current_meta = std::fs::metadata(path)?;
                let current_mtime_ns = current_meta
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_nanos() as i64;
                let current_size = current_meta.len() as i64;

                Ok(current_mtime_ns != meta.file_mtime_ns || current_size != meta.file_size_bytes)
            }
        }
    }

    /// Check staleness for all trace files in a directory
    ///
    /// Default implementation checks each file independently.
    /// Parsers can override for directory-level staleness (e.g., cross-file dependencies).
    fn check_directory_staleness(
        &self,
        dir: &Path,
        cache: &crate::cache::CacheManager,
    ) -> Result<StalenessReport> {
        let files = self.collect_trace_files(dir)?;
        let mut stale_files = Vec::new();
        let mut fresh_files = Vec::new();

        for file in files {
            let cached_meta = cache.get_file_metadata(&file)?;
            if self.is_stale(&file, cached_meta.as_ref())? {
                stale_files.push(file);
            } else {
                fresh_files.push(file);
            }
        }

        Ok(StalenessReport {
            stale_files,
            fresh_files,
        })
    }
}

/// Registry of available trace parsers
pub struct ParserRegistry {
    parsers: Vec<Box<dyn TraceParser>>,
}

impl ParserRegistry {
    /// Create a new registry with all built-in parsers
    pub fn new() -> Self {
        Self {
            parsers: vec![
                Box::new(claude::ClaudeParser::new()),
                Box::new(codex::CodexParser::new()),
            ],
        }
    }

    /// Add a custom parser
    pub fn register(&mut self, parser: Box<dyn TraceParser>) {
        self.parsers.push(parser);
    }

    /// Find the best parser for a given file
    pub fn find_parser(&self, path: &Path) -> Result<Option<&dyn TraceParser>> {
        // Try each parser in order
        for parser in &self.parsers {
            match parser.can_parse(path)? {
                Some(true) => return Ok(Some(parser.as_ref())),
                Some(false) => continue,
                None => continue,
            }
        }
        Ok(None)
    }

    /// Get all registered parsers
    pub fn parsers(&self) -> &[Box<dyn TraceParser>] {
        &self.parsers
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}
