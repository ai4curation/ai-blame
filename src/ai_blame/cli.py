"""CLI for AI Log Miner."""

from pathlib import Path
from typing import Annotated, Optional

import typer

from .config import find_config, get_default_config, load_config
from .extractor import (
    apply_filters,
    convert_to_file_histories,
    extract_edit_history,
)
from .models import FileHistory, FilterConfig, OutputConfig, OutputPolicy
from .updater import apply_rule, preview_update

app = typer.Typer(help="Mine Claude Code traces for curation provenance.")


def resolve_trace_dir(
    trace_dir: Optional[Path],
    target_dir: Optional[Path],
    home_dir: Optional[Path],
) -> Path:
    """
    Resolve the trace directory from the provided options.

    Priority:
    1. If trace_dir is provided, use it directly
    2. Otherwise, compute from target_dir and home_dir:
       - target_dir defaults to cwd
       - home_dir defaults to ~
       - trace_dir = home_dir/.claude/projects/<encoded-target_dir>

    >>> resolve_trace_dir(Path("/explicit/path"), None, None)
    PosixPath('/explicit/path')
    """
    if trace_dir is not None:
        return trace_dir

    # Resolve target directory (the project we're looking at)
    if target_dir is None:
        resolved_target = Path.cwd()
    else:
        resolved_target = target_dir.resolve()

    # Resolve home directory (where .claude lives)
    if home_dir is None:
        resolved_home = Path.home()
    else:
        resolved_home = home_dir.resolve()

    # Encode the target path (replace / with -)
    encoded_path = str(resolved_target).replace("/", "-")

    return resolved_home / ".claude" / "projects" / encoded_path


def print_summary_table(histories: dict[str, FileHistory]) -> None:
    """Print summary table of edits per file."""

    print("\n=== Summary ===")
    print(f"{'File':<50} | {'Edits':>5} | {'First Edit':<20} | {'Last Edit':<20}")
    print("-" * 50 + "-|-" + "-" * 5 + "-|-" + "-" * 20 + "-|-" + "-" * 20)

    for path in sorted(histories.keys()):
        h = histories[path]
        name = Path(path).name[:48]
        count = len(h.events)
        first = h.first_edit.strftime("%Y-%m-%d %H:%M") if h.first_edit else "N/A"
        last = h.last_edit.strftime("%Y-%m-%d %H:%M") if h.last_edit else "N/A"
        print(f"{name:<50} | {count:>5} | {first:<20} | {last:<20}")

    print()


def print_yaml_previews(histories: dict[str, FileHistory], limit: int = 5) -> None:
    """Print YAML previews for each file."""
    for i, (path, history) in enumerate(sorted(histories.items())):
        if i >= limit:
            print(f"\n... and {len(histories) - limit} more files (use --show-all to see all)")
            break
        print(f"\n=== YAML Preview: {path} ===")
        print(preview_update(Path(path), history))


@app.command()
def mine(
    target: Annotated[
        Optional[Path],
        typer.Argument(help="Specific file to filter results (e.g., 'Asthma.yaml')"),
    ] = None,
    trace_dir: Annotated[
        Optional[Path],
        typer.Option(
            "--trace-dir",
            "-t",
            help="Claude trace directory (overrides --dir and --home)",
        ),
    ] = None,
    target_dir: Annotated[
        Optional[Path],
        typer.Option(
            "--dir",
            "-d",
            help="Target project directory (default: cwd). Traces looked up in $home/.claude/projects/<encoded-dir>/",
        ),
    ] = None,
    home_dir: Annotated[
        Optional[Path],
        typer.Option(
            "--home",
            help="Home directory where .claude/ lives (default: ~)",
        ),
    ] = None,
    config_file: Annotated[
        Optional[Path],
        typer.Option(
            "--config",
            "-c",
            help="Config file path (default: auto-find .ai-blame.yaml)",
        ),
    ] = None,
    apply: Annotated[
        bool,
        typer.Option("--apply", "-a", help="Actually apply changes (default is dry-run)"),
    ] = False,
    initial_and_recent: Annotated[
        bool,
        typer.Option(
            "--initial-and-recent",
            "-ir",
            help="Only keep first and last edit per file (recommended to avoid bloat)",
        ),
    ] = False,
    min_change_size: Annotated[
        int,
        typer.Option(
            "--min-change-size",
            "-m",
            help="Skip intermediate edits smaller than N chars",
        ),
    ] = 0,
    show_all: Annotated[
        bool,
        typer.Option("--show-all", help="Show all YAML previews (not just first 5)"),
    ] = False,
    file_pattern: Annotated[
        str,
        typer.Option("--pattern", "-p", help="Filter files by path pattern (e.g., 'kb/' or '.yaml')"),
    ] = "",
) -> None:
    """
    Mine Claude Code traces and add curation history to YAML files.

    Extracts successful Edit/Write operations from Claude Code trace logs and
    appends a edit_history section to each affected file.

    \b
    TRACE DIRECTORY RESOLUTION:
    The trace directory is determined by (in order of priority):
      1. --trace-dir: Use this exact path
      2. --dir + --home: Compute as $home/.claude/projects/<encoded-dir>/
      3. Default: ~/.claude/projects/<encoded-cwd>/

    \b
    EXAMPLES:
        # Mine traces for current directory
        ai-blame mine

        # Mine traces for a specific project directory
        ai-blame mine --dir /path/to/project

        # Mine traces from a different home (e.g., test data)
        ai-blame mine --dir tests/data --home tests/data

        # Use explicit trace directory
        ai-blame mine -t ~/.claude/projects/-other-repo/
    """
    trace_dir = resolve_trace_dir(trace_dir, target_dir, home_dir)

    if not trace_dir.exists():
        typer.echo(f"Trace directory not found: {trace_dir}")
        raise typer.Exit(1)

    typer.echo(f"Scanning traces in: {trace_dir}")

    # Load output config
    output_config: OutputConfig
    if config_file:
        if not config_file.exists():
            typer.echo(f"Config file not found: {config_file}")
            raise typer.Exit(1)
        output_config = load_config(config_file)
        typer.echo(f"Using config: {config_file}")
    else:
        found_config = find_config()
        if found_config:
            output_config = load_config(found_config)
            typer.echo(f"Using config: {found_config}")
        else:
            output_config = get_default_config()

    # Build filter config
    filter_config = FilterConfig(
        initial_and_recent_only=initial_and_recent,
        min_change_size=min_change_size,
        file_pattern=file_pattern,
    )

    # Extract edit history
    edits_by_file = extract_edit_history(trace_dir, filter_config)

    if not edits_by_file:
        typer.echo("No edits found matching criteria.")
        raise typer.Exit(0)

    # Apply filters
    edits_by_file = apply_filters(edits_by_file, filter_config)

    if not edits_by_file:
        typer.echo("No edits remaining after filtering.")
        raise typer.Exit(0)

    # Convert to file histories
    histories = convert_to_file_histories(edits_by_file)

    # Filter to specific target if provided
    if target:
        target_str = str(target)
        histories = {k: v for k, v in histories.items() if target_str in k}
        if not histories:
            typer.echo(f"No history found for: {target}")
            raise typer.Exit(1)

    # Warn if too many entries
    for path, history in histories.items():
        if len(history.events) > 20:
            typer.echo(
                f"Warning: {path} has {len(history.events)} curation events. "
                "Consider using --initial-and-recent to reduce."
            )

    # Print summary table
    print_summary_table(histories)

    # Print YAML previews
    limit = len(histories) if show_all else 5
    print_yaml_previews(histories, limit=limit)

    if not apply:
        typer.echo("\n[DRY RUN] No files modified. Use --apply to actually update files.")
        return

    # Apply changes
    typer.echo("\nApplying changes...")
    for rel_path, history in histories.items():
        # Find the actual file
        file_path = Path(rel_path)
        if not file_path.exists():
            # Try from current working directory
            file_path = Path.cwd() / rel_path
        if not file_path.exists():
            typer.echo(f"  Skipping (not found): {rel_path}")
            continue

        # Get rule for this file
        rule = output_config.get_rule_for_file(rel_path)
        if rule is None:
            typer.echo(f"  Skipping (no matching rule): {rel_path}")
            continue

        if rule.policy == OutputPolicy.SKIP:
            typer.echo(f"  Skipped (policy=skip): {rel_path}")
            continue

        success, msg = apply_rule(file_path, history, rule, dry_run=False)
        if success:
            typer.echo(f"  {msg}")
        else:
            typer.echo(f"  Failed: {msg}")


@app.command()
def stats(
    trace_dir: Annotated[
        Optional[Path],
        typer.Option(
            "--trace-dir",
            "-t",
            help="Claude trace directory (overrides --dir and --home)",
        ),
    ] = None,
    target_dir: Annotated[
        Optional[Path],
        typer.Option(
            "--dir",
            "-d",
            help="Target project directory (default: cwd). Traces looked up in $home/.claude/projects/<encoded-dir>/",
        ),
    ] = None,
    home_dir: Annotated[
        Optional[Path],
        typer.Option(
            "--home",
            help="Home directory where .claude/ lives (default: ~)",
        ),
    ] = None,
    file_pattern: Annotated[
        str,
        typer.Option("--pattern", "-p", help="Filter files by path pattern (e.g., 'kb/' or '.yaml')"),
    ] = "",
) -> None:
    """
    Show statistics about available traces.

    \b
    EXAMPLES:
        # Stats for current directory
        ai-blame stats

        # Stats for a specific project
        ai-blame stats --dir /path/to/project

        # Stats from test data
        ai-blame stats --dir tests/data --home tests/data
    """
    trace_dir = resolve_trace_dir(trace_dir, target_dir, home_dir)

    if not trace_dir.exists():
        typer.echo(f"Trace directory not found: {trace_dir}")
        raise typer.Exit(1)

    typer.echo(f"Trace directory: {trace_dir}")

    # Count trace files
    jsonl_files = list(trace_dir.glob("*.jsonl"))
    typer.echo(f"Trace files: {len(jsonl_files)}")

    # Count agent traces
    agent_files = [f for f in jsonl_files if f.name.startswith("agent-")]
    session_files = [f for f in jsonl_files if not f.name.startswith("agent-")]
    typer.echo(f"  Session traces: {len(session_files)}")
    typer.echo(f"  Agent traces: {len(agent_files)}")

    # Extract and summarize edits
    config = FilterConfig(file_pattern=file_pattern)
    edits_by_file = extract_edit_history(trace_dir, config)

    total_edits = sum(len(edits) for edits in edits_by_file.values())
    pattern_desc = f"matching '{file_pattern}'" if file_pattern else "(all files)"
    typer.echo(f"\nFiles with edits {pattern_desc}: {len(edits_by_file)}")
    typer.echo(f"Total successful edits: {total_edits}")


def main() -> None:
    """Entry point."""
    app()


if __name__ == "__main__":
    main()
