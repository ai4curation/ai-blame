"""Extract edit history from Claude Code trace files."""

import json
import os
from collections import defaultdict
from datetime import datetime
from pathlib import Path
from typing import Iterator, Optional

from .models import CurationAction, CurationEvent, EditRecord, FileHistory, FilterConfig


def get_default_trace_dir() -> Path:
    """Get the default trace directory for the current project."""
    # Convert cwd to Claude's trace directory format
    cwd = os.getcwd()
    encoded_path = cwd.replace("/", "-")
    return Path.home() / ".claude" / "projects" / encoded_path


def normalize_path(abs_path: str, repo_root: Optional[str] = None) -> str:
    """Convert absolute path to repo-relative path."""
    if repo_root is None:
        repo_root = os.getcwd()
    if abs_path.startswith(repo_root):
        return abs_path[len(repo_root) :].lstrip("/")
    return abs_path


def is_successful_edit(record: dict) -> bool:
    """Check if this record is a tool_result for a successful Edit/Write."""
    if record.get("type") != "user":
        return False

    tool_result = record.get("toolUseResult")
    if not tool_result:
        return False

    # toolUseResult can sometimes be a string (error message)
    if not isinstance(tool_result, dict):
        return False

    # Check for error indicators
    if tool_result.get("is_error"):
        return False
    if tool_result.get("error"):
        return False
    if tool_result.get("code", 200) >= 400:
        return False

    # Must have a file path
    file_path = tool_result.get("filePath", "")
    if not file_path:
        return False

    # Check if it's a create or edit
    has_patch = "structuredPatch" in tool_result
    is_create = tool_result.get("type") == "create"

    return has_patch or is_create


def calculate_change_size(tool_result: dict) -> int:
    """Calculate approximate change size in characters."""
    if tool_result.get("type") == "create":
        # For creates, use content length
        content = tool_result.get("content", "")
        return len(content)

    # For edits, calculate difference
    old_string = tool_result.get("oldString", "")
    new_string = tool_result.get("newString", "")
    return abs(len(new_string) - len(old_string)) + max(len(old_string), len(new_string))


def parse_trace_file(trace_path: Path, file_pattern: str = "") -> Iterator[EditRecord]:
    """Parse a single trace file and yield successful edit records."""
    # Build an index of messages by UUID for parent lookups
    messages_by_uuid: dict[str, dict] = {}

    with open(trace_path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            # Skip non-JSON lines (encrypted thinking blocks, etc.)
            if not line.startswith("{"):
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            uuid = record.get("uuid")
            if uuid:
                messages_by_uuid[uuid] = record

    # Now process looking for successful edits
    for uuid, record in messages_by_uuid.items():
        if not is_successful_edit(record):
            continue

        tool_result = record.get("toolUseResult", {})
        file_path = tool_result.get("filePath", "")

        # Apply file pattern filter
        if file_pattern and file_pattern not in file_path:
            continue

        # Get model from parent message (assistant message with tool_use)
        parent_uuid = record.get("parentUuid")
        model = None
        if parent_uuid and parent_uuid in messages_by_uuid:
            parent = messages_by_uuid[parent_uuid]
            model = parent.get("message", {}).get("model")

        # Get agent version from the record
        agent_version = record.get("version")

        # Parse timestamp
        ts_str = record.get("timestamp", "")
        timestamp = datetime.fromisoformat(ts_str.replace("Z", "+00:00")) if ts_str else datetime.now()

        yield EditRecord(
            file_path=file_path,
            timestamp=timestamp,
            model=model or "unknown",
            session_id=record.get("sessionId", "unknown"),
            is_create=tool_result.get("type") == "create",
            change_size=calculate_change_size(tool_result),
            agent_version=agent_version,
        )


def extract_edit_history(
    trace_dir: Optional[Path] = None,
    config: Optional[FilterConfig] = None,
) -> dict[str, list[EditRecord]]:
    """Extract all edit records from trace directory, grouped by file."""
    if trace_dir is None:
        trace_dir = get_default_trace_dir()
    if config is None:
        config = FilterConfig()

    file_pattern = config.file_pattern or ""
    edits_by_file: dict[str, list[EditRecord]] = defaultdict(list)

    # Process all JSONL files in trace directory
    if not trace_dir.exists():
        return dict(edits_by_file)

    for trace_file in trace_dir.glob("*.jsonl"):
        for edit in parse_trace_file(trace_file, file_pattern):
            # Apply time filters
            if config.since and edit.timestamp < config.since:
                continue
            if config.until and edit.timestamp > config.until:
                continue

            edits_by_file[edit.file_path].append(edit)

    # Sort by timestamp within each file
    for file_path in edits_by_file:
        edits_by_file[file_path].sort(key=lambda e: e.timestamp)

    return dict(edits_by_file)


def apply_filters(
    edits_by_file: dict[str, list[EditRecord]],
    config: FilterConfig,
) -> dict[str, list[EditRecord]]:
    """Apply filtering to edit records."""
    filtered: dict[str, list[EditRecord]] = {}

    for file_path, edits in edits_by_file.items():
        if not edits:
            continue

        # Apply size filter
        if config.min_change_size > 0:
            edits = [e for e in edits if e.change_size >= config.min_change_size]

        if not edits:
            continue

        # Apply initial_and_recent_only filter
        if config.initial_and_recent_only and len(edits) > 2:
            # Keep first and last only
            edits = [edits[0], edits[-1]]

        filtered[file_path] = edits

    return filtered


def convert_to_file_histories(
    edits_by_file: dict[str, list[EditRecord]],
    repo_root: Optional[str] = None,
) -> dict[str, FileHistory]:
    """Convert edit records to FileHistory objects with CurationEvents."""
    if repo_root is None:
        repo_root = os.getcwd()

    histories: dict[str, FileHistory] = {}

    for abs_path, edits in edits_by_file.items():
        rel_path = normalize_path(abs_path, repo_root)

        events = []
        for i, edit in enumerate(edits):
            # First edit is CREATED, rest are EDITED
            # But only if it's actually a create operation
            if i == 0 and edit.is_create:
                action = CurationAction.CREATED
            else:
                action = CurationAction.EDITED

            events.append(
                CurationEvent(
                    timestamp=edit.timestamp,
                    model=edit.model,
                    action=action,
                    agent_tool=edit.agent_tool,
                    agent_version=edit.agent_version,
                )
            )

        histories[rel_path] = FileHistory(file_path=rel_path, events=events)

    return histories
