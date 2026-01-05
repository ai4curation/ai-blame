# Tauri + Core Lib/CLI Best Practices and Repo Review

## Best Practices (Summary)

- Prefer a single repo with a Cargo workspace when the CLI and Tauri UI share a core Rust library and are released together.
- Keep a small, reusable core crate; have separate CLI and Tauri crates depend on it.
- Tauri’s default layout keeps the Rust app in `src-tauri`; if you want a workspace, add a root `Cargo.toml` with `members = ["src-tauri"]` and keep the Tauri bin target in that crate.
- Avoid relocating the Tauri entrypoint out of `src-tauri` without configuring the CLI; community reports indicate the Tauri CLI expects a bin target in the Tauri crate.
- Use dedicated frontend build output for `distDir`, and wire `beforeDevCommand`/`beforeBuildCommand` when the UI is no longer static.

Sources:
- https://v2.tauri.app/develop/debug/rustrover/
- https://github.com/orgs/tauri-apps/discussions/12630
- https://doc.rust-lang.org/cargo/reference/workspaces.html
- https://v1.tauri.app/v1/api/config
- https://v1.tauri.app/v1/api/cli
- https://github.com/tauri-apps/create-tauri-app

## Findings in This Repo

### High
1) `src-tauri/tauri.conf.json:14` uses an SVG (`../docs/assets/favicon.svg`) for `bundle.icon`.
   - Tauri bundling typically expects PNG/ICO/ICNS. SVG can fail on some platforms.
   - You already have `src-tauri/icons/` which is the conventional place for generated icons.

### Medium
2) `src/lib.rs:2` exposes the `cli` module, and `src-tauri/Cargo.toml:9` depends on `ai-blame`.
   - This pulls `clap` into the GUI build even if the UI only needs the core logic.
   - If you want a leaner GUI, split into a core crate and keep CLI-only deps in a separate crate or behind a feature flag.

3) `src-tauri/tauri.conf.json:3-8` uses `devPath` and `distDir` as `../ui` and leaves build commands empty.
   - Works for the static prototype but will become a packaging risk when the UI gains a real build step.
   - Best practice is `distDir` = compiled build output and to set `beforeDevCommand`/`beforeBuildCommand`.

### Low
4) `src-tauri/Cargo.toml:1-6` duplicates version/edition/authors rather than inheriting from the workspace.
   - Not incorrect, but it weakens centralized version management if you intend single-version releases.

## Idiomatic Structures (Examples)

### Example A: Default Tauri Layout (Frontend in Root, Rust in `src-tauri`)
```
.
├── ui/                  # frontend (or src/, etc.)
├── src-tauri/           # Tauri Rust app
│   ├── Cargo.toml
│   ├── src/main.rs
│   └── tauri.conf.json
└── package.json         # if using Node tooling
```
Notes:
- This matches Tauri’s default layout; the Rust project lives under `src-tauri`.
- IDEs can attach the `src-tauri/Cargo.toml` directly or via a root workspace.

### Example B: Workspace With Core + CLI + Tauri
```
.
├── Cargo.toml           # [workspace] members = ["crates/core", "crates/cli", "src-tauri"]
├── crates/
│   ├── core/            # shared library
│   └── cli/             # CLI binary
├── src-tauri/           # Tauri app (bin)
│   ├── Cargo.toml
│   ├── src/main.rs
│   └── tauri.conf.json
└── ui/                  # frontend
```
Notes:
- Keeps the Tauri bin target in `src-tauri` (works with `tauri dev/build`).
- Avoids pulling CLI-only dependencies into the GUI by depending only on `core`.

### Example C: Rust Frontend (No `src-tauri` Subdir)
```
.
├── Cargo.toml           # root is the Tauri app
├── src/                 # Rust frontend or Rust-only UI
└── tauri.conf.json
```
Notes:
- Tauri docs note that `src-tauri` is created by default; a root Cargo project is used when Rust is also the frontend.

## Proposed Outline (Minimal, Workspace-Friendly)

### Option A: Keep Current Top-Level Crate + Add Dedicated Core Crate

- `crates/core` (library): all reusable logic.
- `crates/cli` (bin): depends on `core` and owns CLI parsing.
- `src-tauri` (bin): depends on `core` and exposes Tauri commands.
- Root `Cargo.toml` as workspace with `members = ["crates/core", "crates/cli", "src-tauri"]`.

### Option B: Convert Current Root Crate Into Core + Add CLI/Tauri Crates

- `crates/cli` (bin) depends on root core crate.
- `src-tauri` (bin) depends on root core crate.
- Root stays as the core library; workspace members add `src-tauri` and `crates/cli`.

### Common Configuration Notes

- Keep the Tauri bin target in `src-tauri` so `tauri dev/build` works without extra CLI overrides.
- Prefer frontend build output in `distDir` and add `beforeDevCommand`/`beforeBuildCommand` for the UI.
- Generate platform icons into `src-tauri/icons/` and reference those in `tauri.conf.json`.

## Assumptions / Questions

- The UI will evolve beyond a static prototype and will require a real build step.
- You want to avoid pulling CLI-only deps into the Tauri binary.

## Additional References

- https://tauri.app/ (docs home)
- https://v2.tauri.app/ (latest docs)
- https://v1.tauri.app/v1/guides/getting-started/setup/ (v1 quick start)
- https://v1.tauri.app/v1/guides/getting-started/setup/integrate (v1 integrate guide)
- https://github.com/tauri-apps/tauri (Tauri repo, shows monorepo/workspace patterns)
