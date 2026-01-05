use crate::blame::{compute_line_blame, group_blocks, BlameBlock, BlameMeta, LineBlame};
use crate::config::{
    find_config, get_default_config, load_config, resolve_sidecar_path, write_seed_config,
    SeedFlavor,
};
use crate::extractor::{apply_filters, convert_to_file_histories};
use crate::models::*;
use crate::updater::{apply_rule, preview_update};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlameColumn {
    Agent,
    Model,
    Timestamp,
    Line,
    Code,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranscriptColumn {
    Session,
    Agent,
    Timestamp,
    Messages,
    Files,
    Models,
    LastMessage,
}

#[derive(Debug, Clone, Default)]
struct AliasConfig {
    agent_aliases: HashMap<String, String>,
    model_aliases: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct BlameConfig {
    file: String,
    trace_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    lines: Option<String>,
    blocks: bool,
    show_agent: bool,
    columns: Option<String>,
    agent_alias: Vec<(String, String)>,
    model_alias: Vec<(String, String)>,
    no_header: bool,
}

#[derive(Parser)]
#[command(name = "ai-blame")]
#[command(about = "Extract provenance from Claude Code traces", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a starter `.ai-blame.yaml` config file in the target directory
    Init {
        /// Target project directory to write `.ai-blame.yaml` into (default: cwd)
        #[arg(short = 'd', long)]
        dir: Option<PathBuf>,

        /// Config template flavor
        #[arg(long, value_enum, default_value = "sidecar")]
        flavor: InitFlavor,

        /// Overwrite an existing `.ai-blame.yaml` if present
        #[arg(long)]
        force: bool,
    },

    /// Write a stdout report summarizing curation history (no filesystem changes)
    Report {
        /// Specific file to filter results
        target: Option<String>,

        /// Claude trace directory (overrides --dir and --home)
        #[arg(short = 't', long)]
        trace_dir: Option<PathBuf>,

        /// Target project directory (default: cwd)
        #[arg(short = 'd', long)]
        dir: Option<PathBuf>,

        /// Home directory where .claude/ lives (default: ~)
        #[arg(long)]
        home: Option<PathBuf>,

        /// Config file path (default: auto-find .ai-blame.yaml)
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,

        /// Only keep first and last edit per file
        #[arg(long)]
        initial_and_recent: bool,

        /// Skip intermediate edits smaller than N chars
        #[arg(short = 'm', long, default_value = "0")]
        min_change_size: usize,

        /// Show all YAML previews (not just first 5)
        #[arg(long)]
        show_all: bool,

        /// Filter files by path pattern
        #[arg(short = 'p', long, default_value = "")]
        pattern: String,

        /// Increase verbosity (use -v for basic, -vv for detailed)
        #[arg(short = 'v', long, action = clap::ArgAction::Count)]
        verbose: u8,

        /// Skip Codex/Copilot traces (faster for Claude-only analysis)
        #[arg(long)]
        skip_codex: bool,

        /// Only analyze Claude traces (alias for --skip-codex)
        #[arg(long)]
        only_claude: bool,

        /// Disable cache (always reparse traces)
        #[arg(long)]
        no_cache: bool,

        /// Rebuild cache (invalidate all cached data)
        #[arg(long)]
        rebuild_cache: bool,
    },

    /// Annotate files or write sidecars/comments using output rules (writes by default)
    Annotate {
        /// Specific file to filter results
        target: Option<String>,

        /// Claude trace directory (overrides --dir and --home)
        #[arg(short = 't', long)]
        trace_dir: Option<PathBuf>,

        /// Target project directory (default: cwd)
        #[arg(short = 'd', long)]
        dir: Option<PathBuf>,

        /// Home directory where .claude/ lives (default: ~)
        #[arg(long)]
        home: Option<PathBuf>,

        /// Config file path (default: auto-find .ai-blame.yaml)
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,

        /// Don't write anything; print what would happen
        #[arg(long)]
        dry_run: bool,

        /// Only keep first and last edit per file
        #[arg(long)]
        initial_and_recent: bool,

        /// Skip intermediate edits smaller than N chars
        #[arg(short = 'm', long, default_value = "0")]
        min_change_size: usize,

        /// Filter files by path pattern
        #[arg(short = 'p', long, default_value = "")]
        pattern: String,

        /// Disable cache (always reparse traces)
        #[arg(long)]
        no_cache: bool,

        /// Rebuild cache (invalidate all cached data)
        #[arg(long)]
        rebuild_cache: bool,
    },

    /// Show statistics about available traces
    Stats {
        /// Claude trace directory (overrides --dir and --home)
        #[arg(short = 't', long)]
        trace_dir: Option<PathBuf>,

        /// Target project directory (default: cwd)
        #[arg(short = 'd', long)]
        dir: Option<PathBuf>,

        /// Home directory where .claude/ lives (default: ~)
        #[arg(long)]
        home: Option<PathBuf>,

        /// Filter files by path pattern
        #[arg(short = 'p', long, default_value = "")]
        pattern: String,

        /// Increase verbosity (use -v for basic, -vv for detailed)
        #[arg(short = 'v', long, action = clap::ArgAction::Count)]
        verbose: u8,

        /// Skip Codex/Copilot traces (faster for Claude-only analysis)
        #[arg(long)]
        skip_codex: bool,

        /// Only analyze Claude traces (alias for --skip-codex)
        #[arg(long)]
        only_claude: bool,

        /// Disable cache (always reparse traces)
        #[arg(long)]
        no_cache: bool,

        /// Rebuild cache (invalidate all cached data)
        #[arg(long)]
        rebuild_cache: bool,
    },

    /// Show git-blame-like line (and optional block) attribution for a file
    Blame {
        /// File to show blame for (path relative to cwd, or absolute)
        file: String,

        /// Claude trace directory (overrides --dir and --home)
        #[arg(short = 't', long)]
        trace_dir: Option<PathBuf>,

        /// Target project directory (default: cwd)
        #[arg(short = 'd', long)]
        dir: Option<PathBuf>,

        /// Home directory where .claude/ lives (default: ~)
        #[arg(long)]
        home: Option<PathBuf>,

        /// Restrict output to a line range like "10-20"
        #[arg(long)]
        lines: Option<String>,

        /// Show block boundaries (consecutive lines attributed to the same event)
        #[arg(long)]
        blocks: bool,

        /// Include agent/tool information (agent_tool[@agent_version]) in the output when using
        /// the default columns. Has no effect if --columns is provided; in that case, include
        /// 'A' in the --columns layout string to show the Agent column.
        #[arg(long)]
        show_agent: bool,

        /// Column layout string (e.g. AMTLC for agent/model/timestamp/line/code). When provided,
        /// this overrides --show-agent and all other defaults.
        #[arg(long)]
        columns: Option<String>,

        /// Map agent tool names to abbreviations (format: FROM=TO; repeatable)
        #[arg(long = "agent-alias", value_parser = parse_alias_pair)]
        agent_alias: Vec<(String, String)>,

        /// Map model names to abbreviations (format: FROM=TO; repeatable)
        #[arg(long = "model-alias", value_parser = parse_alias_pair)]
        model_alias: Vec<(String, String)>,

        /// Disable cache (always reparse traces)
        #[arg(long)]
        no_cache: bool,

        /// Rebuild cache (invalidate all cached data)
        #[arg(long)]
        rebuild_cache: bool,

        /// Suppress the file metadata header (creation date, etc.)
        #[arg(long)]
        no_header: bool,
    },

    /// Show timeline of actions in the repository
    Timeline {
        /// Claude trace directory (overrides --dir and --home)
        #[arg(short = 't', long)]
        trace_dir: Option<PathBuf>,

        /// Target project directory (default: cwd)
        #[arg(short = 'd', long)]
        dir: Option<PathBuf>,

        /// Home directory where .claude/ lives (default: ~)
        #[arg(long)]
        home: Option<PathBuf>,

        /// Filter files by path pattern
        #[arg(short = 'p', long, default_value = "")]
        pattern: String,

        /// Increase verbosity (use -v for basic, -vv for detailed)
        #[arg(short = 'v', long, action = clap::ArgAction::Count)]
        verbose: u8,

        /// Skip Codex/Copilot traces (faster for Claude-only analysis)
        #[arg(long)]
        skip_codex: bool,

        /// Only analyze Claude traces (alias for --skip-codex)
        #[arg(long)]
        only_claude: bool,

        /// Limit to N most recent entries (0 for all)
        #[arg(short = 'n', long, default_value = "50")]
        limit: usize,
    },

    /// View conversation transcripts from AI agent sessions
    Transcript {
        #[command(subcommand)]
        action: TranscriptAction,
    },

    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum TranscriptAction {
    /// List all available transcripts
    List {
        /// Claude trace directory (overrides --dir and --home)
        #[arg(short = 't', long)]
        trace_dir: Option<PathBuf>,

        /// Target project directory (default: cwd)
        #[arg(short = 'd', long)]
        dir: Option<PathBuf>,

        /// Home directory where .claude/ lives (default: ~)
        #[arg(long)]
        home: Option<PathBuf>,

        /// Limit to N most recent transcripts (0 for all)
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,

        /// Output format (table, json)
        #[arg(long, value_enum, default_value = "table")]
        format: TranscriptFormat,

        /// Columns to display: S=Session, A=Agent, T=Timestamp, M=Messages, F=Files, O=mOdels, L=Last message.
        /// Default is 'SATM' for table format.
        #[arg(long)]
        columns: Option<String>,
    },

    /// View a specific transcript
    View {
        /// Session ID or file path of the transcript to view
        session: String,

        /// Claude trace directory (overrides --dir and --home)
        #[arg(short = 't', long)]
        trace_dir: Option<PathBuf>,

        /// Target project directory (default: cwd)
        #[arg(short = 'd', long)]
        dir: Option<PathBuf>,

        /// Home directory where .claude/ lives (default: ~)
        #[arg(long)]
        home: Option<PathBuf>,

        /// Output format (text, json, markdown)
        #[arg(long, value_enum, default_value = "text")]
        format: TranscriptViewFormat,

        /// Show full content (don't truncate long messages)
        #[arg(long)]
        full: bool,

        /// Show thinking/reasoning blocks
        #[arg(long)]
        show_thinking: bool,

        /// Show tool use details
        #[arg(long)]
        show_tools: bool,
    },

    /// Search across all transcripts for matching content
    Search {
        /// Search query (text or regex pattern)
        query: String,

        /// Claude trace directory (overrides --dir and --home)
        #[arg(short = 't', long)]
        trace_dir: Option<PathBuf>,

        /// Target project directory (default: cwd)
        #[arg(short = 'd', long)]
        dir: Option<PathBuf>,

        /// Home directory where .claude/ lives (default: ~)
        #[arg(long)]
        home: Option<PathBuf>,

        /// Treat query as a regex pattern
        #[arg(short = 'e', long)]
        regex: bool,

        /// Case-sensitive search (default: case-insensitive)
        #[arg(short = 's', long)]
        case_sensitive: bool,

        /// Filter by session ID pattern
        #[arg(long)]
        session: Option<String>,

        /// Filter by agent tool (e.g., "claude-code", "codex")
        #[arg(long)]
        agent: Option<String>,

        /// Filter by model name (e.g., "opus", "sonnet")
        #[arg(long)]
        model: Option<String>,

        /// Only include transcripts since this date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,

        /// Only include transcripts until this date (YYYY-MM-DD)
        #[arg(long)]
        until: Option<String>,

        /// Limit to N matching transcripts (0 for all)
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,

        /// Output format (table, json)
        #[arg(long, value_enum, default_value = "table")]
        format: TranscriptFormat,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TranscriptFormat {
    Table,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TranscriptViewFormat {
    Text,
    Json,
    Markdown,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InitFlavor {
    /// Defaults to sidecar output (minimizes edits to existing files).
    Sidecar,
    /// Prefer in-place output when possible (append for YAML/JSON, comments for common code/doc types).
    InPlace,
}

const AGENT_WIDTH: usize = 18;
const MODEL_WIDTH: usize = 20;
const TIMESTAMP_WIDTH: usize = 16;
const LINE_WIDTH: usize = 5;
const MIN_HEADER_LINE_LENGTH: usize = 10;

fn parse_alias_pair(s: &str) -> Result<(String, String), String> {
    let (from, to) = s
        .split_once('=')
        .ok_or_else(|| "expected FROM=TO".to_string())?;
    if from.trim().is_empty() || to.trim().is_empty() {
        return Err("alias parts cannot be empty".to_string());
    }
    Ok((from.trim().to_string(), to.trim().to_string()))
}

fn apply_alias(value: &str, aliases: &HashMap<String, String>) -> String {
    aliases
        .get(value)
        .cloned()
        .unwrap_or_else(|| value.to_string())
}

fn parse_column_spec(spec: Option<&str>, show_agent: bool) -> Result<Vec<BlameColumn>, String> {
    let mut cols = Vec::new();
    if let Some(spec) = spec {
        for ch in spec.chars() {
            let col = match ch {
                'A' | 'a' => Some(BlameColumn::Agent),
                'M' | 'm' => Some(BlameColumn::Model),
                'T' | 't' => Some(BlameColumn::Timestamp),
                'L' | 'l' => Some(BlameColumn::Line),
                'C' | 'c' => Some(BlameColumn::Code),
                _ if ch.is_whitespace() => None,
                _ => {
                    return Err(format!(
                        "invalid column specifier '{}'; allowed: A,M,T,L,C",
                        ch
                    ))
                }
            };
            if let Some(c) = col {
                cols.push(c);
            }
        }
    } else if show_agent {
        cols = vec![
            BlameColumn::Agent,
            BlameColumn::Model,
            BlameColumn::Timestamp,
            BlameColumn::Line,
            BlameColumn::Code,
        ];
    } else {
        cols = vec![
            BlameColumn::Model,
            BlameColumn::Timestamp,
            BlameColumn::Line,
            BlameColumn::Code,
        ];
    }

    if cols.is_empty() {
        return Err("column list cannot be empty".to_string());
    }
    Ok(cols)
}

fn pad(value: String, width: usize, align_right: bool) -> String {
    if align_right {
        format!("{:>width$}", value)
    } else {
        format!("{:<width$}", value)
    }
}

fn truncate(value: String, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut chars = value.chars();
    let mut truncated = String::new();
    for _ in 0..width {
        match chars.next() {
            Some(c) => truncated.push(c),
            None => return truncated,
        }
    }
    if chars.next().is_some() && !truncated.is_empty() {
        truncated = truncated.chars().take(width.saturating_sub(1)).collect();
        truncated.push('…');
    }
    truncated
}

fn column_label(column: BlameColumn) -> &'static str {
    match column {
        BlameColumn::Agent => "agent",
        BlameColumn::Model => "model",
        BlameColumn::Timestamp => "timestamp",
        BlameColumn::Line => "line",
        BlameColumn::Code => "code",
    }
}

fn column_width_and_alignment(column: BlameColumn) -> (Option<usize>, bool) {
    match column {
        BlameColumn::Agent => (Some(AGENT_WIDTH), false),
        BlameColumn::Model => (Some(MODEL_WIDTH), false),
        BlameColumn::Timestamp => (Some(TIMESTAMP_WIDTH), false),
        BlameColumn::Line => (Some(LINE_WIDTH), true),
        BlameColumn::Code => (None, false),
    }
}

fn push_cell(parts: &mut Vec<String>, column: BlameColumn, value: String) {
    let (width, align_right) = column_width_and_alignment(column);
    if let Some(w) = width {
        let value = truncate(value, w);
        parts.push(pad(value, w, align_right));
    } else {
        parts.push(value);
    }
}

fn format_agent(meta: Option<&BlameMeta>, aliases: &AliasConfig) -> String {
    match meta {
        Some(m) => {
            let mut agent = apply_alias(&m.agent_tool, &aliases.agent_aliases);
            if let Some(v) = m.agent_version.as_deref() {
                if !v.is_empty() {
                    agent.push('@');
                    agent.push_str(v);
                }
            }
            agent
        }
        None => "-".to_string(),
    }
}

fn format_model(meta: Option<&BlameMeta>, aliases: &AliasConfig) -> String {
    match meta {
        Some(m) => apply_alias(&m.model, &aliases.model_aliases),
        None => "-".to_string(),
    }
}

fn format_timestamp(meta: Option<&BlameMeta>) -> String {
    match meta {
        Some(m) => m.timestamp.format("%Y-%m-%d %H:%M").to_string(),
        None => "-".to_string(),
    }
}

fn format_row(columns: &[BlameColumn], line: &LineBlame, aliases: &AliasConfig) -> String {
    let mut parts = Vec::new();
    for col in columns {
        match col {
            BlameColumn::Agent => {
                push_cell(&mut parts, *col, format_agent(line.meta.as_ref(), aliases))
            }
            BlameColumn::Model => {
                push_cell(&mut parts, *col, format_model(line.meta.as_ref(), aliases))
            }
            BlameColumn::Timestamp => {
                push_cell(&mut parts, *col, format_timestamp(line.meta.as_ref()))
            }
            BlameColumn::Line => push_cell(&mut parts, *col, line.line_no.to_string()),
            BlameColumn::Code => parts.push(format!("| {}", line.text)),
        }
    }
    parts.join(" ")
}

fn format_header(columns: &[BlameColumn]) -> String {
    let mut parts = Vec::new();
    for col in columns {
        match col {
            BlameColumn::Code => parts.push("| code".to_string()),
            _ => {
                let (width, align_right) = column_width_and_alignment(*col);
                if let Some(w) = width {
                    let value = truncate(column_label(*col).to_string(), w);
                    parts.push(pad(value, w, align_right));
                }
            }
        }
    }
    parts.join(" ")
}

fn format_block_label(
    columns: &[BlameColumn],
    block: &BlameBlock,
    aliases: &AliasConfig,
) -> String {
    match block.meta.as_ref() {
        None => format!("block {}-{}: (unknown)", block.start_line, block.end_line),
        Some(meta) => {
            let mut parts = Vec::new();
            for col in columns {
                match col {
                    BlameColumn::Agent => parts.push(format_agent(Some(meta), aliases)),
                    BlameColumn::Model => parts.push(format_model(Some(meta), aliases)),
                    BlameColumn::Timestamp => parts.push(format_timestamp(Some(meta))),
                    BlameColumn::Line | BlameColumn::Code => {}
                }
            }
            if parts.is_empty() {
                format!("block {}-{}", block.start_line, block.end_line)
            } else {
                format!(
                    "block {}-{}: {}",
                    block.start_line,
                    block.end_line,
                    parts.join(" ")
                )
            }
        }
    }
}

fn resolve_trace_dir(
    trace_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
) -> PathBuf {
    if let Some(td) = trace_dir {
        return td;
    }

    let resolved_target = target_dir
        .map(|p| p.canonicalize().unwrap_or(p))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let resolved_home = home_dir
        .map(|p| p.canonicalize().unwrap_or(p))
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));
    crate::paths::resolve_claude_trace_dir(&resolved_home, &resolved_target)
}

fn parse_line_range(spec: &str) -> Option<(usize, usize)> {
    let spec = spec.trim();
    if spec.is_empty() {
        return None;
    }
    let (a, b) = spec.split_once('-')?;
    let start = a.trim().parse::<usize>().ok()?;
    let end = b.trim().parse::<usize>().ok()?;
    if start == 0 || end == 0 || end < start {
        return None;
    }
    Some((start, end))
}

/// Get the creation date of a file from git history (date of first commit that introduced it).
/// Returns None if the file is not tracked by git or git is not available.
fn get_file_creation_date(file_path: &Path) -> Option<DateTime<Utc>> {
    // Try to get an absolute path and run git from the repo root
    let abs_path = file_path
        .canonicalize()
        .unwrap_or_else(|_| file_path.to_path_buf());
    let work_dir = abs_path.parent().unwrap_or(Path::new("."));

    // Get the first commit that introduced this file (oldest commit in the file's history)
    let output = Command::new("git")
        .args([
            "log",
            "--follow",
            "--format=%aI",
            "--reverse",
            "--diff-filter=A",
            "--",
        ])
        .arg(&abs_path)
        .current_dir(work_dir)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?;

    // Parse the ISO 8601 timestamp
    DateTime::parse_from_rfc3339(first_line)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn blame_command(config: BlameConfig) -> Result<()> {
    let trace_dir = resolve_trace_dir(config.trace_dir, config.target_dir, config.home_dir);
    if !trace_dir.exists() {
        eprintln!("Trace directory not found: {:?}", trace_dir);
        std::process::exit(1);
    }

    let file_path = PathBuf::from(&config.file);
    let file_path = if file_path.exists() {
        file_path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&config.file)
    };
    if !file_path.exists() {
        eprintln!("File not found: {:?}", file_path);
        std::process::exit(1);
    }

    let current_content = std::fs::read_to_string(&file_path)?;

    // Extract edits from both Claude and Codex traces, using a loose substring filter to reduce scan cost.
    let pattern = Path::new(&config.file)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&config.file)
        .to_string();
    let filter_config = FilterConfig {
        file_pattern: Some(pattern),
        ..Default::default()
    };
    let all_trace_dirs = crate::extractor::get_all_trace_dirs(&trace_dir);
    let trace_dir_refs: Vec<&Path> = all_trace_dirs.iter().map(|p| p.as_path()).collect();

    // Determine repo root from file path
    let repo_root = std::env::current_dir().ok();
    let edits_by_file = crate::extractor::extract_edit_history_from_dirs(
        &trace_dir_refs,
        &filter_config,
        repo_root.as_deref(),
    )?;

    // Pick the best-matching trace path.
    let rel = file_path
        .strip_prefix(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .ok()
        .and_then(|p| p.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| config.file.clone());

    let mut best: Option<(&String, &Vec<EditRecord>, usize)> = None;
    for (k, v) in &edits_by_file {
        let norm = crate::extractor::normalize_path(k, None);
        let score = if norm == rel || norm == config.file {
            0
        } else if norm.ends_with(&rel) || norm.ends_with(&config.file) || rel.ends_with(&norm) {
            1
        } else if Path::new(&norm)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| config.file.ends_with(n) || rel.ends_with(n))
            .unwrap_or(false)
        {
            2
        } else {
            continue;
        };
        if best.map(|b| score < b.2).unwrap_or(true) {
            best = Some((k, v, score));
        }
    }

    let edits = best.map(|b| b.1.clone()).unwrap_or_default();
    let blamed = compute_line_blame(&current_content, &edits)?;

    let (mut start_line, mut end_line) = (1usize, blamed.len().max(1));
    if let Some(spec) = config.lines.as_deref() {
        if let Some((s, e)) = parse_line_range(spec) {
            start_line = s;
            end_line = e;
        } else {
            eprintln!("Invalid --lines value (expected N-M): {}", spec);
            std::process::exit(2);
        }
    }

    let start_line = start_line.max(1);
    let end_line = end_line.min(blamed.len());
    if start_line > end_line {
        println!("(no lines to show)");
        return Ok(());
    }

    let blocks_vec = if config.blocks {
        group_blocks(&blamed)
    } else {
        Vec::new()
    };

    let columns =
        parse_column_spec(config.columns.as_deref(), config.show_agent).map_err(|e| anyhow!(e))?;
    let alias_config = AliasConfig {
        agent_aliases: config.agent_alias.into_iter().collect(),
        model_aliases: config.model_alias.into_iter().collect(),
    };

    // Display file metadata header (unless suppressed)
    if !config.no_header {
        if let Some(creation_date) = get_file_creation_date(&file_path) {
            println!("Created: {}", creation_date.format("%Y-%m-%d %H:%M"));
        }
        println!();
    }

    let header = format_header(&columns);
    println!("{}", header);
    println!("{}", "-".repeat(header.len().max(MIN_HEADER_LINE_LENGTH)));

    let mut block_iter = blocks_vec.iter().peekable();
    for line in &blamed[(start_line - 1)..end_line] {
        while let Some(b) = block_iter.peek() {
            if b.start_line == line.line_no {
                let label = format_block_label(&columns, b, &alias_config);
                println!("{}", label);
                break;
            } else {
                block_iter.next();
            }
        }

        println!("{}", format_row(&columns, line, &alias_config));
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn sample_meta() -> BlameMeta {
        BlameMeta {
            timestamp: Utc.with_ymd_and_hms(2025, 12, 1, 9, 0, 0).unwrap(),
            model: "claude-3-opus".to_string(),
            session_id: "s1".to_string(),
            agent_tool: "claude-code".to_string(),
            agent_version: Some("1.0.0".to_string()),
        }
    }

    #[test]
    fn test_parse_column_spec_defaults_follow_show_agent() {
        let cols = parse_column_spec(None, false).unwrap();
        assert_eq!(
            cols,
            vec![
                BlameColumn::Model,
                BlameColumn::Timestamp,
                BlameColumn::Line,
                BlameColumn::Code
            ]
        );

        let cols_with_agent = parse_column_spec(None, true).unwrap();
        assert!(cols_with_agent.starts_with(&[BlameColumn::Agent, BlameColumn::Model]));
    }

    #[test]
    fn test_parse_column_spec_rejects_unknown() {
        let err = parse_column_spec(Some("AMX"), false).unwrap_err();
        assert!(err.contains("invalid column specifier"));
    }

    #[test]
    fn test_format_row_applies_aliases() {
        let meta = sample_meta();
        let line = LineBlame {
            line_no: 42,
            text: "print('hi')".to_string(),
            meta: Some(meta),
        };
        let aliases = AliasConfig {
            agent_aliases: HashMap::from([("claude-code".to_string(), "CC".to_string())]),
            model_aliases: HashMap::from([("claude-3-opus".to_string(), "opus-4.5".to_string())]),
        };
        let cols = parse_column_spec(Some("AMLC"), false).unwrap();
        let row = format_row(&cols, &line, &aliases);
        assert!(row.contains("CC@1.0.0"));
        assert!(row.contains("opus-4.5"));
        assert!(row.contains("| print('hi')"));
    }

    #[test]
    fn test_format_block_label_respects_columns() {
        let meta = sample_meta();
        let block = BlameBlock {
            start_line: 1,
            end_line: 3,
            meta: Some(meta),
        };
        let aliases = AliasConfig::default();
        let cols = parse_column_spec(Some("MT"), false).unwrap();
        let label = format_block_label(&cols, &block, &aliases);
        assert!(label.contains("claude-3-opus"));
        assert!(label.contains("2025-12-01 09:00"));
        assert!(!label.contains("claude-code"));
    }
}

fn print_summary_table(histories: &HistoriesByFile) {
    println!("\n=== Summary ===");
    println!(
        "{:<50} | {:>5} | {:<20} | {:<20}",
        "File", "Edits", "First Edit", "Last Edit"
    );
    println!("{}", "-".repeat(105));

    let mut sorted_paths: Vec<_> = histories.keys().collect();
    sorted_paths.sort();

    for path in sorted_paths {
        let h = &histories[path];
        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        let name = crate::utils::safe_truncate(name, 48);
        let count = h.events.len();
        let first = h
            .first_edit()
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let last = h
            .last_edit()
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "N/A".to_string());
        println!("{:<50} | {:>5} | {:<20} | {:<20}", name, count, first, last);
    }

    println!();
}

fn print_yaml_previews(histories: &HistoriesByFile, limit: usize) -> Result<()> {
    let mut sorted_paths: Vec<_> = histories.keys().collect();
    sorted_paths.sort();

    for (i, path) in sorted_paths.iter().enumerate() {
        if i >= limit {
            println!(
                "\n... and {} more files (use --show-all to see all)",
                histories.len() - limit
            );
            break;
        }
        let history = &histories[*path];
        println!("\n=== YAML Preview: {} ===", path);
        let preview = preview_update(Path::new(path), history)?;
        println!("{}", preview);
    }

    Ok(())
}

fn load_output_config(config_file: Option<PathBuf>) -> Result<OutputConfig> {
    if let Some(cf) = config_file {
        if !cf.exists() {
            eprintln!("Config file not found: {:?}", cf);
            std::process::exit(1);
        }
        println!("Using config: {:?}", cf);
        return load_config(&cf);
    }

    if let Some(found_config) = find_config(None) {
        println!("Using config: {:?}", found_config);
        return load_config(&found_config);
    }

    Ok(get_default_config())
}

#[allow(clippy::too_many_arguments)]
fn build_histories(
    target: Option<String>,
    trace_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    config_file: Option<PathBuf>,
    initial_and_recent: bool,
    min_change_size: usize,
    file_pattern: String,
    skip_codex: bool,
) -> Result<(PathBuf, OutputConfig, HistoriesByFile)> {
    build_histories_verbose(
        target,
        trace_dir,
        target_dir,
        home_dir,
        config_file,
        initial_and_recent,
        min_change_size,
        file_pattern,
        0,
        skip_codex,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_histories_verbose(
    target: Option<String>,
    trace_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    config_file: Option<PathBuf>,
    initial_and_recent: bool,
    min_change_size: usize,
    file_pattern: String,
    verbose: u8,
    skip_codex: bool,
) -> Result<(PathBuf, OutputConfig, HistoriesByFile)> {
    let trace_dir = resolve_trace_dir(trace_dir, target_dir, home_dir);

    if !trace_dir.exists() {
        eprintln!("Trace directory not found: {:?}", trace_dir);
        std::process::exit(1);
    }

    println!("Scanning traces in: {:?}", trace_dir);

    if skip_codex {
        eprintln!("[*] Skipping Codex traces (--skip-codex)");
    }

    // Load output config
    let output_config = load_output_config(config_file)?;

    // Build filter config
    let filter_config = FilterConfig {
        initial_and_recent_only: initial_and_recent,
        min_change_size,
        since: None,
        until: None,
        file_pattern: if file_pattern.is_empty() {
            None
        } else {
            Some(file_pattern.clone())
        },
    };

    // Extract edit history from both Claude and Codex traces
    let mut all_trace_dirs = crate::extractor::get_all_trace_dirs(&trace_dir);

    // Filter out Codex traces if requested
    if skip_codex {
        all_trace_dirs.retain(|d| {
            !d.to_string_lossy().contains("codex") && !d.to_string_lossy().contains(".codex")
        });
    }

    let trace_dir_refs: Vec<&Path> = all_trace_dirs.iter().map(|p| p.as_path()).collect();
    let repo_root = std::env::current_dir().ok();

    if verbose >= 2 {
        eprintln!(
            "[*] Analyzing {} trace directories...",
            trace_dir_refs.len()
        );
    }

    let mut edits_by_file = if verbose >= 2 {
        crate::extractor::extract_edit_history_from_dirs_verbose(
            &trace_dir_refs,
            &filter_config,
            repo_root.as_deref(),
            verbose,
        )?
    } else {
        crate::extractor::extract_edit_history_from_dirs(
            &trace_dir_refs,
            &filter_config,
            repo_root.as_deref(),
        )?
    };

    if edits_by_file.is_empty() {
        if verbose >= 1 {
            eprintln!("[*] No edits found matching criteria.");
        } else {
            println!("No edits found matching criteria.");
        }
        return Ok((trace_dir, output_config, HistoriesByFile::new()));
    }

    // Apply filters
    edits_by_file = apply_filters(edits_by_file, &filter_config);

    if edits_by_file.is_empty() {
        println!("No edits remaining after filtering.");
        return Ok((trace_dir, output_config, HistoriesByFile::new()));
    }

    // Convert to file histories
    let mut histories = convert_to_file_histories(edits_by_file, None);

    // Filter to specific target if provided
    if let Some(target_str) = target {
        histories.retain(|k, _| k.contains(&target_str));
        if histories.is_empty() {
            println!("No history found for: {}", target_str);
            std::process::exit(1);
        }
    }

    // Warn if too many entries
    for (path, history) in &histories {
        if history.events.len() > 20 {
            println!(
                "Warning: {} has {} curation events. Consider using --initial-and-recent to reduce.",
                path,
                history.events.len()
            );
        }
    }

    Ok((trace_dir, output_config, histories))
}

fn print_output_plan(output_config: &OutputConfig, histories: &HistoriesByFile) {
    println!("\n=== Output Plan ===");
    println!("{:<50} | {:<10} | destination", "File", "Policy");
    println!("{}", "-".repeat(95));

    let mut sorted_paths: Vec<_> = histories.keys().collect();
    sorted_paths.sort();

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for path in sorted_paths {
        let rule = output_config.get_rule_for_file(path);
        let (policy, dest) = match rule {
            None => ("(none)".to_string(), "(no matching rule)".to_string()),
            Some(r) => match r.policy {
                OutputPolicy::Skip => ("skip".to_string(), "(skipped)".to_string()),
                OutputPolicy::Append => ("append".to_string(), "in-place".to_string()),
                OutputPolicy::Comment => ("comment".to_string(), "in-place".to_string()),
                OutputPolicy::Sidecar => {
                    let mut file_path = PathBuf::from(path);
                    if !file_path.exists() {
                        file_path = cwd.join(path);
                    }
                    let pattern = r
                        .sidecar_pattern
                        .as_deref()
                        .unwrap_or("{stem}.history.yaml");
                    let sidecar = resolve_sidecar_path(&file_path, pattern);
                    ("sidecar".to_string(), sidecar.to_string_lossy().to_string())
                }
            },
        };

        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        let name = crate::utils::safe_truncate(name, 48);
        println!("{:<50} | {:<10} | {}", name, policy, dest);
    }
    println!();
}

#[allow(clippy::too_many_arguments)]
fn report_command(
    target: Option<String>,
    trace_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    config_file: Option<PathBuf>,
    initial_and_recent: bool,
    min_change_size: usize,
    show_all: bool,
    file_pattern: String,
    verbose: u8,
    skip_codex: bool,
) -> Result<()> {
    if verbose > 0 {
        eprintln!("[*] Extracting edit history...");
    }
    let (_trace_dir, output_config, histories) = if verbose >= 2 {
        build_histories_verbose(
            target,
            trace_dir,
            target_dir,
            home_dir,
            config_file,
            initial_and_recent,
            min_change_size,
            file_pattern,
            verbose,
            skip_codex,
        )?
    } else {
        build_histories(
            target,
            trace_dir,
            target_dir,
            home_dir,
            config_file,
            initial_and_recent,
            min_change_size,
            file_pattern,
            skip_codex,
        )?
    };

    if histories.is_empty() {
        return Ok(());
    }

    print_summary_table(&histories);
    print_output_plan(&output_config, &histories);

    let limit = if show_all { histories.len() } else { 5 };
    print_yaml_previews(&histories, limit)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn annotate_command(
    target: Option<String>,
    trace_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    config_file: Option<PathBuf>,
    dry_run: bool,
    initial_and_recent: bool,
    min_change_size: usize,
    file_pattern: String,
) -> Result<()> {
    let (_trace_dir, output_config, histories) = build_histories(
        target,
        trace_dir,
        target_dir,
        home_dir,
        config_file,
        initial_and_recent,
        min_change_size,
        file_pattern,
        false,
    )?;

    if histories.is_empty() {
        return Ok(());
    }

    if dry_run {
        println!("\n[DRY RUN] No files will be modified.");
    } else {
        println!("\nApplying changes...");
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for (rel_path, history) in &histories {
        // Find the actual file
        let mut file_path = PathBuf::from(rel_path);
        if !file_path.exists() {
            file_path = cwd.join(rel_path);
        }
        if !file_path.exists() {
            println!("  Skipping (not found): {}", rel_path);
            continue;
        }

        // Get rule for this file
        let rule = match output_config.get_rule_for_file(rel_path) {
            Some(r) => r,
            None => {
                println!("  Skipping (no matching rule): {}", rel_path);
                continue;
            }
        };

        if rule.policy == OutputPolicy::Skip {
            println!("  Skipped (policy=skip): {}", rel_path);
            continue;
        }

        match apply_rule(&file_path, history, &rule, dry_run) {
            Ok((true, msg)) => println!("  {}", msg),
            Ok((false, msg)) => println!("  Failed: {}", msg),
            Err(e) => println!("  Error: {}", e),
        }
    }

    Ok(())
}

fn timeline_command(
    trace_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    file_pattern: String,
    verbose: u8,
    skip_codex: bool,
    limit: usize,
) -> Result<()> {
    let trace_dir = resolve_trace_dir(trace_dir, target_dir, home_dir);

    if verbose > 0 {
        eprintln!("[*] Collecting timeline events...");
    }

    if !trace_dir.exists() {
        eprintln!("Trace directory not found: {:?}", trace_dir);
        std::process::exit(1);
    }

    if verbose > 0 && skip_codex {
        eprintln!("[*] Skipping Codex traces (--skip-codex)");
    }

    let config = FilterConfig {
        file_pattern: if file_pattern.is_empty() {
            None
        } else {
            Some(file_pattern.clone())
        },
        ..Default::default()
    };

    let mut all_trace_dirs = crate::extractor::get_all_trace_dirs(&trace_dir);

    // Filter out Codex traces if requested
    if skip_codex {
        all_trace_dirs.retain(|d| {
            !d.to_string_lossy().contains("codex") && !d.to_string_lossy().contains(".codex")
        });
    }

    let trace_dir_refs: Vec<&Path> = all_trace_dirs.iter().map(|p| p.as_path()).collect();
    let repo_root = std::env::current_dir().ok();

    if verbose >= 2 {
        eprintln!(
            "[*] Analyzing {} trace directories...",
            trace_dir_refs.len()
        );
    }

    let edits_by_file = if verbose >= 2 {
        crate::extractor::extract_edit_history_from_dirs_verbose(
            &trace_dir_refs,
            &config,
            repo_root.as_deref(),
            verbose,
        )?
    } else {
        crate::extractor::extract_edit_history_from_dirs(
            &trace_dir_refs,
            &config,
            repo_root.as_deref(),
        )?
    };

    // Collect all edits into a flat list
    let mut all_edits: Vec<&EditRecord> = edits_by_file
        .values()
        .flat_map(|edits| edits.iter())
        .collect();

    // Filter out Codex/Copilot edits if requested (handles traces from Codex directories that weren't filtered earlier)
    if skip_codex {
        all_edits.retain(|edit| {
            !edit.agent_tool.to_lowercase().contains("copilot")
                && !edit.agent_tool.to_lowercase().contains("codex")
        });
    }

    if all_edits.is_empty() {
        println!("No edits found.");
        return Ok(());
    }

    // Sort by timestamp (most recent first)
    all_edits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Apply limit if specified
    let display_edits = if limit > 0 && all_edits.len() > limit {
        &all_edits[..limit]
    } else {
        &all_edits[..]
    };

    println!("\n=== Timeline of Actions ===");
    println!(
        "Showing {} most recent edit{}",
        display_edits.len(),
        if display_edits.len() == 1 { "" } else { "s" }
    );
    println!();
    println!(
        "{:<20} {:<10} {:<50} {:<25} {:<20}",
        "Timestamp", "Action", "File", "Model", "Agent"
    );
    println!("{}", "-".repeat(125));

    for edit in display_edits {
        let timestamp = edit.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let action = if edit.is_create { "CREATED" } else { "EDITED" };

        // Truncate file path if too long
        let file_display = if edit.file_path.len() > 48 {
            format!("...{}", &edit.file_path[edit.file_path.len() - 45..])
        } else {
            edit.file_path.clone()
        };

        // Format agent with version if available
        let agent_display = if let Some(version) = &edit.agent_version {
            format!("{}@{}", edit.agent_tool, version)
        } else {
            edit.agent_tool.clone()
        };

        println!(
            "{:<20} {:<10} {:<50} {:<25} {:<20}",
            timestamp, action, file_display, edit.model, agent_display
        );
    }

    if limit > 0 && all_edits.len() > limit {
        println!();
        println!(
            "... and {} more edit(s) (use -n 0 to show all or -n <N> to show more)",
            all_edits.len() - limit
        );
    }

    println!();
    println!("Total edits found: {}", all_edits.len());

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn stats_command(
    trace_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    file_pattern: String,
    verbose: u8,
    skip_codex: bool,
    no_cache: bool,
    rebuild_cache: bool,
) -> Result<()> {
    // Handle cache control flags
    if rebuild_cache {
        // Delete existing cache to force rebuild
        if let Ok(cwd) = std::env::current_dir() {
            let cache_path = cwd.join(".ai-blame.ddb");
            if cache_path.exists() {
                std::fs::remove_file(&cache_path)?;
                if verbose > 0 {
                    eprintln!("Deleted cache: {:?}", cache_path);
                }
            }
        }
    }

    if no_cache {
        // Disable cache for this run
        std::env::set_var("AI_BLAME_NO_CACHE", "1");
    }

    let trace_dir = resolve_trace_dir(trace_dir, target_dir, home_dir);

    if verbose > 0 {
        eprintln!("[*] Collecting trace files...");
    }

    if !trace_dir.exists() {
        eprintln!("Trace directory not found: {:?}", trace_dir);
        std::process::exit(1);
    }

    println!("Trace directory: {:?}", trace_dir);

    if skip_codex {
        eprintln!("[*] Skipping Codex traces (--skip-codex)");
    }

    // Count trace files (including nested directories like ~/.codex/sessions/)
    let mut jsonl_files = Vec::new();
    let _ = crate::extractor::collect_jsonl_files(&trace_dir, &mut jsonl_files);

    println!("Trace files: {}", jsonl_files.len());

    let agent_files: Vec<_> = jsonl_files
        .iter()
        .filter(|f| {
            f.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.starts_with("agent-"))
                .unwrap_or(false)
        })
        .collect();
    let session_files_count = jsonl_files.len() - agent_files.len();

    println!("  Session traces: {}", session_files_count);
    println!("  Agent traces: {}", agent_files.len());

    // Extract and summarize edits from both Claude and Codex traces
    let config = FilterConfig {
        file_pattern: if file_pattern.is_empty() {
            None
        } else {
            Some(file_pattern.clone())
        },
        ..Default::default()
    };
    let mut all_trace_dirs = crate::extractor::get_all_trace_dirs(&trace_dir);

    // Filter out Codex traces if requested
    if skip_codex {
        all_trace_dirs.retain(|d| {
            !d.to_string_lossy().contains("codex") && !d.to_string_lossy().contains(".codex")
        });
    }

    let trace_dir_refs: Vec<&Path> = all_trace_dirs.iter().map(|p| p.as_path()).collect();
    let repo_root = std::env::current_dir().ok();

    if verbose >= 2 {
        eprintln!(
            "[*] Analyzing {} trace directories...",
            trace_dir_refs.len()
        );
    }

    let edits_by_file = if verbose >= 2 {
        crate::extractor::extract_edit_history_from_dirs_verbose(
            &trace_dir_refs,
            &config,
            repo_root.as_deref(),
            verbose,
        )?
    } else {
        crate::extractor::extract_edit_history_from_dirs(
            &trace_dir_refs,
            &config,
            repo_root.as_deref(),
        )?
    };

    let total_edits: usize = edits_by_file.values().map(|v| v.len()).sum();
    let pattern_desc = if file_pattern.is_empty() {
        "(all files)".to_string()
    } else {
        format!("matching '{}'", file_pattern)
    };

    println!(
        "\nFiles with edits {}: {}",
        pattern_desc,
        edits_by_file.len()
    );
    println!("Total successful edits: {}", total_edits);

    Ok(())
}

fn parse_transcript_column_spec(spec: Option<&str>) -> Result<Vec<TranscriptColumn>, String> {
    let mut cols = Vec::new();
    if let Some(spec) = spec {
        for ch in spec.chars() {
            let col = match ch {
                'S' | 's' => Some(TranscriptColumn::Session),
                'A' | 'a' => Some(TranscriptColumn::Agent),
                'T' | 't' => Some(TranscriptColumn::Timestamp),
                'M' | 'm' => Some(TranscriptColumn::Messages),
                'F' | 'f' => Some(TranscriptColumn::Files),
                'O' | 'o' => Some(TranscriptColumn::Models),
                'L' | 'l' => Some(TranscriptColumn::LastMessage),
                _ if ch.is_whitespace() => None,
                _ => {
                    return Err(format!(
                        "invalid column specifier '{}'; allowed: S,A,T,M,F,O,L",
                        ch
                    ))
                }
            };
            if let Some(c) = col {
                cols.push(c);
            }
        }
    } else {
        // Default columns
        cols = vec![
            TranscriptColumn::Session,
            TranscriptColumn::Agent,
            TranscriptColumn::Timestamp,
            TranscriptColumn::Messages,
        ];
    }

    if cols.is_empty() {
        return Err("column list cannot be empty".to_string());
    }
    Ok(cols)
}

fn transcript_list_command(
    trace_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    limit: usize,
    format: TranscriptFormat,
    columns: Option<String>,
) -> Result<()> {
    let trace_dir = resolve_trace_dir(trace_dir, target_dir, home_dir);

    if !trace_dir.exists() {
        eprintln!("Trace directory not found: {:?}", trace_dir);
        std::process::exit(1);
    }

    let transcripts = crate::transcript::parse_transcripts_from_directory(&trace_dir)?;

    if transcripts.is_empty() {
        println!("No transcripts found in {:?}", trace_dir);
        return Ok(());
    }

    let display_transcripts = if limit > 0 && transcripts.len() > limit {
        &transcripts[..limit]
    } else {
        &transcripts[..]
    };

    match format {
        TranscriptFormat::Table => {
            let cols = parse_transcript_column_spec(columns.as_deref()).map_err(|e| anyhow!(e))?;
            print_transcript_table(display_transcripts, &cols, &transcripts, limit);
        }
        TranscriptFormat::Json => {
            let summaries: Vec<_> = display_transcripts.iter().map(|t| t.summary()).collect();
            println!("{}", serde_json::to_string_pretty(&summaries)?);
        }
    }

    Ok(())
}

fn print_transcript_table(
    display_transcripts: &[crate::transcript::Transcript],
    columns: &[TranscriptColumn],
    all_transcripts: &[crate::transcript::Transcript],
    limit: usize,
) {
    println!("\n=== Transcripts ===");

    // Calculate column widths
    let mut widths: Vec<usize> = Vec::new();
    let mut headers: Vec<&str> = Vec::new();

    for col in columns {
        match col {
            TranscriptColumn::Session => {
                widths.push(40);
                headers.push("Session ID");
            }
            TranscriptColumn::Agent => {
                widths.push(15);
                headers.push("Agent");
            }
            TranscriptColumn::Timestamp => {
                widths.push(20);
                headers.push("Start Time");
            }
            TranscriptColumn::Messages => {
                widths.push(8);
                headers.push("Msgs");
            }
            TranscriptColumn::Files => {
                widths.push(8);
                headers.push("Files");
            }
            TranscriptColumn::Models => {
                widths.push(30);
                headers.push("Models");
            }
            TranscriptColumn::LastMessage => {
                widths.push(50);
                headers.push("Last Message");
            }
        }
    }

    // Print header
    let header_line = headers
        .iter()
        .zip(&widths)
        .map(|(h, w)| format!("{:<width$}", h, width = w))
        .collect::<Vec<_>>()
        .join(" ");
    println!("{}", header_line);

    let total_width = widths.iter().sum::<usize>() + (widths.len() - 1);
    println!("{}", "-".repeat(total_width));

    // Print rows
    for transcript in display_transcripts {
        let summary = transcript.summary();
        let row_values: Vec<String> = columns
            .iter()
            .map(|col| match col {
                TranscriptColumn::Session => {
                    crate::utils::safe_truncate(&summary.session_id, 38).to_string()
                }
                TranscriptColumn::Agent => {
                    crate::utils::safe_truncate(&summary.agent_tool, 13).to_string()
                }
                TranscriptColumn::Timestamp => {
                    summary.start_time.format("%Y-%m-%d %H:%M").to_string()
                }
                TranscriptColumn::Messages => format!("{}", summary.message_count),
                TranscriptColumn::Files => format!("{}", summary.files_touched),
                TranscriptColumn::Models => {
                    if summary.all_models.is_empty() {
                        "(unknown)".to_string()
                    } else {
                        summary.all_models.join(", ")
                    }
                }
                TranscriptColumn::LastMessage => summary
                    .last_message_preview
                    .as_ref()
                    .map(|m| crate::utils::safe_truncate(m, 48).to_string())
                    .unwrap_or_else(|| "(no messages)".to_string()),
            })
            .collect();

        let row_line = row_values
            .iter()
            .zip(&widths)
            .enumerate()
            .map(|(idx, (v, w))| {
                if matches!(
                    columns.get(idx),
                    Some(TranscriptColumn::Messages) | Some(TranscriptColumn::Files)
                ) {
                    format!("{:>width$}", v, width = w)
                } else {
                    format!("{:<width$}", v, width = w)
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        println!("{}", row_line);
    }

    if limit > 0 && all_transcripts.len() > limit {
        println!(
            "\n... and {} more transcript(s) (use -n 0 to show all)",
            all_transcripts.len() - limit
        );
    }

    println!("\nTotal transcripts: {}", all_transcripts.len());
}

/// Display options for transcript viewing
struct TranscriptDisplayOptions {
    format: TranscriptViewFormat,
    full: bool,
    show_thinking: bool,
    show_tools: bool,
}

/// Directory resolution context
struct DirectoryContext {
    trace_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
}

fn transcript_view_command(
    session: String,
    dirs: DirectoryContext,
    options: TranscriptDisplayOptions,
) -> Result<()> {
    // Check if session is a file path
    let transcript = if PathBuf::from(&session).exists() {
        crate::transcript::parse_transcript(&PathBuf::from(&session))?
    } else {
        // Search for transcript by session ID
        let trace_dir = resolve_trace_dir(dirs.trace_dir, dirs.target_dir, dirs.home_dir);
        if !trace_dir.exists() {
            eprintln!("Trace directory not found: {:?}", trace_dir);
            std::process::exit(1);
        }

        let transcripts = crate::transcript::parse_transcripts_from_directory(&trace_dir)?;
        transcripts
            .into_iter()
            .find(|t| {
                t.meta.session_id.contains(&session)
                    || t.meta
                        .slug
                        .as_ref()
                        .map(|s| s.contains(&session))
                        .unwrap_or(false)
                    || t.meta
                        .source_file
                        .as_ref()
                        .map(|s| s.contains(&session))
                        .unwrap_or(false)
            })
            .ok_or_else(|| anyhow!("Transcript not found: {}", session))?
    };

    match options.format {
        TranscriptViewFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&transcript)?);
        }
        TranscriptViewFormat::Markdown => {
            print_transcript_markdown(
                &transcript,
                options.full,
                options.show_thinking,
                options.show_tools,
            );
        }
        TranscriptViewFormat::Text => {
            print_transcript_text(
                &transcript,
                options.full,
                options.show_thinking,
                options.show_tools,
            );
        }
    }

    Ok(())
}

fn print_transcript_text(
    transcript: &crate::transcript::Transcript,
    full: bool,
    show_thinking: bool,
    show_tools: bool,
) {
    use crate::transcript::{ContentBlock, Role};

    // Header
    println!("\n{}", "=".repeat(80));
    println!("Session: {}", transcript.meta.session_id);
    if let Some(slug) = &transcript.meta.slug {
        println!("Slug: {}", slug);
    }
    println!("Agent: {}", transcript.meta.agent_tool);
    if let Some(version) = &transcript.meta.agent_version {
        println!("Version: {}", version);
    }
    if let Some(cwd) = &transcript.meta.cwd {
        println!("Working Directory: {}", cwd);
    }
    if let Some(source_file) = &transcript.meta.source_file {
        println!("Trace File: {}", source_file);
    }
    println!(
        "Start: {}",
        transcript.meta.start_time.format("%Y-%m-%d %H:%M:%S")
    );
    if let Some(end) = transcript.meta.end_time {
        println!("End: {}", end.format("%Y-%m-%d %H:%M:%S"));
    }
    println!("Messages: {}", transcript.stats.message_count);
    println!("Files Touched: {}", transcript.stats.files_touched);
    println!("{}", "=".repeat(80));

    // Messages
    for message in &transcript.messages {
        let role_str = match message.role {
            Role::User => "[USER]",
            Role::Assistant => "[ASSISTANT]",
            Role::System => "[SYSTEM]",
        };

        let time_str = message.timestamp.format("%H:%M:%S").to_string();
        let model_str = message
            .model
            .as_ref()
            .map(|m| format!(" ({})", m))
            .unwrap_or_default();

        println!(
            "\n{} {} {}{}",
            "-".repeat(20),
            role_str,
            time_str,
            model_str
        );

        for content in &message.content {
            match content {
                ContentBlock::Text { text } => {
                    let display_text = if full || text.chars().count() <= 500 {
                        text.clone()
                    } else {
                        format!(
                            "{}\n[truncated, use --full to see all]",
                            crate::utils::safe_truncate_with_suffix(text, 500, "...")
                        )
                    };
                    println!("{}", display_text);
                }
                ContentBlock::Thinking { thinking } => {
                    if show_thinking {
                        println!("\n<thinking>");
                        let display = if full || thinking.chars().count() <= 300 {
                            thinking.clone()
                        } else {
                            crate::utils::safe_truncate(thinking, 300)
                        };
                        println!("{}", display);
                        println!("</thinking>");
                    } else {
                        println!("[thinking block - use --show-thinking to view]");
                    }
                }
                ContentBlock::ToolUse { id, name, input } => {
                    if show_tools {
                        println!("\n[Tool Use: {} ({})]", name, id);
                        let input_str = serde_json::to_string_pretty(input).unwrap_or_default();
                        let display = if full || input_str.chars().count() <= 200 {
                            input_str
                        } else {
                            crate::utils::safe_truncate(&input_str, 200)
                        };
                        println!("{}", display);
                    } else {
                        println!("[Tool: {}]", name);
                    }
                }
                ContentBlock::ToolResult {
                    content, is_error, ..
                } => {
                    if show_tools {
                        let prefix = if *is_error { "[Error]" } else { "[Result]" };
                        let display = if full || content.chars().count() <= 200 {
                            content.clone()
                        } else {
                            crate::utils::safe_truncate(content, 200)
                        };
                        println!("{} {}", prefix, display);
                    }
                }
                ContentBlock::FileOperation {
                    operation,
                    file_path,
                    ..
                } => {
                    println!("[File {}: {}]", operation, file_path);
                }
                ContentBlock::Command {
                    command,
                    output,
                    exit_code,
                } => {
                    println!("[Command: {}]", command);
                    if show_tools {
                        if let Some(out) = output {
                            let display = if full || out.chars().count() <= 200 {
                                out.clone()
                            } else {
                                crate::utils::safe_truncate(out, 200)
                            };
                            println!("Output: {}", display);
                        }
                        if let Some(code) = exit_code {
                            println!("Exit code: {}", code);
                        }
                    }
                }
                ContentBlock::Code { code, language } => {
                    let lang = language.as_deref().unwrap_or("text");
                    println!("```{}", lang);
                    let display = if full || code.chars().count() <= 500 {
                        code.clone()
                    } else {
                        crate::utils::safe_truncate(code, 500)
                    };
                    println!("{}", display);
                    println!("```");
                }
            }
        }
    }

    println!("\n{}", "=".repeat(80));
}

fn print_transcript_markdown(
    transcript: &crate::transcript::Transcript,
    full: bool,
    show_thinking: bool,
    show_tools: bool,
) {
    use crate::transcript::{ContentBlock, Role};

    // Header
    println!("# Transcript: {}", transcript.meta.session_id);
    println!();
    if let Some(slug) = &transcript.meta.slug {
        println!("**Slug:** {}", slug);
    }
    println!("**Agent:** {}", transcript.meta.agent_tool);
    if let Some(version) = &transcript.meta.agent_version {
        println!("**Version:** {}", version);
    }
    if let Some(cwd) = &transcript.meta.cwd {
        println!("**Working Directory:** `{}`", cwd);
    }
    if let Some(source_file) = &transcript.meta.source_file {
        println!("**Trace File:** `{}`", source_file);
    }
    println!(
        "**Start:** {}",
        transcript.meta.start_time.format("%Y-%m-%d %H:%M:%S")
    );
    if let Some(end) = transcript.meta.end_time {
        println!("**End:** {}", end.format("%Y-%m-%d %H:%M:%S"));
    }
    println!("**Messages:** {}", transcript.stats.message_count);
    println!("**Files Touched:** {}", transcript.stats.files_touched);
    println!();
    println!("---");
    println!();

    // Messages
    for message in &transcript.messages {
        let role_str = match message.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::System => "System",
        };

        let time_str = message.timestamp.format("%H:%M:%S").to_string();
        let model_str = message
            .model
            .as_ref()
            .map(|m| format!(" _{}_", m))
            .unwrap_or_default();

        println!("## {} ({}){}", role_str, time_str, model_str);
        println!();

        for content in &message.content {
            match content {
                ContentBlock::Text { text } => {
                    let display_text = if full || text.chars().count() <= 500 {
                        text.clone()
                    } else {
                        format!(
                            "{}\n\n*[truncated, use --full to see all]*",
                            crate::utils::safe_truncate_with_suffix(text, 500, "...")
                        )
                    };
                    println!("{}", display_text);
                    println!();
                }
                ContentBlock::Thinking { thinking } => {
                    if show_thinking {
                        println!("<details>");
                        println!("<summary>Thinking</summary>");
                        println!();
                        let display = if full || thinking.chars().count() <= 300 {
                            thinking.clone()
                        } else {
                            crate::utils::safe_truncate(thinking, 300)
                        };
                        println!("{}", display);
                        println!();
                        println!("</details>");
                        println!();
                    } else {
                        println!("*[thinking block - use --show-thinking to view]*");
                        println!();
                    }
                }
                ContentBlock::ToolUse { id, name, input } => {
                    if show_tools {
                        println!("**Tool Use:** `{}` (`{}`)", name, id);
                        println!();
                        println!("```json");
                        let input_str = serde_json::to_string_pretty(input).unwrap_or_default();
                        let display = if full || input_str.chars().count() <= 200 {
                            input_str
                        } else {
                            crate::utils::safe_truncate(&input_str, 200)
                        };
                        println!("{}", display);
                        println!("```");
                        println!();
                    } else {
                        println!("> **Tool:** `{}`", name);
                        println!();
                    }
                }
                ContentBlock::ToolResult {
                    content, is_error, ..
                } => {
                    if show_tools {
                        let prefix = if *is_error { "Error" } else { "Result" };
                        println!("**{}:**", prefix);
                        println!();
                        let display = if full || content.chars().count() <= 200 {
                            content.clone()
                        } else {
                            crate::utils::safe_truncate(content, 200)
                        };
                        println!("```");
                        println!("{}", display);
                        println!("```");
                        println!();
                    }
                }
                ContentBlock::FileOperation {
                    operation,
                    file_path,
                    ..
                } => {
                    println!("> **File {}:** `{}`", operation, file_path);
                    println!();
                }
                ContentBlock::Command {
                    command,
                    output,
                    exit_code,
                } => {
                    println!("**Command:** `{}`", command);
                    if show_tools {
                        if let Some(out) = output {
                            let display = if full || out.chars().count() <= 200 {
                                out.clone()
                            } else {
                                crate::utils::safe_truncate(out, 200)
                            };
                            println!();
                            println!("```");
                            println!("{}", display);
                            println!("```");
                        }
                        if let Some(code) = exit_code {
                            println!("*Exit code: {}*", code);
                        }
                    }
                    println!();
                }
                ContentBlock::Code { code, language } => {
                    let lang = language.as_deref().unwrap_or("");
                    println!("```{}", lang);
                    let display = if full || code.chars().count() <= 500 {
                        code.clone()
                    } else {
                        crate::utils::safe_truncate(code, 500)
                    };
                    println!("{}", display);
                    println!("```");
                    println!();
                }
            }
        }

        println!("---");
        println!();
    }
}

/// Search criteria for transcript search command
struct TranscriptSearchConfig {
    query: String,
    use_regex: bool,
    case_sensitive: bool,
    session_id_pattern: Option<String>,
    agent_tool: Option<String>,
    model: Option<String>,
    since: Option<String>,
    until: Option<String>,
    limit: usize,
    format: TranscriptFormat,
}

fn transcript_search_command(dirs: DirectoryContext, config: TranscriptSearchConfig) -> Result<()> {
    use chrono::NaiveDate;

    let trace_dir = resolve_trace_dir(dirs.trace_dir, dirs.target_dir, dirs.home_dir);

    if !trace_dir.exists() {
        eprintln!("Trace directory not found: {:?}", trace_dir);
        std::process::exit(1);
    }

    // Parse date filters
    let since = if let Some(ref date_str) = config.since {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| anyhow!("Invalid --since date '{}': {}", date_str, e))?;
        Some(
            date.and_hms_opt(0, 0, 0)
                .ok_or_else(|| anyhow!("Invalid date"))?
                .and_utc(),
        )
    } else {
        None
    };

    let until = if let Some(ref date_str) = config.until {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| anyhow!("Invalid --until date '{}': {}", date_str, e))?;
        Some(
            date.and_hms_opt(23, 59, 59)
                .ok_or_else(|| anyhow!("Invalid date"))?
                .and_utc(),
        )
    } else {
        None
    };

    let criteria = crate::transcript::TranscriptSearchCriteria {
        query: Some(config.query.clone()),
        use_regex: config.use_regex,
        case_sensitive: config.case_sensitive,
        session_id_pattern: config.session_id_pattern,
        agent_tool: config.agent_tool,
        model: config.model,
        since,
        until,
    };

    let result = crate::transcript::search_transcripts(&trace_dir, &criteria, config.limit)?;

    match config.format {
        TranscriptFormat::Table => {
            print_search_results_table(&result, &config.query);
        }
        TranscriptFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}

fn print_search_results_table(result: &crate::transcript::SearchResult, query: &str) {
    if result.matching_transcripts.is_empty() {
        println!("\nNo transcripts found matching '{}'", query);
        return;
    }

    println!("\n=== Search Results for '{}' ===", query);
    println!(
        "Found {} matching transcript{}",
        result.total_matches,
        if result.total_matches == 1 { "" } else { "s" }
    );
    println!();

    // Print results with snippets
    for search_match in &result.matching_transcripts {
        let summary = &search_match.transcript;
        let session = crate::utils::safe_truncate(&summary.session_id, 36);
        let agent = &summary.agent_tool;
        let time = summary.start_time.format("%Y-%m-%d %H:%M").to_string();

        println!(
            "{} | {} | {} | {} msgs",
            session, agent, time, summary.message_count
        );

        // Print matching snippets
        for snippet in &search_match.matches {
            let snippet_text = snippet.snippet.replace('\n', " ");
            let snippet_text = crate::utils::safe_truncate(&snippet_text, 100);
            println!("  └─ [{}] {}", snippet.block_type, snippet_text);
        }
        println!();
    }

    if result.matching_transcripts.len() < result.total_matches {
        println!(
            "... showing {} of {} matches (use -n 0 for all)",
            result.matching_transcripts.len(),
            result.total_matches
        );
        println!();
    }

    println!("Use 'ai-blame transcript view <session-id>' to view a transcript");
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { dir, flavor, force } => {
            let dir = dir
                .map(|p| p.canonicalize().unwrap_or(p))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

            let flavor = match flavor {
                InitFlavor::Sidecar => SeedFlavor::Sidecar,
                InitFlavor::InPlace => SeedFlavor::InPlace,
            };

            let path = write_seed_config(&dir, flavor, force)?;
            println!("Wrote {:?}", path);
            println!();
            println!("Next steps:");
            println!("  - Preview (no writes): ai-blame annotate --dry-run");
            println!("  - Apply (writes files): ai-blame annotate");
            Ok(())
        }
        Commands::Report {
            target,
            trace_dir,
            dir,
            home,
            config,
            initial_and_recent,
            min_change_size,
            show_all,
            pattern,
            verbose,
            skip_codex,
            only_claude,
            no_cache,
            rebuild_cache,
        } => {
            if rebuild_cache {
                if let Ok(cwd) = std::env::current_dir() {
                    let _ = std::fs::remove_file(cwd.join(".ai-blame.ddb"));
                }
            }
            if no_cache {
                std::env::set_var("AI_BLAME_NO_CACHE", "1");
            }
            report_command(
                target,
                trace_dir,
                dir,
                home,
                config,
                initial_and_recent,
                min_change_size,
                show_all,
                pattern,
                verbose,
                skip_codex || only_claude,
            )
        }
        Commands::Annotate {
            target,
            trace_dir,
            dir,
            home,
            config,
            dry_run,
            initial_and_recent,
            min_change_size,
            pattern,
            no_cache,
            rebuild_cache,
        } => {
            if rebuild_cache {
                if let Ok(cwd) = std::env::current_dir() {
                    let _ = std::fs::remove_file(cwd.join(".ai-blame.ddb"));
                }
            }
            if no_cache {
                std::env::set_var("AI_BLAME_NO_CACHE", "1");
            }
            annotate_command(
                target,
                trace_dir,
                dir,
                home,
                config,
                dry_run,
                initial_and_recent,
                min_change_size,
                pattern,
            )
        }
        Commands::Stats {
            trace_dir,
            dir,
            home,
            pattern,
            verbose,
            skip_codex,
            only_claude,
            no_cache,
            rebuild_cache,
        } => stats_command(
            trace_dir,
            dir,
            home,
            pattern,
            verbose,
            skip_codex || only_claude,
            no_cache,
            rebuild_cache,
        ),
        Commands::Blame {
            file,
            trace_dir,
            dir,
            home,
            lines,
            blocks,
            show_agent,
            columns,
            agent_alias,
            model_alias,
            no_cache,
            rebuild_cache,
            no_header,
        } => {
            if rebuild_cache {
                if let Ok(cwd) = std::env::current_dir() {
                    let _ = std::fs::remove_file(cwd.join(".ai-blame.ddb"));
                }
            }
            if no_cache {
                std::env::remove_var("AI_BLAME_ENABLE_CACHE");
            }
            blame_command(BlameConfig {
                file,
                trace_dir,
                target_dir: dir,
                home_dir: home,
                lines,
                blocks,
                show_agent,
                columns,
                agent_alias,
                model_alias,
                no_header,
            })
        }
        Commands::Timeline {
            trace_dir,
            dir,
            home,
            pattern,
            verbose,
            skip_codex,
            only_claude,
            limit,
        } => timeline_command(
            trace_dir,
            dir,
            home,
            pattern,
            verbose,
            skip_codex || only_claude,
            limit,
        ),
        Commands::Transcript { action } => match action {
            TranscriptAction::List {
                trace_dir,
                dir,
                home,
                limit,
                format,
                columns,
            } => transcript_list_command(trace_dir, dir, home, limit, format, columns),
            TranscriptAction::View {
                session,
                trace_dir,
                dir,
                home,
                format,
                full,
                show_thinking,
                show_tools,
            } => transcript_view_command(
                session,
                DirectoryContext {
                    trace_dir,
                    target_dir: dir,
                    home_dir: home,
                },
                TranscriptDisplayOptions {
                    format,
                    full,
                    show_thinking,
                    show_tools,
                },
            ),
            TranscriptAction::Search {
                query,
                trace_dir,
                dir,
                home,
                regex,
                case_sensitive,
                session,
                agent,
                model,
                since,
                until,
                limit,
                format,
            } => transcript_search_command(
                DirectoryContext {
                    trace_dir,
                    target_dir: dir,
                    home_dir: home,
                },
                TranscriptSearchConfig {
                    query,
                    use_regex: regex,
                    case_sensitive,
                    session_id_pattern: session,
                    agent_tool: agent,
                    model,
                    since,
                    until,
                    limit,
                    format,
                },
            ),
        },
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "ai-blame", &mut std::io::stdout());
            Ok(())
        }
    }
}
