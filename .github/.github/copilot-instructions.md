# Project-Specific Instructions for ai-blame

## Git Workflow

**IMPORTANT: Never commit directly to `main`**

- Always work on feature branches
- Create a Pull Request for review before merging to main
- This ensures code review and maintains a clean main branch history
- When Claude Code needs to make changes, always create a new branch first

### Example workflow:
```bash
git checkout -b feature/your-feature
# make changes
git commit -m "description"
git push -u origin feature/your-feature
# then create a PR on GitHub
```

---

# Repository Guidelines

## Project Structure & Module Organization
- `src/` holds the Rust crate code. Key modules: `src/main.rs` (binary entry), `src/lib.rs` (library API), `src/cli.rs` (CLI parsing), `src/config.rs` (config loading), `src/extractor.rs` (trace parsing), `src/updater.rs` (file updates), `src/models.rs` (data types).
- `tests/` contains integration-style tests like `tests/config_test.rs` and `tests/models_test.rs`.
- `Cargo.toml` and `Cargo.lock` define dependencies and build metadata.

## Build, Test, and Development Commands

### Using `just` for common tasks (recommended):
```bash
just test    # Run all checks + tests (format, build, lint, test)
just check   # Only run checks without tests (format, build, lint)
just build   # Build debug + release
just all     # Everything (check, test, build)
```

### Manual cargo commands:
- `cargo build` — debug build for local iteration.
- `cargo build --release` — optimized release binary in `target/release/ai-blame`.
- `cargo run -- report` — run the CLI locally (add `RUST_LOG=debug` for verbose output).
- `cargo test` — run the full test suite.
- `cargo fmt` and `cargo clippy` — format and lint; treat warnings as errors via `cargo clippy -- -D warnings`.

### Complete Test Workflow:
Before opening a PR, always run:
```bash
just test
```

This runs the full check suite in order:
1. **Format check:** `cargo fmt --all -- --check` — ensures code formatting is correct
2. **Build check:** `RUSTFLAGS="-D warnings" cargo build --workspace` — builds with warnings treated as errors
3. **Lint check:** `cargo clippy --workspace --all-targets -- -D warnings` — runs clippy with all warnings as errors
4. **Test execution:** `cargo test --workspace` — runs all tests

All must pass before submitting a PR. Fix issues in this order:
- Format: Run `cargo fmt --all` to auto-fix
- Build warnings: Review error messages and fix code
- Clippy warnings: Run clippy and fix suggestions (use `#[allow(...)]` only when justified)
- Test failures: Debug and fix the underlying issue

## Coding Style & Naming Conventions
- Use standard Rust formatting enforced by `cargo fmt` (4-space indentation, rustfmt defaults).
- Naming: `snake_case` for functions/modules, `CamelCase` for types and enums, `SCREAMING_SNAKE_CASE` for constants.
- Keep modules focused on a single responsibility; prefer small, testable helpers in `src/`.

---

## Rust Best Practices

This section documents modern Rust idioms and project-specific conventions. Following these ensures consistent, maintainable code.

### Error Handling: Avoid Panics in Production Code

**Rule:** Do NOT use `.unwrap()` or `.expect()` in non-test code. Handle errors gracefully.

```rust
// ❌ Bad: Can panic in production
let value = some_option.unwrap();
let parsed: i64 = string.parse().unwrap();

// ✅ Good: Return errors or provide defaults
let value = some_option.ok_or_else(|| anyhow::anyhow!("Missing value"))?;
let parsed: i64 = string.parse().map_err(|e| anyhow::anyhow!("Parse failed: {}", e))?;

// ✅ Also acceptable: Provide default when appropriate
let parsed: i64 = string.parse().unwrap_or(0);
```

**Exceptions:**
- **Tests:** `.unwrap()` and `.expect()` are fine in `#[test]` functions
- **Static regexes:** Using `.expect("regex must compile")` for compile-time-known patterns is acceptable:
  ```rust
  static RE: OnceLock<Regex> = OnceLock::new();
  let re = RE.get_or_init(|| Regex::new(r"pattern").expect("static regex must compile"));
  ```
- **Mutex locks:** Prefer `.lock().unwrap()` with a comment explaining why poison is not expected, or use `.lock().map_err(|e| ...)?` if poisoning is possible

**Current violations to fix:**
- `src/cache/mod.rs:181,312` — timestamp `.parse().unwrap()` from database
- `src/parsers/codex.rs:279,286` — `repo_root.unwrap()` in closure

### Module Imports: Prefer `crate::` Over `super::`

**Rule:** Use `crate::` for absolute imports within the crate. Avoid `super::` except in test modules.

```rust
// ❌ Bad: Relative imports are harder to follow
use super::{ParserInfo, TraceParser};
use super::common::extract_model_from_record;

// ✅ Good: Absolute crate paths are explicit
use crate::parsers::{ParserInfo, TraceParser};
use crate::parsers::common::extract_model_from_record;

// ✅ Acceptable in test modules only
#[cfg(test)]
mod tests {
    use super::*;  // OK in tests
}
```

**Rationale:** Absolute paths make refactoring safer and code navigation clearer. See [Rust API Guidelines C-REEXPORT](https://rust-lang.github.io/api-guidelines/interoperability.html#c-reexport).

**Current violations:**
- `src/parsers/codex.rs:1` — `use super::{ParserInfo, TraceParser};`
- `src/parsers/claude.rs:1` — `use super::{ParserInfo, TraceParser};`
- Multiple `super::common::*` and `super::codex::*` calls in parsers

### Re-exports: Use `pub use` Sparingly

**Rule:** Only use `pub use` to re-export types that downstream consumers need, avoiding the need for them to depend on internal implementation crates.

```rust
// ✅ Good: Re-export public API from submodule
pub mod types;
pub use types::{FileMetadata, StalenessReport};  // Users import from cache:: directly

// ❌ Bad: Re-exporting everything or internal types
pub use internal_helpers::*;
```

**Current state:** The existing `pub use` in `src/cache/mod.rs` is correct — it re-exports `FileMetadata` and `StalenessReport` so callers can use `cache::FileMetadata` instead of `cache::types::FileMetadata`.

### Global State: Prefer Explicit Context

**Rule:** Avoid global mutable state via `lazy_static!`, `OnceCell`, `static mut`, etc. Instead, pass explicit context structs.

```rust
// ❌ Bad: Hidden global state
lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::default());
}

// ✅ Good: Explicit context passed through
struct AppContext {
    config: Config,
    cache: CacheManager,
}

fn process(ctx: &AppContext, input: &str) -> Result<Output> { ... }
```

**Acceptable exceptions:**
- **Lazy-compiled regexes:** `OnceLock<Regex>` for expensive compile-once patterns is idiomatic:
  ```rust
  static RE: OnceLock<Regex> = OnceLock::new();
  let re = RE.get_or_init(|| Regex::new(r"...").expect("static regex"));
  ```
  This is read-only after initialization and avoids recompiling on every call.

**Current state:** This codebase correctly uses `OnceLock<Regex>` only for regex caching (`src/blame.rs:45`, `src/updater.rs:254`). No problematic global mutable state exists.

### Additional Rust Idioms

**Use `?` for error propagation:**
```rust
// ✅ Good
let content = fs::read_to_string(path)?;

// ❌ Avoid unless you need custom logic
let content = match fs::read_to_string(path) {
    Ok(c) => c,
    Err(e) => return Err(e.into()),
};
```

**Use `anyhow::Context` for error messages:**
```rust
use anyhow::Context;

let content = fs::read_to_string(path)
    .with_context(|| format!("Failed to read config from {:?}", path))?;
```

**Prefer iterators over loops when transforming data:**
```rust
// ✅ Good
let names: Vec<String> = items.iter().map(|i| i.name.clone()).collect();

// ❌ Less idiomatic
let mut names = Vec::new();
for item in &items {
    names.push(item.name.clone());
}
```

**Use `Default` trait for structs with sensible defaults:**
```rust
#[derive(Default)]
pub struct FilterConfig {
    pub min_size: Option<usize>,
    pub max_size: Option<usize>,
    pub pattern: Option<String>,
}

// Usage
let config = FilterConfig { min_size: Some(100), ..Default::default() };
```

### References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — Official guidelines for Rust library design
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html) — The Rust Book on Result and error handling
- [Anyhow Crate](https://docs.rs/anyhow/latest/anyhow/) — Idiomatic error handling for applications
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/) — All available clippy lints with explanations

## Testing Guidelines
- Tests live in `tests/` and use Rust's built-in test framework.
- Follow the existing pattern: file names like `*_test.rs` and function names like `test_*`.
- Add tests for new functionality or bug fixes; run `cargo test` before opening a PR.

## Commit & Pull Request Guidelines
- Commit messages are short, imperative, and sentence-case (e.g., `Add version flag support to CLI`).
- Keep PRs focused on a single change; include a clear description and update `README.md` for user-facing changes.
- **Always run `just test` before opening a PR** — all checks and tests must pass with no warnings or failures.
- Include your reasoning in commit messages when using `#[allow(...)]` to suppress clippy warnings.
- Test caches (`.ai-blame.ddb`) are git-ignored; tests create them automatically as needed.

## Configuration & Usage Notes
- The tool reads `.ai-blame.yaml` from the project root when present; document new config options in `README.md`.
- Default behavior is dry-run; use `--apply` intentionally when testing changes.
- **DuckDB Caching**: The system uses DuckDB to cache parsed trace files per directory:
  - Cache files: `.ai-blame.ddb` created in each trace directory (git-ignored)
  - Claude traces: All-or-nothing invalidation (full directory re-parse when any file changes)
  - Codex traces: Per-file invalidation (only changed files are re-parsed)
  - Use `--no-cache` or `--rebuild-cache` flags to control caching behavior
  - See `docs/how-to/performance-and-caching.md` for user documentation

## UI Development with Playwright

### Overview
The `ui/` directory contains a static HTML prototype that runs in the Tauri desktop app. Playwright MCP is used for automated browser testing and UI development. The Tauri backend (`src-tauri/src/main.rs`) exposes commands that the frontend calls via `window.__TAURI__.invoke()`.

### File Structure
```
ui/
├── index.html      # Main HTML structure (navigation, views, layouts)
├── app.js          # JavaScript logic (event handlers, state management)
├── styles.css      # CSS styling (layout, components, responsive)
└── TODO.md         # Feature roadmap
```

### Development Workflow

**1. Start the local dev server:**
```bash
cd ui
python3 -m http.server 8080
```

**2. Use Playwright to interact with the UI:**
- Navigate: `mcp__playwright__browser_navigate` → `http://localhost:8080`
- Inspect: `mcp__playwright__browser_snapshot` → Get accessibility tree
- Click: `mcp__playwright__browser_click` → Interact with elements
- Type: `mcp__playwright__browser_type` → Fill form fields
- Screenshot: `mcp__playwright__browser_take_screenshot` → Capture visual state
- Evaluate: `mcp__playwright__browser_evaluate` → Run JavaScript directly

**3. Workflow for adding features:**
1. Read the current state with `browser_snapshot` to identify element refs
2. Edit HTML/CSS/JS files in `ui/`
3. Reload the page with `browser_navigate`
4. Verify changes with `browser_snapshot` or `browser_take_screenshot`
5. Test interactions (clicks, form fills, toggles) with Playwright
6. Iterate until feature is complete

### Key Patterns

**Element References:** Use the `ref=` from `browser_snapshot` (e.g., `ref=e74`) when clicking or interacting.

**State Management in app.js:**
- Use module-level variables for shared state (e.g., `allFiles`, `agentTouchedFiles`)
- Call Tauri commands: `const result = await window.__TAURI__.invoke('command_name', { param: value })`
- Update UI by manipulating the DOM directly

**File Filtering Example:**
```javascript
// Load agent-touched files from backend
const agentRes = await invoke('list_agent_touched_files', { projectDir });
agentTouchedFiles = new Set(agentRes?.files ?? []);

// Filter display based on checkbox state
const visibleFiles = agentTouchedOnlyCheckbox?.checked
  ? allFiles.filter(f => agentTouchedFiles.has(f))
  : allFiles;
```

**Checkbox/Toggle Events:**
```javascript
checkboxElement.addEventListener('change', () => {
  renderFileList(); // Re-render with new filter
});
```

### Testing with Playwright

**Before starting:**
- Ensure the HTTP server is running on port 8080
- Tauri app is not running (browser testing is for static prototype)

**Common test patterns:**
```javascript
// Take a snapshot to plan UI changes
await mcp__playwright__browser_snapshot();

// Click a button and verify state changed
await mcp__playwright__browser_click({ element: "...", ref: "e11" });
// Then snapshot again to confirm

// Fill a form field
await mcp__playwright__browser_type({
  element: "Project path input",
  ref: "e123",
  text: "/path/to/project"
});

// Evaluate JavaScript (test logic, set state)
await mcp__playwright__browser_evaluate({
  function: "() => document.getElementById('id').value = 'test'"
});
```

### Tauri Backend Integration

When adding new features, you may need to add Tauri commands:

1. Add a struct for the result in `src-tauri/src/main.rs`:
   ```rust
   #[derive(Serialize)]
   struct MyResult {
       data: Vec<String>,
   }
   ```

2. Add a command function:
   ```rust
   #[tauri::command]
   fn my_command(param: String) -> Result<MyResult, String> {
       // logic...
       Ok(MyResult { data: vec![] })
   }
   ```

3. Register it in `main()`:
   ```rust
   .invoke_handler(tauri::generate_handler![
       // ... existing commands ...
       my_command
   ])
   ```

4. Call from JS:
   ```javascript
   const result = await window.__TAURI__.invoke('my_command', { param: 'value' });
   ```

### Tips & Best Practices

- **Browser-only preview:** The static HTML works in any browser; use Playwright for automated testing before building Tauri.
- **Responsive design:** Test on different viewport sizes with `mcp__playwright__browser_resize`.
- **Status messages:** Use the status bar (`#status-text`) to give feedback during long operations.
- **Error handling:** In JS, wrap Tauri calls in try/catch and show meaningful status messages.
- **Default to minimal:** The "Agent-touched only" checkbox defaults to checked for a minimal view; users can uncheck to see everything.
- **Batch updates:** When loading files, fetch all data first (all files + agent-touched files) then render once to avoid jank.
