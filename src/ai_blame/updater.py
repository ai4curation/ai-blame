"""Update YAML files with curation history."""

from pathlib import Path
from typing import Optional

import yaml

from .models import FileHistory


def generate_curation_yaml(history: FileHistory) -> str:
    """Generate YAML string for curation_history section."""
    events_data = [e.to_dict() for e in history.events]
    return yaml.dump({"curation_history": events_data}, default_flow_style=False, sort_keys=False)


def update_yaml_file(
    file_path: Path,
    history: FileHistory,
    dry_run: bool = True,
) -> tuple[bool, str]:
    """Update a YAML file with curation history.

    Returns:
        Tuple of (success, message/preview)
    """
    if not file_path.exists():
        return False, f"File not found: {file_path}"

    content = file_path.read_text()

    # Check if curation_history already exists
    if "curation_history:" in content:
        # Remove existing curation_history section
        # Find and remove everything from "curation_history:" to end or next top-level key
        lines = content.split("\n")
        new_lines = []
        in_curation = False
        for line in lines:
            if line.startswith("curation_history:"):
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

    # Add blank line before curation_history if not already there
    if not content.endswith("\n\n"):
        content += "\n"

    new_content = content + curation_yaml

    if dry_run:
        return True, new_content

    # Write the file
    file_path.write_text(new_content)
    return True, f"Updated: {file_path}"


def preview_update(file_path: Path, history: FileHistory) -> str:
    """Generate a preview of the curation_history section for a file."""
    return generate_curation_yaml(history)
