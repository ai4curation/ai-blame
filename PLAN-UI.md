# Plan: Tauri-Based User Interface

## Overview

Create a modern, cross-platform desktop application using Tauri that provides a graphical user interface for ai-blame. This will make the tool more accessible to developers who prefer visual interfaces and enable more advanced visualization features.

## Current State

ai-blame.rs is currently:
- A CLI-only tool
- Text-based output (terminal)
- Configured via YAML files
- Focused on batch processing of traces

## Goals

1. **Visual blame viewer**: Interactive GUI similar to GitKraken or GitHub's blame view
2. **Trace explorer**: Browse and analyze AI agent traces visually
3. **Configuration GUI**: Visual editor for `.ai-blame.yaml` config
4. **Real-time monitoring**: Watch for new traces and update UI live
5. **Rich visualizations**: Charts, graphs, and timelines of AI contributions
6. **Cross-platform**: Works on Windows, macOS, and Linux

## Why Tauri?

**Advantages**:
- **Small binary size**: ~3-5MB vs 50MB+ for Electron
- **Memory efficient**: Uses system webview instead of bundling Chromium
- **Rust backend**: Leverage existing ai-blame.rs code
- **Modern web frontend**: Use React, Vue, or Svelte for UI
- **Native performance**: Fast startup and low resource usage
- **Security**: Built-in security features, Rust's memory safety
- **Cross-platform**: Single codebase for all platforms

**Comparison with Alternatives**:
- vs Electron: 10x smaller, 50% less memory
- vs Native GUI (GTK/Qt): Easier UI development, better cross-platform
- vs Web app: No server needed, works offline, better performance

## Architecture

### System Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Tauri Application                  │
├─────────────────────────────────────────────────────┤
│                                                       │
│  ┌─────────────────┐         ┌──────────────────┐  │
│  │   Frontend (UI)  │◄───────►│  Backend (Rust)  │  │
│  │                  │   IPC   │                  │  │
│  │  - React/Vue     │         │  - ai-blame lib  │  │
│  │  - TypeScript    │         │  - Tauri Core    │  │
│  │  - TailwindCSS   │         │  - File System   │  │
│  └─────────────────┘         └──────────────────┘  │
│         ▲                              ▲            │
│         │                              │            │
│    WebView API                   Native APIs        │
└─────────┼──────────────────────────────┼───────────┘
          │                              │
    System WebView              OS File System
                                AI Agent Traces
```

### Tech Stack

**Backend (Rust)**:
- Tauri Core framework
- ai-blame library (existing)
- tokio for async operations
- notify for file system watching
- serde for JSON serialization

**Frontend**:
- **Framework**: React with TypeScript (or Vue/Svelte alternative)
- **Styling**: TailwindCSS for utility-first styling
- **Components**: shadcn/ui for polished UI components
- **State**: Zustand or Jotai for state management
- **Visualization**: D3.js or Chart.js for graphs
- **Code Display**: Monaco Editor (VS Code's editor) or Prism.js
- **Build**: Vite for fast development

**Communication**:
- Tauri Commands: Frontend ↔ Backend RPC
- Tauri Events: Backend → Frontend push notifications
- WebView API: Standard web APIs in frontend

## Implementation Phases

### Phase 1: Project Setup and Basic Shell (Weeks 1-2)

**Goal**: Create Tauri app scaffold with basic navigation

**Tasks**:
1. Initialize Tauri project
   ```bash
   cargo install create-tauri-app
   cargo create-tauri-app --rc
   ```

2. Project structure:
   ```
   ai-blame.rs/
   ├── src/                    # Existing Rust lib
   ├── src-tauri/             # Tauri backend
   │   ├── src/
   │   │   ├── main.rs        # Tauri entry point
   │   │   ├── commands.rs    # IPC commands
   │   │   └── state.rs       # App state
   │   ├── Cargo.toml
   │   └── tauri.conf.json    # Tauri config
   ├── ui/                    # Frontend
   │   ├── src/
   │   │   ├── App.tsx
   │   │   ├── main.tsx
   │   │   └── components/
   │   ├── package.json
   │   └── vite.config.ts
   └── Cargo.toml             # Workspace root
   ```

3. Basic UI layout
   - Main window with sidebar navigation
   - Header with app title and actions
   - Content area for views
   - Status bar

4. Navigation between core views:
   - Home/Dashboard
   - Blame Viewer
   - Trace Explorer
   - Settings

**Deliverable**: Working Tauri app that launches and shows basic UI

### Phase 2: Blame Viewer (Weeks 3-5)

**Goal**: Core feature - visual git-blame-like interface

**UI Components**:

1. **File Browser Panel** (left sidebar)
   - Tree view of project files
   - Filter by file type
   - Search files
   - Show files with AI edits highlighted
   - Sort by last modified, most edited, etc.

2. **Blame Display** (main area)
   - Code editor with blame annotations
   - Line numbers
   - Blame info for each line (model, timestamp)
   - Color coding by AI model
   - Hover tooltips with full details
   - Click to see full edit history
   - Split view: code + blame metadata

3. **Details Panel** (right sidebar)
   - Selected line/block details
   - Edit history timeline
   - Associated trace information
   - Related files changed in same session

**Features**:
- Syntax highlighting for all file types
- Blame color scheme per AI model
- Time-based filtering (show blame as of date)
- Model-based filtering (show only Claude edits)
- Export blame view as HTML/PDF

**Backend Commands**:
```rust
// src-tauri/src/commands.rs

#[tauri::command]
async fn get_file_tree(project_path: String) -> Result<FileTree, String> {
    // Return tree of files with metadata
}

#[tauri::command]
async fn get_file_blame(file_path: String) -> Result<BlameData, String> {
    // Return line-by-line blame information
}

#[tauri::command]
async fn get_line_history(file_path: String, line_num: usize) -> Result<Vec<Edit>, String> {
    // Return history of changes to specific line
}

#[tauri::command]
async fn export_blame(file_path: String, format: String) -> Result<String, String> {
    // Export blame view to file
}
```

**UI Mockup**:
```
┌──────────────────────────────────────────────────────────────────┐
│ ai-blame                         [−][□][×]                        │
├──────────┬─────────────────────────────────────────┬─────────────┤
│ Files    │ src/main.rs                             │ Details     │
│          │ ┌─────────────────────────────────────┐ │             │
│ 📁 src   │ │ 1  use ai_blame::cli;              │ │ Line 3      │
│   📄 main│ │ 2                                   │ │ ───────────│
│   📄 cli │ │ 3  fn main() {                     │ │ Modified by │
│   📄 conf│ │ 4    if let Err(e) = cli::run() {  │ │ Claude Opus │
│          │ │ 5      eprintln!("Error: {}", e);  │ │ 2025-12-02  │
│ 📁 tests │ └─────────────────────────────────────┘ │ 14:22:10    │
│          │                                          │             │
│ [Filter] │ claude-opus-4   [█████]  42 lines       │ Timeline    │
│ [Search] │ claude-sonnet-4 [███]    18 lines       │ ─────────   │
│          │ human           [█]       5 lines       │ • Dec 2 ... │
└──────────┴─────────────────────────────────────────┴─────────────┘
```

### Phase 3: Trace Explorer (Weeks 6-7)

**Goal**: Visual interface to browse and analyze AI agent traces

**UI Components**:

1. **Trace List** (left panel)
   - List of all traces chronologically
   - Filter by date range
   - Filter by agent/model
   - Search trace content
   - Group by project/session

2. **Trace Details** (main area)
   - Trace metadata (timestamp, model, tool)
   - Files changed in this trace
   - Summary of changes (lines added/modified/deleted)
   - Full trace content viewer (JSON/YAML)
   - Diff view of changes

3. **Statistics Dashboard** (top cards)
   - Total traces processed
   - Files modified by AI
   - Most active AI model
   - Time range of traces
   - Changes per day/week/month chart

**Features**:
- Timeline visualization of traces
- Heatmap of AI activity
- Filter traces by multiple criteria
- Export trace data
- Jump from trace to blame view

**Backend Commands**:
```rust
#[tauri::command]
async fn get_traces(filter: TraceFilter) -> Result<Vec<Trace>, String> {
    // Return list of traces matching filter
}

#[tauri::command]
async fn get_trace_details(trace_id: String) -> Result<TraceDetails, String> {
    // Return full details of a trace
}

#[tauri::command]
async fn get_trace_stats(project_path: String) -> Result<TraceStats, String> {
    // Return statistics about traces
}

#[tauri::command]
async fn refresh_traces(project_path: String) -> Result<(), String> {
    // Re-scan for new traces
}
```

### Phase 4: Configuration GUI (Week 8)

**Goal**: Visual editor for .ai-blame.yaml configuration

**UI Components**:

1. **General Settings Tab**
   - Project path selection
   - Trace directory location
   - Default policy (dropdown: sidecar/append/comment/skip)
   - Sidecar pattern template

2. **Rules Editor Tab**
   - List of rules (sortable)
   - Add/edit/delete rules
   - Pattern input with validation
   - Policy dropdown per rule
   - Format selector (yaml/json)
   - Comment syntax selector

3. **Advanced Tab**
   - Time range filters
   - File size limits
   - Exclude patterns
   - Custom sidecar templates

4. **Preview**
   - Show current config as YAML
   - Validate configuration
   - Test rules against example files

**Features**:
- Visual rule builder (no YAML editing needed)
- Drag-and-drop to reorder rules
- Import/export configurations
- Configuration templates (Python project, Rust project, etc.)
- Real-time validation and error messages

**Backend Commands**:
```rust
#[tauri::command]
async fn load_config(config_path: String) -> Result<Config, String> {
    // Load .ai-blame.yaml
}

#[tauri::command]
async fn save_config(config: Config, config_path: String) -> Result<(), String> {
    // Save configuration to file
}

#[tauri::command]
async fn validate_config(config: Config) -> Result<ValidationResult, String> {
    // Validate configuration
}

#[tauri::command]
async fn test_rule(rule: Rule, test_path: String) -> Result<bool, String> {
    // Test if rule matches path
}
```

### Phase 5: Real-time Monitoring (Week 9)

**Goal**: Watch for new traces and update UI automatically

**Features**:
1. File system watcher for trace directory
2. Automatic refresh when new traces appear
3. Notifications for new AI edits
4. Live update of blame view when file changes
5. Activity log showing recent traces

**Implementation**:
```rust
use notify::{Watcher, RecursiveMode};
use tauri::Manager;

// In main.rs
fn setup_file_watcher(app: tauri::AppHandle) {
    let (tx, rx) = std::sync::mpsc::channel();
    
    let mut watcher = notify::recommended_watcher(tx).unwrap();
    watcher.watch(&trace_dir, RecursiveMode::Recursive).unwrap();
    
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = rx.recv() {
            // Notify frontend of new trace
            app.emit_all("trace-added", &event).unwrap();
        }
    });
}
```

**Frontend Integration**:
```typescript
// Listen for new traces
import { listen } from '@tauri-apps/api/event';

listen('trace-added', (event) => {
  // Refresh trace list
  refreshTraces();
  // Show notification
  showNotification('New AI trace detected');
});
```

### Phase 6: Visualizations and Analytics (Week 10)

**Goal**: Advanced visualization of AI contribution patterns

**Visualizations**:

1. **Timeline Chart**
   - X-axis: Time
   - Y-axis: Number of changes
   - Color-coded by AI model
   - Interactive (zoom, pan, tooltip)

2. **Contribution Heatmap**
   - Calendar-style heatmap
   - Shows AI activity by day
   - Intensity indicates number of changes
   - Click to see details for that day

3. **File Change Graph**
   - Network graph of related file changes
   - Nodes: files
   - Edges: changed in same session
   - Size: number of changes

4. **Model Comparison**
   - Pie chart of contributions by model
   - Bar chart of lines changed per model
   - Stats: avg changes per trace, etc.

5. **Code Churn Visualization**
   - Show files with most AI churn
   - Identify frequently modified areas
   - Highlight potential problem areas

**Libraries**:
- D3.js for custom visualizations
- Chart.js for standard charts
- vis-network for graph visualization

### Phase 7: Polish and Distribution (Weeks 11-12)

**Goal**: Production-ready application with installers

**Tasks**:

1. **UI Polish**
   - Consistent theme and styling
   - Dark mode support
   - Keyboard shortcuts
   - Loading states and error handling
   - Accessibility (ARIA labels, keyboard nav)
   - Animations and transitions

2. **Performance Optimization**
   - Lazy loading of large file lists
   - Virtual scrolling for long lists
   - Web worker for heavy computations
   - Debounce/throttle expensive operations
   - Cache blame data

3. **Testing**
   - Unit tests for Rust commands
   - Integration tests for IPC
   - E2E tests with Playwright or Cypress
   - Manual testing on all platforms

4. **Documentation**
   - User guide with screenshots
   - Video tutorials
   - Keyboard shortcuts reference
   - FAQ

5. **Packaging and Distribution**
   - Build installers for Windows, macOS, Linux
   - Code signing (macOS, Windows)
   - Auto-updater integration
   - Release on GitHub releases
   - Optional: Homebrew formula, Snap, AppImage

**Tauri Updater**:
```rust
// Enable auto-update
#[cfg(not(debug_assertions))]
let updater = app.updater();
updater.check().await?;
```

**Build Commands**:
```bash
# Build for all platforms
npm run tauri build

# Generates:
# - Windows: .msi, .exe
# - macOS: .dmg, .app
# - Linux: .AppImage, .deb
```

## Configuration

### Tauri Config (`src-tauri/tauri.conf.json`)

```json
{
  "build": {
    "beforeBuildCommand": "npm run build",
    "beforeDevCommand": "npm run dev",
    "devPath": "http://localhost:5173",
    "distDir": "../ui/dist"
  },
  "package": {
    "productName": "AI Blame",
    "version": "0.1.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "fs": {
        "all": false,
        "readFile": true,
        "writeFile": true,
        "readDir": true,
        "scope": ["$HOME/**", "$LOCALDATA/**"]
      },
      "dialog": {
        "all": false,
        "open": true,
        "save": true
      },
      "shell": {
        "all": false,
        "execute": false
      }
    },
    "windows": [
      {
        "title": "AI Blame",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' asset: https://asset.localhost"
    }
  }
}
```

### Workspace Configuration

Update root `Cargo.toml`:
```toml
[workspace]
members = [
    ".",
    "src-tauri"
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Chris Mungall"]
license = "BSD-3-Clause"
```

## Dependencies

### New Crates (`src-tauri/Cargo.toml`)

```toml
[dependencies]
# Existing ai-blame library
ai-blame = { path = ".." }

# Tauri
tauri = { version = "1.5", features = ["dialog-open", "dialog-save", "fs-read-file", "fs-write-file", "fs-read-dir"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# File watching
notify = "6.1"

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Optional: Database for caching
rusqlite = { version = "0.31", optional = true }
```

### Frontend Dependencies (`ui/package.json`)

```json
{
  "name": "ai-blame-ui",
  "version": "0.1.0",
  "dependencies": {
    "@tauri-apps/api": "^1.5.0",
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "zustand": "^4.4.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^1.5.0",
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@vitejs/plugin-react": "^4.2.0",
    "typescript": "^5.3.0",
    "vite": "^5.0.0",
    "tailwindcss": "^3.4.0",
    "autoprefixer": "^10.4.0",
    "postcss": "^8.4.0"
  }
}
```

## Technical Challenges

### Challenge 1: Large File Performance
**Problem**: Displaying blame for 10k+ line files
**Solution**: 
- Virtual scrolling (only render visible lines)
- Progressive loading
- Web workers for parsing
- Cache rendered components

### Challenge 2: Cross-Platform Consistency
**Problem**: Different webview engines on each OS
**Solution**:
- Test thoroughly on all platforms
- Use CSS that works across webviews
- Polyfills for missing features
- Platform-specific CSS when needed

### Challenge 3: State Synchronization
**Problem**: Keeping frontend state in sync with file system
**Solution**:
- File watcher for real-time updates
- Periodic polling as backup
- Optimistic updates with rollback
- Event-driven architecture

### Challenge 4: Security
**Problem**: Allowing file system access safely
**Solution**:
- Tauri's scope restrictions
- Whitelist allowed directories
- Validate all paths
- No arbitrary code execution

## User Experience Design

### Key Principles
1. **Familiar**: Similar to git UIs users already know
2. **Fast**: Instant feedback, smooth animations
3. **Clear**: Obvious what each view shows
4. **Helpful**: Tooltips, guides, error messages
5. **Accessible**: Keyboard navigation, screen readers

### Keyboard Shortcuts
- `Cmd/Ctrl + O`: Open project
- `Cmd/Ctrl + R`: Refresh traces
- `Cmd/Ctrl + F`: Search files
- `Cmd/Ctrl + ,`: Open settings
- `Cmd/Ctrl + T`: Toggle theme
- Arrow keys: Navigate file tree
- `Enter`: Open file in blame view

### Theme Support
- Light mode (default)
- Dark mode
- Auto (follow system)
- Custom theme colors per AI model

## Testing Strategy

### Backend Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_tree() {
        let result = get_file_tree("/test/project".into());
        assert!(result.is_ok());
    }
}
```

### Frontend Testing
```typescript
// Component test
import { render, screen } from '@testing-library/react';
import BlameViewer from './BlameViewer';

test('renders blame viewer', () => {
  render(<BlameViewer filePath="/test/file.rs" />);
  expect(screen.getByText(/fn main/)).toBeInTheDocument();
});
```

### E2E Testing
```typescript
// Tauri E2E test
import { _electron as electron } from 'playwright';

test('launch app', async () => {
  const app = await electron.launch({ args: ['dist/app'] });
  const window = await app.firstWindow();
  await expect(window.title()).toBe('AI Blame');
});
```

## Documentation

### User Documentation
- **Getting Started Guide**: Install, launch, open first project
- **Blame Viewer Tutorial**: How to use the blame interface
- **Configuration Guide**: Setting up project rules
- **Keyboard Shortcuts**: Quick reference card
- **Troubleshooting**: Common issues and solutions

### Developer Documentation
- **Architecture Overview**: How frontend and backend communicate
- **Adding New Features**: Guide for contributors
- **Building from Source**: Development setup
- **Release Process**: How to build and publish

## Success Metrics

1. **Performance**: App launches in <2 seconds
2. **Responsiveness**: UI actions respond in <100ms
3. **Memory**: Uses <200MB for typical projects
4. **Binary size**: <10MB installer
5. **Compatibility**: Works on Windows 10+, macOS 11+, Ubuntu 20.04+

## Future Enhancements

### Phase 2 Features
1. **Diff View**: Side-by-side comparison of versions
2. **Blame Annotations**: Add notes/comments to blame history
3. **Integration**: Open files in external editor
4. **Teams**: Multi-user provenance tracking
5. **Web Export**: Generate standalone HTML report
6. **AI Chat**: Query blame history in natural language

### Advanced Features
1. **VSCode Extension**: Inline blame in editor
2. **CI/CD Integration**: Generate reports in pipelines
3. **Cloud Sync**: Share configurations across machines
4. **Plugins**: Extensibility for custom visualizations

## Timeline Summary

- **Weeks 1-2**: Project setup and basic shell
- **Weeks 3-5**: Blame viewer (core feature)
- **Weeks 6-7**: Trace explorer
- **Week 8**: Configuration GUI
- **Week 9**: Real-time monitoring
- **Week 10**: Visualizations and analytics
- **Weeks 11-12**: Polish and distribution

**Total Estimated Time**: 12 weeks for MVP

## Open Questions

1. Should we bundle Monaco Editor (large) or use simpler syntax highlighting?
2. What's the minimum supported OS versions?
3. Should config GUI write back to .ai-blame.yaml or separate UI config?
4. Do we need offline mode or assume internet for updates?
5. Should we implement telemetry for crash reporting?

## References

- [Tauri Documentation](https://tauri.app/)
- [Tauri Examples](https://github.com/tauri-apps/tauri/tree/dev/examples)
- [GitKraken UI](https://www.gitkraken.com/) - Inspiration for blame UI
- [VS Code Architecture](https://code.visualstudio.com/api) - For editor integration
- [shadcn/ui](https://ui.shadcn.com/) - Component library
