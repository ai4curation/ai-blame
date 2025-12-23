"""Tests for the extractor module."""

import json
import tempfile
from datetime import datetime, timezone
from pathlib import Path

import pytest

from ai_blame.extractor import parse_trace_file
from ai_blame.models import CurationAction, CurationEvent


@pytest.fixture
def sample_trace_file():
    """Create a sample trace file with version info."""
    records = [
        # Assistant message with tool_use (parent)
        {
            "uuid": "assistant-1",
            "parentUuid": None,
            "type": "assistant",
            "version": "2.0.75",
            "sessionId": "session-123",
            "timestamp": "2025-12-22T17:09:18.478Z",
            "message": {
                "model": "claude-opus-4-5-20251101",
                "role": "assistant",
                "content": [
                    {
                        "type": "tool_use",
                        "id": "toolu_123",
                        "name": "Edit",
                        "input": {"file_path": "/repo/test.py"},
                    }
                ],
            },
        },
        # Tool result (user message with toolUseResult)
        {
            "uuid": "user-1",
            "parentUuid": "assistant-1",
            "type": "user",
            "version": "2.0.75",
            "sessionId": "session-123",
            "timestamp": "2025-12-22T17:09:20.000Z",
            "toolUseResult": {
                "filePath": "/repo/test.py",
                "structuredPatch": {"hunks": []},
                "oldString": "old",
                "newString": "new content",
            },
        },
        # Another edit with different version
        {
            "uuid": "assistant-2",
            "parentUuid": "user-1",
            "type": "assistant",
            "version": "2.1.0",
            "sessionId": "session-123",
            "timestamp": "2025-12-22T18:00:00.000Z",
            "message": {
                "model": "claude-sonnet-4-20250514",
                "role": "assistant",
                "content": [
                    {
                        "type": "tool_use",
                        "id": "toolu_456",
                        "name": "Write",
                        "input": {"file_path": "/repo/new_file.py"},
                    }
                ],
            },
        },
        {
            "uuid": "user-2",
            "parentUuid": "assistant-2",
            "type": "user",
            "version": "2.1.0",
            "sessionId": "session-123",
            "timestamp": "2025-12-22T18:00:01.000Z",
            "toolUseResult": {
                "filePath": "/repo/new_file.py",
                "type": "create",
                "content": "print('hello')",
            },
        },
    ]

    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        for record in records:
            f.write(json.dumps(record) + "\n")
        temp_path = Path(f.name)

    yield temp_path
    temp_path.unlink()


def test_parse_trace_file_extracts_version(sample_trace_file: Path):
    """Test that parse_trace_file extracts agent_tool and agent_version."""
    records = list(parse_trace_file(sample_trace_file))

    assert len(records) == 2

    # First edit
    edit1 = records[0]
    assert edit1.file_path == "/repo/test.py"
    assert edit1.model == "claude-opus-4-5-20251101"
    assert edit1.agent_tool == "claude-code"
    assert edit1.agent_version == "2.0.75"

    # Second edit (create)
    edit2 = records[1]
    assert edit2.file_path == "/repo/new_file.py"
    assert edit2.model == "claude-sonnet-4-20250514"
    assert edit2.agent_tool == "claude-code"
    assert edit2.agent_version == "2.1.0"
    assert edit2.is_create is True


def test_curation_event_to_dict_includes_agent_info():
    """Test that CurationEvent.to_dict includes agent_tool and agent_version."""
    event = CurationEvent(
        timestamp=datetime(2025, 12, 22, 17, 9, 20, tzinfo=timezone.utc),
        model="claude-opus-4-5-20251101",
        action=CurationAction.EDITED,
        agent_tool="claude-code",
        agent_version="2.0.75",
    )

    d = event.to_dict()

    assert d["timestamp"] == "2025-12-22T17:09:20+00:00"
    assert d["model"] == "claude-opus-4-5-20251101"
    assert d["agent_tool"] == "claude-code"
    assert d["agent_version"] == "2.0.75"
    assert d["action"] == "EDITED"


def test_parse_trace_file_handles_missing_version(sample_trace_file: Path):
    """Test graceful handling when version is missing."""
    # Create a trace file without version field
    records = [
        {
            "uuid": "assistant-1",
            "parentUuid": None,
            "type": "assistant",
            "sessionId": "session-123",
            "timestamp": "2025-12-22T17:09:18.478Z",
            "message": {
                "model": "claude-opus-4-5-20251101",
                "role": "assistant",
                "content": [{"type": "tool_use", "id": "toolu_123", "name": "Edit"}],
            },
        },
        {
            "uuid": "user-1",
            "parentUuid": "assistant-1",
            "type": "user",
            "sessionId": "session-123",
            "timestamp": "2025-12-22T17:09:20.000Z",
            "toolUseResult": {
                "filePath": "/repo/test.py",
                "structuredPatch": {"hunks": []},
            },
        },
    ]

    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        for record in records:
            f.write(json.dumps(record) + "\n")
        temp_path = Path(f.name)

    try:
        result = list(parse_trace_file(temp_path))
        assert len(result) == 1
        assert result[0].agent_tool == "claude-code"  # Default value
        assert result[0].agent_version is None
    finally:
        temp_path.unlink()
