# UI Prototype TODO

This UI is currently a **static prototype** (HTML/CSS/JS) hosted in a Tauri window.

## What works today
- Sidebar navigation (Home / Blame Viewer / Trace Explorer / Settings)
- Status bar messages for button clicks
- In the desktop app (Tauri):
  - **Open Project**: opens a folder picker (requires `withGlobalTauri`)
  - **Scan / Refresh Traces**: calls the backend stub to locate the Claude trace dir and count `*.jsonl` files

## What is still mocked / not implemented
- **Export HTML**
- **Save Config**
- **Add Rule / Rule builder UI**
- Loading actual file list + blame data in Blame Viewer
- Trace list populated from real trace metadata
- Persisting selected project + settings between runs

## Next steps (suggested)
1. Add a real backend API layer (Tauri commands) that wraps the existing `ai-blame` library for:
   - scanning traces, listing sessions, summarizing models
   - running blame for a file and returning structured results
2. Add minimal state management in the frontend:
   - selected project dir
   - latest scan results + errors
3. Replace mocked widgets with real data incrementally (start with Trace Explorer list).

# UI Prototype Status / TODO

This `ui/` folder is currently a **static prototype** (mock data + mock interactions).

The goal is to validate layout and navigation before wiring real functionality to the Rust backend.

## What works today
- Navigation between views (Home / Blame Viewer / Trace Explorer / Settings)
- Basic button responsiveness + status updates (no-op actions are called out below)

## TODO (next implementation steps)
- **Open Project**
  - Persist selected project path
  - Use it as the basis for trace discovery (Claude Code: `~/.claude/projects/<encoded-path>`)
- **Refresh/Scan Traces**
  - Call Rust backend to scan trace directory and return:
    - trace count
    - sessions/models summary
    - list of recently touched files
- **Trace Explorer**
  - Replace mock list with real traces + filters
- **Blame Viewer**
  - Load real file list for the project
  - Render blame output from `ai-blame blame` / library API
  - Export HTML
- **Settings**
  - Load/save `.ai-blame.yaml`
  - Validate config and preview effects

See also: `PLAN-UI.md`.


