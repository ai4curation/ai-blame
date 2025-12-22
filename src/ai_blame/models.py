"""Data models for AI Log Miner."""

from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Optional


class CurationAction(str, Enum):
    """Type of curation action."""

    CREATED = "CREATED"
    EDITED = "EDITED"


@dataclass
class CurationEvent:
    """A single curation event in the audit trail."""

    timestamp: datetime
    model: Optional[str] = None
    action: Optional[CurationAction] = None
    description: Optional[str] = None

    def to_dict(self) -> dict:
        """Convert to dictionary for YAML output."""
        d = {"timestamp": self.timestamp.isoformat()}
        if self.model:
            d["model"] = self.model
        if self.action:
            d["action"] = self.action.value
        if self.description:
            d["description"] = self.description
        return d


@dataclass
class EditRecord:
    """A record of a successful file edit extracted from traces."""

    file_path: str
    timestamp: datetime
    model: str
    session_id: str
    is_create: bool
    change_size: int  # Approximate size of change in chars


@dataclass
class FilterConfig:
    """Configuration for filtering edit records."""

    initial_and_recent_only: bool = False
    min_change_size: int = 0
    since: Optional[datetime] = None
    until: Optional[datetime] = None
    file_pattern: Optional[str] = None  # e.g., "kb/disorders/"


@dataclass
class FileHistory:
    """Aggregated edit history for a single file."""

    file_path: str
    events: list[CurationEvent] = field(default_factory=list)

    @property
    def first_edit(self) -> Optional[datetime]:
        """Get timestamp of first edit."""
        if not self.events:
            return None
        return min(e.timestamp for e in self.events)

    @property
    def last_edit(self) -> Optional[datetime]:
        """Get timestamp of last edit."""
        if not self.events:
            return None
        return max(e.timestamp for e in self.events)
