"""Tests for updater module - writers for different output policies."""

import json
import tempfile
from datetime import datetime, timezone
from pathlib import Path

import pytest
import yaml

from ai_blame.models import (
    CommentSyntax,
    CurationAction,
    CurationEvent,
    FileHistory,
    FileRule,
    OutputPolicy,
)
from ai_blame.updater import (
    append_json,
    append_yaml,
    apply_rule,
    write_comment,
    write_sidecar,
)


@pytest.fixture
def sample_history() -> FileHistory:
    """Create a sample FileHistory for testing."""
    return FileHistory(
        file_path="/test/foo.py",
        events=[
            CurationEvent(
                timestamp=datetime(2025, 1, 1, 12, 0, 0, tzinfo=timezone.utc),
                model="claude-opus-4-5",
                action=CurationAction.CREATED,
                agent_tool="claude-code",
                agent_version="2.0.75",
            ),
            CurationEvent(
                timestamp=datetime(2025, 1, 2, 12, 0, 0, tzinfo=timezone.utc),
                model="claude-opus-4-5",
                action=CurationAction.EDITED,
                agent_tool="claude-code",
                agent_version="2.0.76",
            ),
        ],
    )


class TestAppendYaml:
    """Tests for append_yaml writer."""

    def test_append_to_yaml_file(self, sample_history: FileHistory):
        """Test appending curation history to a YAML file."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write("name: test\nvalue: 42\n")
            f.flush()
            path = Path(f.name)

        success, content = append_yaml(path, sample_history, dry_run=True)

        assert success
        assert "edit_history:" in content
        assert "claude-opus-4-5" in content
        assert "agent_tool: claude-code" in content

        path.unlink()

    def test_append_replaces_existing_history(self, sample_history: FileHistory):
        """Test that existing edit_history is replaced."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write("name: test\nedit_history:\n  - old: data\n")
            f.flush()
            path = Path(f.name)

        success, content = append_yaml(path, sample_history, dry_run=True)

        assert success
        assert "old: data" not in content
        assert "claude-opus-4-5" in content

        path.unlink()

    def test_append_writes_file(self, sample_history: FileHistory):
        """Test that dry_run=False actually writes the file."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write("name: test\n")
            f.flush()
            path = Path(f.name)

        success, msg = append_yaml(path, sample_history, dry_run=False)

        assert success
        content = path.read_text()
        assert "edit_history:" in content

        path.unlink()


class TestAppendJson:
    """Tests for append_json writer."""

    def test_append_to_json_file(self, sample_history: FileHistory):
        """Test appending curation history to a JSON file."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            json.dump({"name": "test", "value": 42}, f)
            f.flush()
            path = Path(f.name)

        success, content = append_json(path, sample_history, dry_run=True)

        assert success
        data = json.loads(content)
        assert "edit_history" in data
        assert len(data["edit_history"]) == 2

        path.unlink()

    def test_append_replaces_existing_json_history(self, sample_history: FileHistory):
        """Test that existing edit_history is replaced in JSON."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            json.dump({"name": "test", "edit_history": [{"old": "data"}]}, f)
            f.flush()
            path = Path(f.name)

        success, content = append_json(path, sample_history, dry_run=True)

        assert success
        data = json.loads(content)
        assert data["edit_history"][0].get("old") is None
        assert data["edit_history"][0]["model"] == "claude-opus-4-5"

        path.unlink()


class TestWriteSidecar:
    """Tests for write_sidecar writer."""

    def test_write_sidecar_creates_file(self, sample_history: FileHistory):
        """Test that sidecar file is created."""
        with tempfile.TemporaryDirectory() as tmpdir:
            source = Path(tmpdir) / "foo.py"
            source.write_text("print('hello')")

            success, msg = write_sidecar(
                source, sample_history, "{stem}.history.yaml", dry_run=False
            )

            assert success
            sidecar = Path(tmpdir) / "foo.history.yaml"
            assert sidecar.exists()

            content = yaml.safe_load(sidecar.read_text())
            assert content["source_file"] == "foo.py"
            assert len(content["edit_history"]) == 2

    def test_write_sidecar_merges_existing(self, sample_history: FileHistory):
        """Test that sidecar merges with existing history."""
        with tempfile.TemporaryDirectory() as tmpdir:
            source = Path(tmpdir) / "foo.py"
            source.write_text("print('hello')")

            sidecar = Path(tmpdir) / "foo.history.yaml"
            existing = {
                "source_file": "foo.py",
                "edit_history": [
                    {"timestamp": "2024-12-01T12:00:00+00:00", "model": "old-model"}
                ],
            }
            sidecar.write_text(yaml.dump(existing))

            success, msg = write_sidecar(
                source, sample_history, "{stem}.history.yaml", dry_run=False
            )

            assert success
            content = yaml.safe_load(sidecar.read_text())
            # Should have 3 events: 1 existing + 2 new
            assert len(content["edit_history"]) == 3

    def test_write_sidecar_deduplicates(self, sample_history: FileHistory):
        """Test that sidecar deduplicates by timestamp."""
        with tempfile.TemporaryDirectory() as tmpdir:
            source = Path(tmpdir) / "foo.py"
            source.write_text("print('hello')")

            sidecar = Path(tmpdir) / "foo.history.yaml"
            # Add event with same timestamp as one in sample_history
            existing = {
                "source_file": "foo.py",
                "edit_history": [
                    {"timestamp": "2025-01-01T12:00:00+00:00", "model": "duplicate"}
                ],
            }
            sidecar.write_text(yaml.dump(existing))

            success, msg = write_sidecar(
                source, sample_history, "{stem}.history.yaml", dry_run=False
            )

            assert success
            content = yaml.safe_load(sidecar.read_text())
            # Should have 2 events (duplicate removed)
            assert len(content["edit_history"]) == 2


class TestWriteComment:
    """Tests for write_comment writer."""

    def test_write_hash_comment(self, sample_history: FileHistory):
        """Test writing history as hash comments."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False) as f:
            f.write("print('hello')\n")
            f.flush()
            path = Path(f.name)

        success, content = write_comment(
            path, sample_history, CommentSyntax.HASH, dry_run=True
        )

        assert success
        assert "# --- edit_history ---" in content
        assert "# --- end edit_history ---" in content
        assert "# - timestamp:" in content

        path.unlink()

    def test_write_slash_comment(self, sample_history: FileHistory):
        """Test writing history as slash comments."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".js", delete=False) as f:
            f.write("console.log('hello');\n")
            f.flush()
            path = Path(f.name)

        success, content = write_comment(
            path, sample_history, CommentSyntax.SLASH, dry_run=True
        )

        assert success
        assert "// --- edit_history ---" in content
        assert "// --- end edit_history ---" in content

        path.unlink()

    def test_write_html_comment(self, sample_history: FileHistory):
        """Test writing history as HTML comments."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".md", delete=False) as f:
            f.write("# Hello\n\nSome content.\n")
            f.flush()
            path = Path(f.name)

        success, content = write_comment(
            path, sample_history, CommentSyntax.HTML, dry_run=True
        )

        assert success
        assert "<!-- edit_history" in content
        assert "-->" in content

        path.unlink()

    def test_write_comment_replaces_existing(self, sample_history: FileHistory):
        """Test that existing comment block is replaced."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False) as f:
            f.write("print('hello')\n\n# --- edit_history ---\n# old data\n# --- end edit_history ---\n")
            f.flush()
            path = Path(f.name)

        success, content = write_comment(
            path, sample_history, CommentSyntax.HASH, dry_run=True
        )

        assert success
        assert "old data" not in content
        assert content.count("--- edit_history ---") == 1

        path.unlink()


class TestApplyRule:
    """Tests for apply_rule dispatcher."""

    def test_apply_skip_policy(self, sample_history: FileHistory):
        """Test that skip policy returns success without writing."""
        rule = FileRule(pattern="*", policy=OutputPolicy.SKIP)

        success, msg = apply_rule(Path("/any/file.txt"), sample_history, rule)

        assert success
        assert "Skipped" in msg

    def test_apply_append_yaml_policy(self, sample_history: FileHistory):
        """Test apply_rule with append policy for YAML."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write("name: test\n")
            f.flush()
            path = Path(f.name)

        rule = FileRule(pattern="*.yaml", policy=OutputPolicy.APPEND)
        success, content = apply_rule(path, sample_history, rule, dry_run=True)

        assert success
        assert "edit_history:" in content

        path.unlink()

    def test_apply_append_json_policy(self, sample_history: FileHistory):
        """Test apply_rule with append policy for JSON."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            json.dump({"name": "test"}, f)
            f.flush()
            path = Path(f.name)

        rule = FileRule(pattern="*.json", policy=OutputPolicy.APPEND, format="json")
        success, content = apply_rule(path, sample_history, rule, dry_run=True)

        assert success
        data = json.loads(content)
        assert "edit_history" in data

        path.unlink()

    def test_apply_sidecar_policy(self, sample_history: FileHistory):
        """Test apply_rule with sidecar policy."""
        with tempfile.TemporaryDirectory() as tmpdir:
            source = Path(tmpdir) / "foo.py"
            source.write_text("print('hello')")

            rule = FileRule(
                pattern="*.py",
                policy=OutputPolicy.SIDECAR,
                sidecar_pattern="{stem}.history.yaml",
            )
            success, msg = apply_rule(source, sample_history, rule, dry_run=False)

            assert success
            assert Path(tmpdir, "foo.history.yaml").exists()

    def test_apply_comment_policy(self, sample_history: FileHistory):
        """Test apply_rule with comment policy."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False) as f:
            f.write("print('hello')\n")
            f.flush()
            path = Path(f.name)

        rule = FileRule(
            pattern="*.py", policy=OutputPolicy.COMMENT, comment_syntax=CommentSyntax.HASH
        )
        success, content = apply_rule(path, sample_history, rule, dry_run=True)

        assert success
        assert "# --- edit_history ---" in content

        path.unlink()

    def test_apply_comment_policy_missing_syntax(self, sample_history: FileHistory):
        """Test that comment policy fails without comment_syntax."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False) as f:
            f.write("print('hello')\n")
            f.flush()
            path = Path(f.name)

        rule = FileRule(pattern="*.py", policy=OutputPolicy.COMMENT)  # No comment_syntax
        success, msg = apply_rule(path, sample_history, rule, dry_run=True)

        assert not success
        assert "requires comment_syntax" in msg

        path.unlink()
