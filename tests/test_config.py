"""Tests for config loading and pattern matching."""

import tempfile
from pathlib import Path


from ai_blame.config import (
    find_config,
    get_default_config,
    load_config,
    resolve_sidecar_path,
)
from ai_blame.models import CommentSyntax, OutputPolicy


class TestGetDefaultConfig:
    """Tests for get_default_config."""

    def test_yaml_files_append(self):
        """YAML files should use append policy by default."""
        config = get_default_config()
        rule = config.get_rule_for_file("foo.yaml")
        assert rule is not None
        assert rule.policy == OutputPolicy.APPEND

    def test_yml_files_append(self):
        """YML files should use append policy by default."""
        config = get_default_config()
        rule = config.get_rule_for_file("foo.yml")
        assert rule is not None
        assert rule.policy == OutputPolicy.APPEND

    def test_json_files_append_json(self):
        """JSON files should use append policy with json format."""
        config = get_default_config()
        rule = config.get_rule_for_file("data.json")
        assert rule is not None
        assert rule.policy == OutputPolicy.APPEND
        assert rule.format == "json"

    def test_other_files_sidecar(self):
        """Other files should use sidecar policy by default."""
        config = get_default_config()
        for filename in ["script.py", "readme.md", "code.ts", "main.go"]:
            rule = config.get_rule_for_file(filename)
            assert rule is not None
            assert rule.policy == OutputPolicy.SIDECAR
            assert rule.sidecar_pattern == "{stem}.history.yaml"


class TestLoadConfig:
    """Tests for load_config."""

    def test_load_simple_config(self):
        """Test loading a simple config file."""
        config_content = """
defaults:
  policy: sidecar
  sidecar_pattern: "{stem}.history.yaml"

rules:
  - pattern: "*.yaml"
    policy: append
  - pattern: "*.py"
    policy: comment
    comment_syntax: hash
"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write(config_content)
            f.flush()
            config = load_config(Path(f.name))

        assert len(config.rules) == 2
        assert config.rules[0].pattern == "*.yaml"
        assert config.rules[0].policy == OutputPolicy.APPEND
        assert config.rules[1].pattern == "*.py"
        assert config.rules[1].policy == OutputPolicy.COMMENT
        assert config.rules[1].comment_syntax == CommentSyntax.HASH

        Path(f.name).unlink()

    def test_load_config_with_skip_policy(self):
        """Test loading config with skip policy."""
        config_content = """
rules:
  - pattern: "generated/**"
    policy: skip
"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write(config_content)
            f.flush()
            config = load_config(Path(f.name))

        assert len(config.rules) == 1
        assert config.rules[0].policy == OutputPolicy.SKIP

        Path(f.name).unlink()


class TestPatternMatching:
    """Tests for pattern matching in OutputConfig."""

    def test_simple_glob_pattern(self):
        """Test simple glob patterns like *.yaml."""
        config = get_default_config()

        assert config.get_rule_for_file("foo.yaml").policy == OutputPolicy.APPEND
        assert config.get_rule_for_file("bar.yml").policy == OutputPolicy.APPEND
        assert config.get_rule_for_file("data.json").policy == OutputPolicy.APPEND
        assert config.get_rule_for_file("script.py").policy == OutputPolicy.SIDECAR

    def test_path_with_directories(self):
        """Test patterns match filenames, not full paths for simple patterns."""
        config = get_default_config()

        # Should match *.yaml regardless of directory
        assert config.get_rule_for_file("src/models/foo.yaml").policy == OutputPolicy.APPEND
        assert config.get_rule_for_file("/absolute/path/to/bar.yml").policy == OutputPolicy.APPEND

    def test_first_match_wins(self):
        """Test that first matching rule wins."""
        config_content = """
rules:
  - pattern: "*.yaml"
    policy: skip
  - pattern: "*.yaml"
    policy: append
"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write(config_content)
            f.flush()
            config = load_config(Path(f.name))

        # First rule should win
        assert config.get_rule_for_file("foo.yaml").policy == OutputPolicy.SKIP

        Path(f.name).unlink()


class TestFindConfig:
    """Tests for find_config."""

    def test_find_config_in_current_dir(self):
        """Test finding config in current directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir).resolve() / ".ai-blame.yaml"
            config_path.write_text("rules: []")

            found = find_config(Path(tmpdir).resolve())
            assert found == config_path

    def test_find_config_in_parent_dir(self):
        """Test finding config in parent directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create config in root
            config_path = Path(tmpdir).resolve() / ".ai-blame.yaml"
            config_path.write_text("rules: []")

            # Create subdir
            subdir = Path(tmpdir).resolve() / "subdir"
            subdir.mkdir()

            found = find_config(subdir)
            assert found == config_path

    def test_find_config_not_found(self):
        """Test when no config exists."""
        with tempfile.TemporaryDirectory() as tmpdir:
            found = find_config(Path(tmpdir).resolve())
            assert found is None


class TestResolveSidecarPath:
    """Tests for resolve_sidecar_path."""

    def test_stem_pattern(self):
        """Test {stem} pattern variable."""
        result = resolve_sidecar_path(Path("src/foo.py"), "{stem}.history.yaml")
        assert result == Path("src/foo.history.yaml")

    def test_name_pattern(self):
        """Test {name} pattern variable."""
        result = resolve_sidecar_path(Path("src/foo.py"), "{name}.history.yaml")
        assert result == Path("src/foo.py.history.yaml")

    def test_hidden_directory_pattern(self):
        """Test pattern with hidden directory."""
        result = resolve_sidecar_path(Path("src/foo.py"), ".history/{name}.yaml")
        assert result == Path("src/.history/foo.py.yaml")

    def test_ext_pattern(self):
        """Test {ext} pattern variable."""
        result = resolve_sidecar_path(Path("src/foo.py"), "{stem}{ext}.history.yaml")
        assert result == Path("src/foo.py.history.yaml")
