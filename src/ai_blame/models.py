"""Data models for AI blame."""

from datetime import datetime
from enum import Enum
from fnmatch import fnmatch
from pathlib import Path
from typing import Optional

from pydantic import BaseModel, Field, computed_field


class CurationAction(str, Enum):
    """Type of curation action."""

    CREATED = "CREATED"
    EDITED = "EDITED"


class CurationEvent(BaseModel):
    """
    A single curation event in the audit trail.

    >>> from datetime import datetime, timezone
    >>> event = CurationEvent(
    ...     timestamp=datetime(2025, 1, 1, tzinfo=timezone.utc),
    ...     model="claude-opus-4-5",
    ...     action=CurationAction.CREATED
    ... )
    >>> event.to_dict()["model"]
    'claude-opus-4-5'
    """

    timestamp: datetime
    model: Optional[str] = None
    action: Optional[CurationAction] = None
    description: Optional[str] = None
    agent_tool: Optional[str] = None
    agent_version: Optional[str] = None

    def to_dict(self) -> dict:
        """Convert to dictionary for YAML output, excluding None values."""
        d = {"timestamp": self.timestamp.isoformat()}
        if self.model:
            d["model"] = self.model
        if self.agent_tool:
            d["agent_tool"] = self.agent_tool
        if self.agent_version:
            d["agent_version"] = self.agent_version
        if self.action:
            d["action"] = self.action.value
        if self.description:
            d["description"] = self.description
        return d


class EditRecord(BaseModel):
    """A record of a successful file edit extracted from traces."""

    file_path: str
    timestamp: datetime
    model: str
    session_id: str
    is_create: bool
    change_size: int = Field(description="Approximate size of change in chars")
    agent_tool: str = "claude-code"
    agent_version: Optional[str] = None


class FilterConfig(BaseModel):
    """Configuration for filtering edit records."""

    initial_and_recent_only: bool = False
    min_change_size: int = 0
    since: Optional[datetime] = None
    until: Optional[datetime] = None
    file_pattern: Optional[str] = None


class FileHistory(BaseModel):
    """
    Aggregated edit history for a single file.

    >>> from datetime import datetime, timezone
    >>> history = FileHistory(
    ...     file_path="foo.py",
    ...     events=[
    ...         CurationEvent(timestamp=datetime(2025, 1, 1, tzinfo=timezone.utc)),
    ...         CurationEvent(timestamp=datetime(2025, 1, 2, tzinfo=timezone.utc)),
    ...     ]
    ... )
    >>> history.first_edit.day
    1
    >>> history.last_edit.day
    2
    """

    file_path: str
    events: list[CurationEvent] = Field(default_factory=list)

    @computed_field  # type: ignore[prop-decorator]
    @property
    def first_edit(self) -> Optional[datetime]:
        """Get timestamp of first edit."""
        if not self.events:
            return None
        return min(e.timestamp for e in self.events)

    @computed_field  # type: ignore[prop-decorator]
    @property
    def last_edit(self) -> Optional[datetime]:
        """Get timestamp of last edit."""
        if not self.events:
            return None
        return max(e.timestamp for e in self.events)


# --- Output Configuration ---


class OutputPolicy(str, Enum):
    """Policy for how curation history is written to a file."""

    APPEND = "append"
    SIDECAR = "sidecar"
    COMMENT = "comment"
    SKIP = "skip"


class CommentSyntax(str, Enum):
    """Comment syntax for embedding history in code files."""

    HASH = "hash"
    SLASH = "slash"
    HTML = "html"


class FileRule(BaseModel):
    """
    Rule for how to handle a file matching a pattern.

    >>> rule = FileRule(pattern="*.yaml", policy=OutputPolicy.APPEND)
    >>> rule.policy
    <OutputPolicy.APPEND: 'append'>

    >>> rule = FileRule(pattern="*.yaml", policy="append")
    >>> rule.policy
    <OutputPolicy.APPEND: 'append'>
    """

    pattern: str = "*"
    policy: OutputPolicy = OutputPolicy.APPEND
    format: str = "yaml"
    comment_syntax: Optional[CommentSyntax] = None
    sidecar_pattern: Optional[str] = None


class OutputConfig(BaseModel):
    """
    Configuration for output policies, mapping file patterns to rules.

    Rules are evaluated in order; first match wins.

    >>> config = OutputConfig(
    ...     defaults=FileRule(pattern="*", policy=OutputPolicy.SIDECAR),
    ...     rules=[FileRule(pattern="*.yaml", policy=OutputPolicy.APPEND)]
    ... )
    >>> config.get_rule_for_file("foo.yaml").policy
    <OutputPolicy.APPEND: 'append'>
    >>> config.get_rule_for_file("foo.py").policy
    <OutputPolicy.SIDECAR: 'sidecar'>
    """

    defaults: Optional[FileRule] = None
    rules: list[FileRule] = Field(default_factory=list)

    def get_rule_for_file(self, path: str) -> Optional[FileRule]:
        """
        Return first matching rule, or defaults if no rule matches.

        Uses fnmatch for glob-style pattern matching.
        """
        filename = Path(path).name

        for rule in self.rules:
            # Use full path for patterns with path separators or **
            if "/" in rule.pattern or "**" in rule.pattern:
                if fnmatch(path, rule.pattern):
                    return rule
            else:
                # Match against filename only
                if fnmatch(filename, rule.pattern):
                    return rule

        return self.defaults
