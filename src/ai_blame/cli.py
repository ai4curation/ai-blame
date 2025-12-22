"""CLI for AI Log Miner."""

from datetime import datetime
from pathlib import Path
from typing import Annotated, Optional

import typer

from .extractor import (
    apply_filters,
    convert_to_file_histories,
    extract_edit_history,
    get_default_trace_dir,
)
from .models import FilterConfig
from .updater import preview_update, update_yaml_file

app = typer.Typer(help="Mine Claude Code traces for curation provenance.")


def print_summary_table(histories: dict[str, "FileHistory"]) -> None:
    """Print summary table of edits per file."""
    from .models import FileHistory

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


def print_yaml_previews(histories: dict[str, "FileHistory"], limit: int = 5) -> None:
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
            help="Claude trace directory. Default: ~/.claude/projects/<cwd-encoded>/",
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
        typer.Option("--pattern", "-p", help="Filter files by path pattern"),
    ] = "kb/disorders/",
) -> None:
    """
    Mine Claude Code traces and add curation history to YAML files.

    Extracts successful Edit/Write operations from Claude Code trace logs and
    appends a curation_history section to each affected file.

    \b
    TRACE DIRECTORY DETECTION:
    If --trace-dir is not specified, the tool looks for traces in:
      ~/.claude/projects/<encoded-cwd>/
    where <encoded-cwd> is your current working directory with '/' replaced by '-'.
    For example, /Users/cjm/repos/dismech becomes:
      ~/.claude/projects/-Users-cjm-repos-dismech/

    \b
    EXAMPLES:
        # Dry run - see what would be added (default)
        uv run python -m ai_log_miner mine

        # Only first + last edit per file (recommended)
        uv run python -m ai_log_miner mine --initial-and-recent

        # Actually apply changes
        uv run python -m ai_log_miner mine --apply --initial-and-recent

        # Filter to a specific file
        uv run python -m ai_log_miner mine Asthma.yaml

        # Use a different trace directory
        uv run python -m ai_log_miner mine -t ~/.claude/projects/-other-repo/
    """
    if trace_dir is None:
        trace_dir = get_default_trace_dir()

    if not trace_dir.exists():
        typer.echo(f"Trace directory not found: {trace_dir}")
        raise typer.Exit(1)

    typer.echo(f"Scanning traces in: {trace_dir}")

    # Build filter config
    config = FilterConfig(
        initial_and_recent_only=initial_and_recent,
        min_change_size=min_change_size,
        file_pattern=file_pattern,
    )

    # Extract edit history
    edits_by_file = extract_edit_history(trace_dir, config)

    if not edits_by_file:
        typer.echo("No edits found matching criteria.")
        raise typer.Exit(0)

    # Apply filters
    edits_by_file = apply_filters(edits_by_file, config)

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

        success, msg = update_yaml_file(file_path, history, dry_run=False)
        if success:
            typer.echo(f"  Updated: {rel_path}")
        else:
            typer.echo(f"  Failed: {msg}")


@app.command()
def stats(
    trace_dir: Annotated[
        Optional[Path],
        typer.Option(
            "--trace-dir",
            "-t",
            help="Claude trace directory. Default: ~/.claude/projects/<cwd-encoded>/",
        ),
    ] = None,
    file_pattern: Annotated[
        str,
        typer.Option("--pattern", "-p", help="Filter files by path pattern"),
    ] = "kb/disorders/",
) -> None:
    """
    Show statistics about available traces.

    \b
    EXAMPLES:
        uv run python -m ai_log_miner stats
        uv run python -m ai_log_miner stats --pattern "kb/"
    """
    if trace_dir is None:
        trace_dir = get_default_trace_dir()

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
    typer.echo(f"\nFiles with edits matching '{file_pattern}': {len(edits_by_file)}")
    typer.echo(f"Total successful edits: {total_edits}")


def main() -> None:
    """Entry point."""
    app()


if __name__ == "__main__":
    main()
