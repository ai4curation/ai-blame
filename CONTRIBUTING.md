# Contributing to ai-blame

Thank you for your interest in contributing to ai-blame!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/ai-blame`
3. Create a branch: `git checkout -b my-feature`
4. Make your changes
5. Run tests: `cargo test`
6. Run linters: `cargo fmt && cargo clippy`
7. Commit your changes: `git commit -am 'Add new feature'`
8. Push to your fork: `git push origin my-feature`
9. Create a Pull Request

## Development Setup

### Prerequisites

- Rust 1.70+ (2021 edition)
- Cargo

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Developer fixtures (reproducible CLI testing)

This repo includes a small, repo-local example project plus a matching “fake home”
containing Claude trace files so you can try the CLI without touching your real
`~/.claude` directory:

- **Example project**: `tests/examples/simple/`
- **Fixture home**: `tests/examples/simple-home/`

Try it locally:

```bash
cd tests/examples/simple

# Show blame using the fixture traces (no need to set --trace-dir)
ai-blame blame main.py --home ../simple-home

# Include agent/tool info alongside model
ai-blame blame main.py --home ../simple-home --show-agent

# Block-level grouping
ai-blame blame main.py --home ../simple-home --blocks --show-agent
```

Notes:

- The `--home` flag controls where `ai-blame` looks for traces; it computes
  `$home/.claude/projects/<encoded-cwd>/` based on your current working directory.
- The fixture traces are copied from a real Claude Code session and then rewritten
  so `toolUseResult.filePath` points at the repo-local example files.

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run linter
cargo clippy

# Run linter with warnings as errors
cargo clippy -- -D warnings
```

## Python Package Distribution

ai-blame is distributed to Python users via PyPI using maturin. This allows Python developers to install it using `pip` or `uv` without needing Rust installed.

### Testing Python Builds Locally

```bash
# Build a wheel using uv (maturin is in optional dev dependencies)
uv run -e .[dev] maturin build --release

# The wheel will be in target/wheels/
# Install it to test
# Note: Wheel filenames use underscores per PEP 427, even though the package
# name in pyproject.toml uses a hyphen (ai-blame).
uv pip install target/wheels/ai_blame-*.whl

# Verify the installation
ai-blame --version
```

### Building for Development

```bash
# Install in development mode (editable install)
uv run -e .[dev] maturin develop

# Test the CLI
ai-blame --version
```

### CI/CD for Python Wheels

Python wheels are automatically built and published to PyPI when a new release is created:

1. Create a new release on GitHub
2. The `python-wheels.yml` workflow builds wheels for all platforms
3. Wheels are automatically uploaded to PyPI

See `.github/workflows/python-wheels.yml` for details.

### Python API (Not Currently Available)

The current version provides only the CLI binary, not Python API bindings. If you need programmatic Python access, please open an issue describing your use case. We can consider adding PyO3 bindings if there's sufficient demand.

## Making Releases

This project is released to two ecosystems: **Cargo** (Rust) and **PyPI** (Python/pre-built binaries). Both use the same version number and are released together.

### Version Strategy

We follow **semantic versioning** (major.minor.patch):

- **0.x.y**: Pre-1.0 development releases (breaking changes are OK)
- **1.0.0+**: Stable; breaking changes bump major version

Update version in `Cargo.toml`:

```toml
[package]
version = "0.2.0"  # Update this
```

### Checklist Before Release

- [ ] All tests passing: `cargo test`
- [ ] All linters passing: `cargo fmt` + `cargo clippy`
- [ ] Docs are up to date
- [ ] CHANGELOG updated (if you have one)
- [ ] Version updated in `Cargo.toml`
- [ ] All PRs merged to main branch

### Step 1: Test Everything Locally

```bash
# Run full test suite
cargo test

# Run linters
cargo fmt --check
cargo clippy -- -D warnings

# Test Python wheel build locally
uv run -e .[dev] maturin build --release

# Verify the wheel works
uv pip install target/wheels/ai_blame-*.whl
ai-blame --version
```

### Step 2: Commit Version Bump

```bash
# Update Cargo.toml with new version
# e.g., change "0.1.0" to "0.2.0"

git add Cargo.toml
git commit -m "Bump version to 0.5.0"
git push origin main
```

### Step 3: Create a Git Tag

Tags trigger the entire release pipeline (Cargo + Python wheels):

```bash
# Create an annotated tag
git tag -a v0.2.0 -m "Release version 0.2.0"

# Push the tag (this triggers CI/CD)
git push origin v0.2.0
```

**Important**: Tags must match the pattern `v*` (e.g., `v0.2.0`). The CI/CD workflows watch for this pattern.

### Step 4: Create GitHub Release

1. Go to https://github.com/ai4curation/ai-blame/releases
2. Click "Draft a new release"
3. Choose the tag you just pushed (e.g., `v0.2.0`)
4. Add release notes (highlights of changes, breaking changes if any)
5. Check "Set as the latest release"
6. Click "Publish release"

This triggers:
- **.github/workflows/python-wheels.yml** → Builds wheels for all platforms (Linux x86_64/aarch64, macOS Intel/Apple Silicon, Windows) and uploads to PyPI
- Cargo release (see below)

### Step 5: Release to Cargo

Once the GitHub release is created, publish to crates.io:

```bash
# Make sure you're on the release tag
git checkout v0.2.0

# Publish (requires crates.io account + auth token in ~/.cargo/credentials)
cargo publish
```

You'll be prompted to verify the package contents. Review and confirm.

**Note**: If you don't have a crates.io account yet:
1. Go to https://crates.io
2. Sign up with GitHub
3. Create an API token at https://crates.io/me
4. Run `cargo login` and paste your token

### Step 6: Verify Both Releases

**Cargo (Rust)**:
```bash
# Wait ~30 seconds for crates.io indexing
cargo search ai-blame
# Should show: ai-blame = "0.2.0"
```

**PyPI (Python)**:
```bash
# Wait ~1 minute for PyPI indexing
pip index versions ai-blame
# Should show: Available versions: 0.2.0, ...

# Try installing it
uv add --dev ai-blame  # or: pip install ai-blame
ai-blame --version
```

**GitHub Releases**:
- Check https://github.com/ai4curation/ai-blame/releases for your release
- Pre-built binaries should be attached (Linux, macOS, Windows)

---

## First Release Notes (For Chris)

Since you haven't done an initial Rust release yet, here are some things to know:

1. **Your first release will be v0.1.0** (or whatever version you decide)
2. **PyPI wheels will publish automatically** — GitHub Actions builds them and uploads to PyPI using the `PYPI_API_TOKEN` secret
3. **You'll need a crates.io account** — Create one before publishing
4. **Publish Cargo after GitHub release** — The GitHub release triggers the wheel builds, but you publish Cargo manually
5. **Allow time for indexing** — Both Cargo and PyPI take 30-60 seconds to index new releases

**After your first release**, please update this guide with any surprises or gotchas you encounter. The workflow should be smooth, but real-world experience often reveals tweaks needed!

---

## Code Style

- Follow Rust standard style guidelines (enforced by `cargo fmt`)
- Write clear, descriptive commit messages
- Add tests for new functionality
- Update documentation as needed

## Pull Request Guidelines

- Keep PRs focused on a single feature or fix
- Include tests for new functionality
- Update README.md if adding user-facing features
- Ensure all tests pass
- Ensure `cargo fmt` and `cargo clippy` pass with no warnings

## Areas for Contribution

We especially welcome contributions in these areas:

- **Additional agent support**: Add support for other AI coding assistants (Cursor, Aider, Copilot, etc.)
- **Performance improvements**: Optimize trace parsing and file processing
- **Bug fixes**: Fix reported issues
- **Documentation**: Improve README, add examples, write tutorials
- **Testing**: Expand test coverage
- **Features**: Implement new output modes or filtering options

## Working on the Tauri UI with AI Agents

The repository includes a Tauri-based desktop UI shell (`src-tauri/`) with a static prototype layout in `ui/`. AI coding agents (like Claude Code, GitHub Copilot, etc.) can effectively work on this UI using **Playwright MCP** (Model Context Protocol) for browser automation.

### UI Overview

![Screenshot of AI Blame UI showing sidebar navigation with Home, Blame Viewer, Trace Explorer, and Settings tabs, plus status cards and feature cards in the main content area](https://github.com/user-attachments/assets/84dcd892-d0ce-4428-83b2-ddf34b4c3b4a)

The UI prototype features:
- **Sidebar navigation**: Home, Blame Viewer, Trace Explorer, Settings
- **Live status cards**: Display trace detection status
- **Feature cards**: Quick access to core functionality

### Setting Up Playwright MCP

Playwright MCP allows AI agents to interact with web UIs visually—navigating, clicking, taking screenshots, and verifying changes in real-time.

#### 1. Install Playwright MCP Server

Add the Playwright MCP server to your AI agent's MCP configuration. For Claude Desktop, add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "playwright": {
      "command": "npx",
      "args": ["@anthropic-ai/mcp-server-playwright"]
    }
  }
}
```

Or install globally:

```bash
npm install -g @anthropic-ai/mcp-server-playwright
```

#### 2. Start a Local Development Server

The static UI prototype can be served locally:

```bash
# From the repository root
cd ui
python3 -m http.server 8080
# Or use any static file server
npx serve .
```

For Tauri development:

```bash
# Install Tauri CLI if not already installed
cargo install tauri-cli

# Run in development mode
cd src-tauri
cargo tauri dev
```

### Agent Workflow for UI Development

Here's the typical workflow when AI agents work on the Tauri UI:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    AI Agent + Playwright MCP Flow                    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  1. START SERVER                                                      │
│     └─► python3 -m http.server 8080  (in ui/ directory)              │
│                                                                       │
│  2. NAVIGATE TO UI                                                    │
│     └─► browser_navigate → http://localhost:8080                     │
│                                                                       │
│  3. INSPECT CURRENT STATE                                             │
│     └─► browser_snapshot → Get accessibility tree of UI elements     │
│                                                                       │
│  4. MAKE CODE CHANGES                                                 │
│     └─► Edit ui/index.html, ui/styles.css, ui/app.js                 │
│                                                                       │
│  5. REFRESH & VERIFY                                                  │
│     └─► browser_navigate → Reload page                                │
│     └─► browser_snapshot → Verify changes rendered correctly         │
│                                                                       │
│  6. TAKE SCREENSHOT                                                   │
│     └─► browser_take_screenshot → Document the visual result         │
│                                                                       │
│  7. ITERATE                                                           │
│     └─► Repeat steps 4-6 until feature is complete                   │
│                                                                       │
└─────────────────────────────────────────────────────────────────────┘
```

### Common Playwright MCP Commands for UI Work

| Command | Purpose | Example |
|---------|---------|---------|
| `browser_navigate` | Load a URL | `http://localhost:8080` |
| `browser_snapshot` | Get current page structure | Returns accessibility tree |
| `browser_click` | Click UI elements | Click buttons, nav items |
| `browser_type` | Enter text in inputs | Fill form fields |
| `browser_take_screenshot` | Capture visual state | Document changes |
| `browser_evaluate` | Run JavaScript | Test custom logic |

### Example: Adding a New UI Feature

Here's how an agent might add a new button to the UI:

1. **Start the server and navigate**:
   ```
   Agent: browser_navigate → http://localhost:8080
   ```

2. **Inspect current state**:
   ```
   Agent: browser_snapshot → See current UI structure
   ```

3. **Edit the HTML**:
   ```html
   <!-- In ui/index.html, add new button -->
   <button class="primary" data-action="new-feature">New Feature</button>
   ```

4. **Add JavaScript handler** (in `ui/app.js`):
   ```javascript
   document.querySelector('[data-action="new-feature"]')
     ?.addEventListener('click', () => {
       updateStatus('New feature clicked!');
     });
   ```

5. **Refresh and verify**:
   ```
   Agent: browser_navigate → Reload page
   Agent: browser_snapshot → Confirm button appears
   Agent: browser_click → Test the new button
   ```

6. **Take screenshot for documentation**:
   ```
   Agent: browser_take_screenshot → Save visual proof
   ```

### UI File Structure

```
ui/
├── index.html      # Main HTML structure
├── styles.css      # CSS styles and layout
├── app.js          # JavaScript interactions
└── TODO.md         # UI development roadmap

src-tauri/
├── src/            # Rust backend code
├── Cargo.toml      # Tauri dependencies
├── tauri.conf.json # Tauri configuration
└── icons/          # App icons
```

### Tips for AI Agents Working on the UI

1. **Always start with `browser_snapshot`**: This gives you the current state of all UI elements and their references.

2. **Use semantic selectors**: The UI uses `data-view` and `data-action` attributes for easy targeting.

3. **Test incrementally**: Make small changes and verify with screenshots before making more changes.

4. **Check the status bar**: The UI has a status bar at the bottom that shows feedback for actions.

5. **Review `ui/TODO.md`**: This file tracks what's implemented vs. mocked in the prototype.

6. **Tauri backend wiring**: For features requiring backend integration, see `src-tauri/src/` and the commands in `PLAN-UI.md`.

### Debugging Common Issues

- **Server not starting**: Ensure port 8080 is free, or use a different port
- **Changes not visible**: Hard refresh the browser or clear cache
- **Tauri commands not working**: Check that `withGlobalTauri` is set in `tauri.conf.json`
- **JavaScript errors**: Use `browser_console_messages` to see console output

## Questions?

Feel free to open an issue for any questions or discussions.

## License

By contributing, you agree that your contributions will be licensed under the BSD-3-Clause License.
