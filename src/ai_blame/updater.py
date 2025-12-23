"""Update files with curation history."""

import json
from pathlib import Path

import yaml

from .config import resolve_sidecar_path
from .models import CommentSyntax, FileHistory, FileRule, OutputPolicy


def generate_curation_yaml(history: FileHistory) -> str:
    """Generate YAML string for edit_history section."""
    events_data = [e.to_dict() for e in history.events]
    return yaml.dump({"edit_history": events_data}, default_flow_style=False, sort_keys=False)


def generate_curation_json(history: FileHistory) -> str:
    """Generate JSON string for edit_history section."""
    events_data = [e.to_dict() for e in history.events]
    return json.dumps({"edit_history": events_data}, indent=2, default=str)


def append_yaml(
    file_path: Path,
    history: FileHistory,
    dry_run: bool = True,
) -> tuple[bool, str]:
    """
    Append edit_history to a YAML file.

    Replaces existing edit_history if present.
    """
    if not file_path.exists():
        return False, f"File not found: {file_path}"

    content = file_path.read_text()

    # Check if edit_history already exists and remove it
    if "edit_history:" in content:
        lines = content.split("\n")
        new_lines = []
        in_curation = False
        for line in lines:
            if line.startswith("edit_history:"):
                in_curation = True
                continue
            if in_curation:
                # Check if this is a new top-level key (not indented and not empty)
                if line and not line.startswith(" ") and not line.startswith("-"):
                    in_curation = False
                    new_lines.append(line)
                continue
            new_lines.append(line)
        content = "\n".join(new_lines)

    # Generate new curation history
    curation_yaml = generate_curation_yaml(history)

    # Ensure content ends with newline
    if not content.endswith("\n"):
        content += "\n"

    # Add blank line before edit_history if not already there
    if not content.endswith("\n\n"):
        content += "\n"

    new_content = content + curation_yaml

    if dry_run:
        return True, new_content

    file_path.write_text(new_content)
    return True, f"Updated: {file_path}"


def append_json(
    file_path: Path,
    history: FileHistory,
    dry_run: bool = True,
) -> tuple[bool, str]:
    """
    Append edit_history to a JSON file.

    Adds or replaces the edit_history key in the JSON object.
    """
    if not file_path.exists():
        return False, f"File not found: {file_path}"

    content = file_path.read_text()

    data = json.loads(content)

    # Add/replace edit_history
    events_data = [e.to_dict() for e in history.events]
    data["edit_history"] = events_data

    new_content = json.dumps(data, indent=2, default=str) + "\n"

    if dry_run:
        return True, new_content

    file_path.write_text(new_content)
    return True, f"Updated: {file_path}"


def write_sidecar(
    file_path: Path,
    history: FileHistory,
    sidecar_pattern: str,
    dry_run: bool = True,
) -> tuple[bool, str]:
    """
    Write curation history to a sidecar file.

    If sidecar exists, merges events (deduplicates by timestamp).

    >>> from ai_blame.models import CurationEvent
    >>> from datetime import datetime, timezone
    >>> import tempfile
    >>> with tempfile.TemporaryDirectory() as tmpdir:
    ...     source = Path(tmpdir) / "foo.py"
    ...     _ = source.write_text("print('hello')")
    ...     history = FileHistory(
    ...         file_path=str(source),
    ...         events=[CurationEvent(timestamp=datetime(2025, 1, 1, tzinfo=timezone.utc), model="test")]
    ...     )
    ...     success, msg = write_sidecar(source, history, "{stem}.history.yaml", dry_run=False)
    ...     sidecar = Path(tmpdir) / "foo.history.yaml"
    ...     sidecar.exists()
    True
    """
    sidecar_path = resolve_sidecar_path(file_path, sidecar_pattern)

    # Merge with existing sidecar if it exists
    existing_events: list[dict] = []
    if sidecar_path.exists():
        existing_content = sidecar_path.read_text()
        existing_data = yaml.safe_load(existing_content)
        if existing_data and "edit_history" in existing_data:
            existing_events = existing_data["edit_history"]

    # Convert new events to dicts
    new_events = [e.to_dict() for e in history.events]

    # Merge and deduplicate by timestamp
    all_events = existing_events + new_events
    seen_timestamps: set[str] = set()
    merged_events = []
    for event in all_events:
        ts = event.get("timestamp", "")
        if ts not in seen_timestamps:
            seen_timestamps.add(ts)
            merged_events.append(event)

    # Sort by timestamp
    merged_events.sort(key=lambda e: e.get("timestamp", ""))

    # Include source file reference
    sidecar_data = {
        "source_file": file_path.name,
        "edit_history": merged_events,
    }

    new_content = yaml.dump(sidecar_data, default_flow_style=False, sort_keys=False)

    if dry_run:
        return True, f"Would write sidecar: {sidecar_path}\n{new_content}"

    # Create parent directories if needed
    sidecar_path.parent.mkdir(parents=True, exist_ok=True)
    sidecar_path.write_text(new_content)
    return True, f"Wrote sidecar: {sidecar_path}"


def write_comment(
    file_path: Path,
    history: FileHistory,
    syntax: CommentSyntax,
    dry_run: bool = True,
) -> tuple[bool, str]:
    """
    Embed curation history as a comment block at the end of a file.

    Replaces existing edit_history comment block if present.
    """
    if not file_path.exists():
        return False, f"File not found: {file_path}"

    content = file_path.read_text()

    # Generate the curation history YAML (without the outer key)
    events_data = [e.to_dict() for e in history.events]
    history_yaml = yaml.dump(events_data, default_flow_style=False, sort_keys=False)

    # Format as comment block based on syntax
    if syntax == CommentSyntax.HASH:
        marker_start = "# --- edit_history ---"
        marker_end = "# --- end edit_history ---"
        commented = "\n".join(f"# {line}" if line else "#" for line in history_yaml.split("\n"))
        comment_block = f"{marker_start}\n{commented}\n{marker_end}\n"
    elif syntax == CommentSyntax.SLASH:
        marker_start = "// --- edit_history ---"
        marker_end = "// --- end edit_history ---"
        commented = "\n".join(f"// {line}" if line else "//" for line in history_yaml.split("\n"))
        comment_block = f"{marker_start}\n{commented}\n{marker_end}\n"
    elif syntax == CommentSyntax.HTML:
        marker_start = "<!-- edit_history"
        marker_end = "-->"
        comment_block = f"{marker_start}\n{history_yaml}{marker_end}\n"
    else:
        return False, f"Unknown comment syntax: {syntax}"

    # Remove existing edit_history comment block if present
    if "edit_history" in content:
        if syntax == CommentSyntax.HTML:
            # Remove <!-- edit_history ... -->
            import re
            content = re.sub(r"<!--\s*edit_history.*?-->\n?", "", content, flags=re.DOTALL)
        else:
            # Remove marker-based blocks
            lines = content.split("\n")
            new_lines = []
            in_block = False
            for line in lines:
                if "--- edit_history ---" in line:
                    in_block = True
                    continue
                if "--- end edit_history ---" in line:
                    in_block = False
                    continue
                if not in_block:
                    new_lines.append(line)
            content = "\n".join(new_lines)

    # Ensure content ends with newline
    if content and not content.endswith("\n"):
        content += "\n"

    new_content = content + "\n" + comment_block

    if dry_run:
        return True, new_content

    file_path.write_text(new_content)
    return True, f"Updated: {file_path}"


def apply_rule(
    file_path: Path,
    history: FileHistory,
    rule: FileRule,
    dry_run: bool = True,
) -> tuple[bool, str]:
    """
    Apply a FileRule to write curation history.

    Dispatches to the appropriate writer based on policy.
    """
    if rule.policy == OutputPolicy.SKIP:
        return True, f"Skipped (policy=skip): {file_path}"

    if rule.policy == OutputPolicy.APPEND:
        if rule.format == "json":
            return append_json(file_path, history, dry_run)
        return append_yaml(file_path, history, dry_run)

    if rule.policy == OutputPolicy.SIDECAR:
        pattern = rule.sidecar_pattern or "{stem}.history.yaml"
        return write_sidecar(file_path, history, pattern, dry_run)

    if rule.policy == OutputPolicy.COMMENT:
        if rule.comment_syntax is None:
            return False, f"Comment policy requires comment_syntax for {file_path}"
        return write_comment(file_path, history, rule.comment_syntax, dry_run)

    return False, f"Unknown policy: {rule.policy}"


# --- Backward compatibility ---


def update_yaml_file(
    file_path: Path,
    history: FileHistory,
    dry_run: bool = True,
) -> tuple[bool, str]:
    """Update a YAML file with curation history (backward compatibility wrapper)."""
    return append_yaml(file_path, history, dry_run)


def preview_update(file_path: Path, history: FileHistory) -> str:
    """Generate a preview of the edit_history section for a file."""
    return generate_curation_yaml(history)
