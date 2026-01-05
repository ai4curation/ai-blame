# Plan: Ubiquitous Workflow Integration (Like uv)

## Overview

Make ai-blame.rs a ubiquitous tool in developers' workflows, similar to how `uv` has become essential in Python development. This requires focusing on ease of installation, seamless integration, minimal friction, and becoming a default tool for AI-assisted development.

## Current State

ai-blame.rs is currently:
- Installed manually from source via `cargo install`
- Used as a standalone CLI tool
- Requires explicit configuration per project
- Run manually when needed
- Not integrated with other development tools

## Goals

1. **Zero-friction installation**: Install with a single command, no prerequisites
2. **Automatic integration**: Works out-of-the-box with common workflows
3. **IDE/Editor integration**: Built-in support in VS Code, IntelliJ, etc.
4. **CI/CD integration**: Automatic provenance tracking in pipelines
5. **Git integration**: Works naturally with git workflows
6. **Package manager availability**: Available via Homebrew, apt, winget, cargo-binstall
7. **Pre-commit hooks**: Automatic tracking before commits
8. **Universal compatibility**: Works with all major AI coding assistants

## Why uv is Ubiquitous

Learning from `uv`'s success:

**Key Success Factors**:
1. **Speed**: 10-100x faster than alternatives (pip, pip-tools)
2. **Simple installation**: Single curl command, single binary
3. **Zero config**: Works immediately without setup
4. **Replaces multiple tools**: pip, pip-tools, virtualenv, pipx all in one
5. **Backward compatible**: Drop-in replacement, no migration needed
6. **Great documentation**: Clear, concise, example-driven
7. **Reliable**: Just works, every time
8. **Community adoption**: Popular projects switched, creating momentum

**What we can learn**:
- Make it the easiest way to track AI provenance
- Replace/augment existing manual processes
- Work with existing tools, don't require switching
- Be incredibly fast and reliable
- Excellent onboarding experience

## Strategy

### Phase 1: Installation Excellence (Weeks 1-2)

**Goal**: Make installation trivially easy on all platforms

#### 1.1 Multiple Installation Methods

**Single-line installers**:
```bash
# macOS/Linux (curl)
curl -LsSf https://github.com/ai4curation/ai-blame/releases/latest/download/install.sh | sh

# Windows (PowerShell)
irm https://github.com/ai4curation/ai-blame/releases/latest/download/install.ps1 | iex

# Universal installer (works everywhere)
cargo binstall ai-blame
```

**Package managers**:
```bash
# Homebrew (macOS/Linux)
brew install ai-blame

# MacPorts
sudo port install ai-blame

# Scoop (Windows)
scoop install ai-blame

# Chocolatey (Windows)
choco install ai-blame

# APT (Debian/Ubuntu)
sudo apt install ai-blame

# DNF (Fedora/RHEL)
sudo dnf install ai-blame

# Arch User Repository
yay -S ai-blame

# Cargo (Rust)
cargo install ai-blame
```

**Pre-built binaries**:
- GitHub Releases with binaries for all platforms
- Automatic GitHub Actions workflow for releases
- Signed binaries for macOS/Windows
- Checksum verification

#### 1.2 Installation Script

Create `install.sh`:
```bash
#!/bin/sh
set -e

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case $ARCH in
    x86_64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Download URL
VERSION="latest"
URL="https://github.com/ai4curation/ai-blame/releases/$VERSION/download/ai-blame-$OS-$ARCH.tar.gz"

# Download and install
echo "Downloading ai-blame..."
curl -L "$URL" -o ai-blame.tar.gz
tar xzf ai-blame.tar.gz
chmod +x ai-blame

# Install to user's bin
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
mv ai-blame "$INSTALL_DIR/"
rm ai-blame.tar.gz

# Add to PATH if needed
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$HOME/.bashrc"
    echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$HOME/.zshrc"
fi

echo "✓ ai-blame installed successfully!"
echo "Run 'ai-blame --help' to get started"
```

#### 1.3 Automated Releases

**GitHub Actions** (`.github/workflows/release.yml`):
```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      
      - name: Package
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ai-blame-${{ matrix.target }}.tar.gz ai-blame
      
      - name: Upload to release
        uses: softprops/action-gh-release@v1
        with:
          files: target/${{ matrix.target }}/release/ai-blame-*.tar.gz
```

### Phase 2: Zero-Config Setup (Weeks 3-4)

**Goal**: Work immediately after installation without configuration

#### 2.1 Intelligent Defaults

**Auto-detect project type**:
```rust
// src/auto_config.rs

pub fn auto_detect_config(project_path: &Path) -> Config {
    let mut config = Config::default();
    
    // Detect Rust project
    if project_path.join("Cargo.toml").exists() {
        config.add_rule("*.rs", Policy::Sidecar);
        config.add_rule("Cargo.toml", Policy::Append);
    }
    
    // Detect Python project
    if project_path.join("pyproject.toml").exists() 
        || project_path.join("setup.py").exists() {
        config.add_rule("*.py", Policy::Comment);
        config.add_rule("*.yaml", Policy::Append);
        config.add_rule("*.yml", Policy::Append);
    }
    
    // Detect JavaScript/TypeScript project
    if project_path.join("package.json").exists() {
        config.add_rule("*.js", Policy::Sidecar);
        config.add_rule("*.ts", Policy::Sidecar);
        config.add_rule("*.json", Policy::Append);
    }
    
    // Detect Go project
    if project_path.join("go.mod").exists() {
        config.add_rule("*.go", Policy::Sidecar);
    }
    
    // Universal rules
    config.add_rule("*.md", Policy::Append);
    config.add_rule("*.txt", Policy::Append);
    config.add_rule(".github/**", Policy::Skip);
    config.add_rule("node_modules/**", Policy::Skip);
    config.add_rule("target/**", Policy::Skip);
    
    config
}
```

**Auto-discover traces**:
```rust
// Automatically find Claude Code traces
fn find_trace_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    
    // Check standard locations
    if let Some(home) = dirs::home_dir() {
        // Claude Code
        dirs.push(home.join(".claude/traces"));
        
        // Future: other agents
        dirs.push(home.join(".cursor/traces"));
        dirs.push(home.join(".aider/traces"));
    }
    
    // Check environment variables
    if let Ok(custom_dir) = env::var("AI_BLAME_TRACE_DIR") {
        dirs.push(PathBuf::from(custom_dir));
    }
    
    dirs.into_iter().filter(|d| d.exists()).collect()
}
```

#### 2.2 Init Command

**Quick setup wizard**:
```bash
$ ai-blame init

🤖 AI Blame Setup
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Detected: Rust project (Cargo.toml found)
✓ Auto-configured rules for Rust files

Trace locations found:
  • ~/.claude/traces (Claude Code)

Configuration saved to .ai-blame.yaml

Next steps:
  1. Run: ai-blame annotate --apply
  2. Add pre-commit hook: ai-blame install-hooks
  3. Check status: ai-blame stats

Done! Your project is ready for AI provenance tracking.
```

**One-command setup**:
```bash
# Initialize and run immediately
ai-blame init --apply
```

### Phase 3: Git Integration (Weeks 5-6)

**Goal**: Seamless integration with git workflows

#### 3.1 Git Hooks

**Pre-commit hook**:
```bash
# Install hook
$ ai-blame install-hooks

# Creates .git/hooks/pre-commit:
#!/bin/sh
# AI Blame pre-commit hook
# Automatically tracks AI provenance before commits

ai-blame annotate --apply --staged-files-only
```

**Automatic hook installation**:
```rust
#[derive(Parser)]
struct InstallHooks {
    #[arg(long)]
    force: bool,
}

fn install_git_hooks(force: bool) -> Result<()> {
    let git_dir = find_git_dir()?;
    let hooks_dir = git_dir.join("hooks");
    let pre_commit = hooks_dir.join("pre-commit");
    
    if pre_commit.exists() && !force {
        println!("Pre-commit hook already exists. Use --force to overwrite.");
        return Ok(());
    }
    
    let hook_content = include_str!("../hooks/pre-commit.sh");
    fs::write(&pre_commit, hook_content)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&pre_commit)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&pre_commit, perms)?;
    }
    
    println!("✓ Pre-commit hook installed");
    Ok(())
}
```

#### 3.2 Git Commands Integration

**Git alias**:
```bash
# Setup git aliases
$ ai-blame setup-git

# Adds to .gitconfig:
[alias]
    ai-blame = "!ai-blame show"
    ai-stats = "!ai-blame stats"
    ai-annotate = "!ai-blame annotate --apply"
```

**Use with git**:
```bash
# Show AI blame for a file
git ai-blame src/main.rs

# Show AI stats for repo
git ai-stats
```

#### 3.3 Git Diff Integration

**Enhanced git diff**:
```bash
# Show diffs with AI attribution
git diff --ai-blame

# Implementation: git diff wrapper
ai-blame diff HEAD~1 HEAD --show-models
```

### Phase 4: IDE Integration (Weeks 7-10)

**Goal**: Native support in popular editors

#### 4.1 VS Code Extension

**Extension features**:
- Inline blame annotations (like GitLens)
- Hover tooltips showing AI model
- Status bar showing current file's AI provenance
- Sidebar with AI edit history
- Command palette commands
- Auto-run on save (optional)

**Package structure**:
```
vscode-ai-blame/
├── package.json
├── src/
│   ├── extension.ts       # Main extension
│   ├── blame.ts          # Blame provider
│   ├── commands.ts       # Commands
│   └── decorations.ts    # UI decorations
└── README.md
```

**Key features**:
```typescript
// Inline blame annotations
const decorationType = vscode.window.createTextEditorDecorationType({
    after: {
        color: new vscode.ThemeColor('editorCodeLens.foreground'),
        contentText: ' // AI: Claude Opus, 2 days ago'
    }
});

// Hover provider
class AIBlameHoverProvider implements vscode.HoverProvider {
    provideHover(document, position, token) {
        const blame = getBlameForLine(document.fileName, position.line);
        return new vscode.Hover(`
            **AI Model**: ${blame.model}
            **Date**: ${blame.timestamp}
            **Action**: ${blame.action}
        `);
    }
}

// Commands
vscode.commands.registerCommand('aiBlame.show', () => {
    // Show blame in sidebar
});

vscode.commands.registerCommand('aiBlame.update', () => {
    // Run ai-blame annotate
});
```

**Publish to VS Code marketplace**:
```bash
vsce package
vsce publish
```

#### 4.2 IntelliJ Plugin

**Plugin features**:
- Gutter icons showing AI provenance
- Annotate view (like git annotate)
- Tool window with blame history
- Inspection warnings for untracked AI edits
- Automatic mining on project open

**Plugin structure** (Kotlin):
```kotlin
// AIBlameAnnotator.kt
class AIBlameAnnotator : Annotator {
    override fun annotate(element: PsiElement, holder: AnnotationHolder) {
        val blame = getBlameForElement(element)
        if (blame != null) {
            holder.newAnnotation(HighlightSeverity.INFORMATION, 
                "AI: ${blame.model}")
                .range(element.textRange)
                .create()
        }
    }
}
```

#### 4.3 Other Editor Support

**Vim/Neovim plugin**:
```lua
-- ai-blame.nvim
local M = {}

function M.show_blame()
    local line = vim.fn.line('.')
    local file = vim.fn.expand('%:p')
    local blame = vim.fn.system('ai-blame show ' .. file .. ' --line ' .. line)
    vim.notify(blame)
end

return M
```

**Emacs package**:
```elisp
;; ai-blame.el
(defun ai-blame-show ()
  "Show AI blame for current line"
  (interactive)
  (let* ((line (line-number-at-pos))
         (file (buffer-file-name))
         (blame (shell-command-to-string 
                  (format "ai-blame show %s --line %d" file line))))
    (message blame)))
```

**Sublime Text plugin**:
```python
# ai_blame.py
import sublime
import sublime_plugin
import subprocess

class AiBlameCommand(sublime_plugin.TextCommand):
    def run(self, edit):
        line = self.view.rowcol(self.view.sel()[0].begin())[0] + 1
        file = self.view.file_name()
        result = subprocess.check_output(['ai-blame', 'show', file, '--line', str(line)])
        sublime.message_dialog(result.decode())
```

### Phase 5: CI/CD Integration (Weeks 11-12)

**Goal**: Automatic provenance tracking in CI pipelines

#### 5.1 GitHub Actions Integration

**Action definition** (`.github/actions/ai-blame/action.yml`):
```yaml
name: 'AI Blame'
description: 'Track AI provenance in your codebase'
inputs:
  apply:
    description: 'Apply changes'
    required: false
    default: 'false'
  pattern:
    description: 'File pattern to process'
    required: false
runs:
  using: 'composite'
  steps:
    - name: Install ai-blame
      shell: bash
      run: |
        curl -LsSf https://ai-blame.rs/install.sh | sh
        echo "$HOME/.local/bin" >> $GITHUB_PATH
    
    - name: Extract provenance
      shell: bash
      run: |
        ai-blame annotate \
          ${{ inputs.apply == 'true' && '--apply' || '' }} \
          ${{ inputs.pattern && format('--pattern {0}', inputs.pattern) || '' }}
```

**Usage in workflows**:
```yaml
# .github/workflows/ai-blame.yml
name: AI Provenance

on: [push, pull_request]

jobs:
  track-provenance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - uses: cmungall/ai-blame-action@v1
        with:
          apply: true
      
      - name: Commit changes
        run: |
          git config user.name "AI Blame Bot"
          git config user.email "bot@ai-blame.rs"
          git add .
          git commit -m "chore: update AI provenance" || true
          git push
```

#### 5.2 Pre-built Actions

**Reusable workflows**:
```yaml
# .github/workflows/reusable-ai-blame.yml
name: Reusable AI Blame

on:
  workflow_call:
    inputs:
      apply:
        type: boolean
        default: false

jobs:
  ai-blame:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: cmungall/ai-blame-action@v1
        with:
          apply: ${{ inputs.apply }}
```

**Use in projects**:
```yaml
# In your project
jobs:
  track:
    uses: cmungall/ai-blame/.github/workflows/reusable-ai-blame.yml@main
    with:
      apply: true
```

#### 5.3 Other CI Systems

**GitLab CI** (`.gitlab-ci.yml`):
```yaml
ai-blame:
  stage: test
  script:
    - curl -LsSf https://ai-blame.rs/install.sh | sh
    - export PATH="$HOME/.local/bin:$PATH"
    - ai-blame annotate --apply
  artifacts:
    paths:
      - "**/*.history.yaml"
```

**Jenkins** (Jenkinsfile):
```groovy
pipeline {
    agent any
    stages {
        stage('AI Blame') {
            steps {
                sh 'curl -LsSf https://ai-blame.rs/install.sh | sh'
                sh 'export PATH="$HOME/.local/bin:$PATH"'
                sh 'ai-blame annotate --apply'
            }
        }
    }
}
```

**CircleCI** (`.circleci/config.yml`):
```yaml
version: 2.1

jobs:
  ai-blame:
    docker:
      - image: cimg/rust:1.75
    steps:
      - checkout
      - run:
          name: Install ai-blame
          command: cargo install ai-blame
      - run:
          name: Track provenance
          command: ai-blame annotate --apply
```

### Phase 6: Package Manager Integration (Weeks 13-14)

**Goal**: Available through all major package managers

#### 6.1 Homebrew Formula

**Create formula** (`homebrew-ai-blame/ai-blame.rb`):
```ruby
class AiBlame < Formula
  desc "Track AI provenance in your codebase"
  homepage "https://github.com/ai4curation/ai-blame"
  url "https://github.com/ai4curation/ai-blame/archive/v0.1.0.tar.gz"
  sha256 "..."
  license "BSD-3-Clause"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/ai-blame", "--version"
  end
end
```

**Tap setup**:
```bash
# Users install with:
brew tap cmungall/ai-blame
brew install ai-blame

# Or direct install:
brew install cmungall/ai-blame/ai-blame
```

#### 6.2 Cargo Binstall Support

**Add metadata to Cargo.toml**:
```toml
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/v{ version }/{ name }-{ target }.tar.gz"
bin-dir = "{ bin }{ binary-ext }"
pkg-fmt = "tgz"
```

**Users install with**:
```bash
cargo binstall ai-blame
```

#### 6.3 Debian/Ubuntu Package

**Create debian package**:
```bash
# debian/control
Source: ai-blame
Section: devel
Priority: optional
Maintainer: Chris Mungall <...>
Build-Depends: cargo, rustc
Standards-Version: 4.5.0
Homepage: https://github.com/ai4curation/ai-blame

Package: ai-blame
Architecture: any
Depends: ${shlibs:Depends}, ${misc:Depends}
Description: Track AI provenance in your codebase
 Like git blame, but for AI-assisted edits.
```

**Build and publish**:
```bash
dpkg-buildpackage -us -uc
# Upload to PPA or Debian mentors
```

#### 6.4 Windows Package Managers

**Scoop manifest** (`scoop-bucket/ai-blame.json`):
```json
{
    "version": "0.1.0",
    "description": "Track AI provenance in your codebase",
    "homepage": "https://github.com/ai4curation/ai-blame",
    "license": "BSD-3-Clause",
    "url": "https://github.com/ai4curation/ai-blame/releases/download/v0.1.0/ai-blame-windows.zip",
    "bin": "ai-blame.exe",
    "checkver": "github",
    "autoupdate": {
        "url": "https://github.com/ai4curation/ai-blame/releases/download/v$version/ai-blame-windows.zip"
    }
}
```

**Chocolatey package** (`chocolatey/ai-blame.nuspec`):
```xml
<?xml version="1.0"?>
<package>
  <metadata>
    <id>ai-blame</id>
    <version>0.1.0</version>
    <title>AI Blame</title>
    <authors>Chris Mungall</authors>
    <description>Track AI provenance in your codebase</description>
    <projectUrl>https://github.com/ai4curation/ai-blame</projectUrl>
    <license type="expression">BSD-3-Clause</license>
  </metadata>
</package>
```

### Phase 7: Community Building (Ongoing)

**Goal**: Build active community and ecosystem

#### 7.1 Documentation

**Comprehensive docs**:
- **Quick Start**: 5-minute getting started guide
- **Installation Guide**: All installation methods
- **Configuration Reference**: Complete config options
- **CLI Reference**: Every command and flag
- **Integration Guides**: VS Code, IntelliJ, CI/CD
- **Best Practices**: How to use effectively
- **Troubleshooting**: Common issues
- **FAQ**: Frequently asked questions
- **Cookbook**: Real-world examples

**Interactive tutorials**:
```bash
# Built-in tutorial
ai-blame tutorial

# Interactive lessons
ai-blame tutorial init
ai-blame tutorial first-annotate
ai-blame tutorial git-hooks
```

#### 7.2 Marketing and Adoption

**Content creation**:
- Blog posts: "Introducing ai-blame"
- Video tutorials on YouTube
- Conference talks (RustConf, etc.)
- Podcast appearances
- Twitter/X thread explaining benefits
- Reddit posts in r/rust, r/programming
- Hacker News Show HN post
- Product Hunt launch

**Social proof**:
- Get early adopters from AI coding community
- Case studies from real users
- Testimonials and quotes
- Usage statistics (stars, downloads, etc.)
- Integration with popular projects

**Community engagement**:
- Active GitHub Discussions
- Discord server for support
- Regular blog updates
- Office hours for questions
- Contributor guide
- Good first issues for contributors

#### 7.3 Partnerships

**Tool integrations**:
- **Claude Code**: Official integration
- **Cursor**: Built-in support
- **Aider**: Automatic provenance
- **Continue.dev**: Native support
- **GitHub Copilot**: Extension support

**Project adoption**:
- Reach out to popular open-source projects
- Offer to add ai-blame integration
- Create PRs adding it to notable repos
- Get endorsements from project maintainers

### Phase 8: Performance and Reliability (Weeks 15-16)

**Goal**: Make it incredibly fast and reliable

#### 8.1 Performance Optimization

**Benchmark suite**:
```rust
// benches/extracting.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_extract_traces(c: &mut Criterion) {
    c.bench_function("extract 100 traces", |b| {
        b.iter(|| extract_traces(black_box(&test_project)))
    });
}

criterion_group!(benches, bench_extract_traces);
criterion_main!(benches);
```

**Optimizations**:
- Parallel trace processing
- Incremental processing (only new traces)
- File caching to avoid re-reading
- Lazy loading of large files
- Efficient diff algorithms
- Memory-mapped file reading

**Performance targets**:
- Mine 1000 traces in <10 seconds
- Process 10k line file in <1 second
- Startup time <100ms
- Memory usage <50MB for typical use

#### 8.2 Reliability

**Error handling**:
- Graceful degradation on errors
- Clear error messages
- Automatic recovery when possible
- No data loss on crashes
- Safe concurrent access

**Testing**:
- Unit tests: >90% coverage
- Integration tests: All workflows
- Property tests: Edge cases
- Fuzz testing: Random inputs
- Stress testing: Large repos
- Platform testing: All OSes

**Monitoring**:
- Optional crash reporting (opt-in)
- Usage analytics (anonymous, opt-in)
- Performance metrics
- Error rate tracking

### Phase 9: Advanced Features for Ubiquity (Weeks 17-18)

**Goal**: Features that make it indispensable

#### 9.1 Smart Defaults

**Context-aware behavior**:
```rust
// Auto-adjust based on project size
fn get_config(project: &Project) -> Config {
    let mut config = base_config();
    
    if project.file_count() > 10000 {
        // Large project: be more selective
        config.set_pattern("src/**/*.rs");
        config.set_policy(Policy::Sidecar);
    } else {
        // Small project: track everything
        config.set_pattern("**/*");
    }
    
    if project.has_ci() {
        // Enable auto-mining in CI
        config.enable_ci_mode();
    }
    
    config
}
```

#### 9.2 Integration with AI Tools

**API for AI assistants**:
```rust
// Export API for AI tools to use
pub trait ProvenanceTracker {
    fn track_edit(&self, file: &Path, change: &Edit) -> Result<()>;
    fn get_blame(&self, file: &Path, line: usize) -> Result<Blame>;
    fn export_history(&self, file: &Path) -> Result<Vec<Edit>>;
}
```

**Claude Code integration**:
- Automatic tracking without separate step
- Built into Claude Code workflow
- No user intervention needed

**Universal agent support**:
- Plugin system for new agents
- Standard provenance format
- Easy to add new agent types

#### 9.3 Developer Experience

**Shell completions**:
```bash
# Generate completions
ai-blame completions bash > /etc/bash_completion.d/ai-blame
ai-blame completions zsh > /usr/local/share/zsh/site-functions/_ai-blame
ai-blame completions fish > ~/.config/fish/completions/ai-blame.fish
```

**Man pages**:
```bash
# Generate and install man pages
ai-blame man > /usr/local/share/man/man1/ai-blame.1
man ai-blame
```

**Built-in help**:
```bash
# Contextual help
ai-blame help
ai-blame annotate --help
ai-blame config --help

# Examples
ai-blame examples
ai-blame examples init
```

## Key Metrics for Success

### Installation Metrics
- **Time to install**: <30 seconds
- **Success rate**: >99%
- **Platform coverage**: Windows, macOS, Linux (all major versions)
- **Install methods**: 5+ different options

### Adoption Metrics
- **GitHub stars**: 1000+ in 6 months
- **Downloads**: 10k+ per month
- **Active projects**: 100+ using ai-blame
- **IDE extensions**: 1000+ installs

### Integration Metrics
- **Package managers**: Available in 5+ package managers
- **CI platforms**: Actions for GitHub, GitLab, Jenkins
- **IDE support**: VS Code, IntelliJ, Vim/Neovim
- **AI tools**: Integration with 3+ AI coding assistants

### Community Metrics
- **Contributors**: 20+ contributors
- **Issues/PRs**: Active issue tracker
- **Documentation**: Comprehensive docs, tutorials, examples
- **Support**: Active community helping each other

### Performance Metrics
- **Speed**: 10x faster than manual tracking
- **Reliability**: 99.9% uptime
- **Resource usage**: <50MB memory, <1% CPU
- **Accuracy**: >95% correct attribution

## Comparison: ai-blame vs Manual Tracking

| Aspect | Manual | ai-blame | Improvement |
|--------|--------|----------|-------------|
| Setup time | 30 min | 30 sec | 60x faster |
| Per-edit overhead | 2 min | 0 sec (auto) | ∞ |
| Accuracy | ~60% | >95% | 1.5x better |
| Consistency | Poor | Excellent | ✓ |
| CI integration | Manual | Automatic | ✓ |
| IDE support | None | Built-in | ✓ |
| Maintenance | Ongoing | None | ✓ |

## Timeline Summary

- **Weeks 1-2**: Installation excellence
- **Weeks 3-4**: Zero-config setup
- **Weeks 5-6**: Git integration
- **Weeks 7-10**: IDE integration
- **Weeks 11-12**: CI/CD integration
- **Weeks 13-14**: Package managers
- **Weeks 15-16**: Performance and reliability
- **Weeks 17-18**: Advanced features
- **Ongoing**: Community building

**Total Estimated Time**: 18 weeks for full ubiquity

## Success Stories (Anticipated)

> "ai-blame is now as essential to our workflow as git itself. We can't imagine working without it." - Developer at YC startup

> "Installation took 30 seconds, and it just worked. No configuration needed. This is how tools should be." - Solo developer

> "We added ai-blame to our CI pipeline and now automatically track all AI contributions. It's completely transparent." - Engineering manager

> "The VS Code extension is amazing. I see AI attribution inline as I code. It's like GitLens for AI." - Frontend developer

## Open Questions

1. Should we have a freemium model or stay fully open-source?
2. What's the right balance between auto-magic and explicit control?
3. Should we build a hosted service or stay purely client-side?
4. How to handle privacy concerns with trace data?
5. What's the minimum viable feature set for "ubiquity"?

## References

- [uv: Python packaging in Rust](https://github.com/astral-sh/uv)
- [ripgrep: Fast grep tool that achieved ubiquity](https://github.com/BurntSushi/ripgrep)
- [exa/eza: Modern ls replacement adoption strategy](https://github.com/eza-community/eza)
- [bat: cat clone with adoption lessons](https://github.com/sharkdp/bat)
- [GitHub Actions: CI/CD integration patterns](https://docs.github.com/en/actions)
- [VS Code Extension API](https://code.visualstudio.com/api)

## Conclusion

Achieving ubiquity requires:
1. **Effortless installation** - One command, works everywhere
2. **Zero friction** - Works out of the box, no configuration
3. **Native integrations** - Built into tools developers already use
4. **Reliability** - Never fails, never gets in the way
5. **Performance** - Fast enough to be invisible
6. **Community** - Active users and contributors
7. **Momentum** - Early adopters create viral growth

By following this plan, ai-blame can become as essential to AI-assisted development as git is to version control.
