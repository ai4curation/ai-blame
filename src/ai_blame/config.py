"""Configuration loading for ai-blame."""

from pathlib import Path
from typing import Optional

import yaml

from .models import FileRule, OutputConfig, OutputPolicy

CONFIG_FILENAME = ".ai-blame.yaml"


def get_default_config() -> OutputConfig:
    """
    Return sensible default configuration.

    - YAML files: append edit_history directly
    - JSON files: append edit_history as JSON
    - Everything else: sidecar file

    >>> config = get_default_config()
    >>> config.get_rule_for_file("foo.yaml").policy
    <OutputPolicy.APPEND: 'append'>
    >>> config.get_rule_for_file("data.json").policy
    <OutputPolicy.APPEND: 'append'>
    >>> config.get_rule_for_file("script.py").policy
    <OutputPolicy.SIDECAR: 'sidecar'>
    """
    return OutputConfig(
        defaults=FileRule(
            pattern="*",
            policy=OutputPolicy.SIDECAR,
            sidecar_pattern="{stem}.history.yaml",
        ),
        rules=[
            FileRule(pattern="*.yaml", policy=OutputPolicy.APPEND),
            FileRule(pattern="*.yml", policy=OutputPolicy.APPEND),
            FileRule(pattern="*.json", policy=OutputPolicy.APPEND, format="json"),
        ],
    )


def find_config(start_dir: Optional[Path] = None) -> Optional[Path]:
    """
    Walk up from start_dir to find .ai-blame.yaml.

    Returns the path to the config file, or None if not found.

    >>> import tempfile
    >>> with tempfile.TemporaryDirectory() as tmpdir:
    ...     # No config file exists
    ...     result = find_config(Path(tmpdir))
    ...     result is None
    True
    """
    if start_dir is None:
        start_dir = Path.cwd()

    current = start_dir.resolve()

    while current != current.parent:
        config_path = current / CONFIG_FILENAME
        if config_path.exists():
            return config_path
        current = current.parent

    # Check root
    config_path = current / CONFIG_FILENAME
    if config_path.exists():
        return config_path

    return None


def load_config(path: Path) -> OutputConfig:
    """
    Load configuration from a YAML file.

    Pydantic handles validation and enum coercion automatically.

    >>> import tempfile
    >>> with tempfile.NamedTemporaryFile(mode='w', suffix='.yaml', delete=False) as f:
    ...     _ = f.write('''
    ... defaults:
    ...   policy: sidecar
    ...   sidecar_pattern: "{stem}.history.yaml"
    ... rules:
    ...   - pattern: "*.yaml"
    ...     policy: append
    ...   - pattern: "*.py"
    ...     policy: comment
    ...     comment_syntax: hash
    ... ''')
    ...     f.flush()
    ...     config = load_config(Path(f.name))
    ...     config.get_rule_for_file("foo.yaml").policy
    <OutputPolicy.APPEND: 'append'>
    >>> import os; os.unlink(f.name)
    """
    with open(path) as f:
        data = yaml.safe_load(f)

    if data is None:
        return get_default_config()

    # Pydantic handles enum coercion (e.g., "append" -> OutputPolicy.APPEND)
    return OutputConfig.model_validate(data)


def resolve_sidecar_path(source_path: Path, pattern: str) -> Path:
    """
    Resolve sidecar file path from source path and pattern.

    Pattern variables:
    - {dir}: parent directory
    - {name}: filename with extension
    - {stem}: filename without extension
    - {ext}: extension with dot

    >>> resolve_sidecar_path(Path("src/foo.py"), "{stem}.history.yaml")
    PosixPath('src/foo.history.yaml')

    >>> resolve_sidecar_path(Path("src/foo.py"), "{name}.history.yaml")
    PosixPath('src/foo.py.history.yaml')

    >>> resolve_sidecar_path(Path("src/foo.py"), ".history/{name}.yaml")
    PosixPath('src/.history/foo.py.yaml')
    """
    parent = source_path.parent
    name = source_path.name
    stem = source_path.stem
    ext = source_path.suffix

    result = pattern.format(
        dir=str(parent),
        name=name,
        stem=stem,
        ext=ext,
    )

    # If pattern starts with a path component, join with parent
    if result.startswith(".") or "/" not in result:
        return parent / result
    else:
        return Path(result)
