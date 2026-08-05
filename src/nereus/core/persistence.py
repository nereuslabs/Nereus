from __future__ import annotations

import logging
import sqlite3
from enum import Enum
from pathlib import Path
from typing import Any

from nereus.config.settings import settings

logger = logging.getLogger("nereus.persistence")

# Pydantic models / enums stored in NereusState must be allow-listed for
# LangGraph's msgpack checkpointer (avoids warnings and is forward-compatible
# with LANGGRAPH_STRICT_MSGPACK=true). Kept here so both factory and tests
# share a single source of truth.
_ALLOWED_MSGPCK: list[tuple[str, str]] = [
    ("nereus.core.state", "UserProfile"),
    ("nereus.core.state", "Roadmap"),
    ("nereus.core.state", "RoadmapTopic"),
    ("nereus.core.state", "Assessment"),
    ("nereus.core.state", "Verdict"),
    ("nereus.core.state", "UserLevel"),
    ("nereus.core.state", "LearningStatus"),
    ("nereus.core.state", "RetrievedChunk"),
    ("nereus.core.session", "LearningSession"),
]


class CheckpointBackend(str, Enum):
    """Available LangGraph checkpoint backends."""

    MEMORY = "memory"
    SQLITE = "sqlite"
    REDIS = "redis"


def _serde() -> Any:
    """Shared serde with the Nereus msgpack allowlist."""
    from langgraph.checkpoint.serde.jsonplus import JsonPlusSerializer

    return JsonPlusSerializer(allowed_msgpack_modules=_ALLOWED_MSGPCK)


def _sqlite(serde: Any | None = None) -> Any:
    """Build a SqliteSaver backed by ``settings.checkpoint_db``."""
    from langgraph.checkpoint.sqlite import SqliteSaver

    path = settings.checkpoint_db
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    # LangGraph executes graph nodes in a thread pool — the connection must
    # therefore be shareable across threads (check_same_thread=False) and the
    # saver serializes writes internally via its own lock.
    conn = sqlite3.connect(str(path), check_same_thread=False)
    return SqliteSaver(conn, serde=serde)


def _redis(serde: Any | None = None) -> Any:
    """Build a RedisSaver; fall back to SQLite on connection failure."""
    from langgraph.checkpoint.redis import RedisSaver

    url = settings.redis_url
    try:
        return RedisSaver.from_conn_string(url, serde=serde)
    except Exception as exc:  # noqa: BLE001
        logger.warning(
            "Redis checkpointer unavailable at %s (%s); falling back to SQLite.",
            url,
            exc,
        )
        return _sqlite(serde)


def build_checkpointer(
    backend: str | CheckpointBackend | None = None,
) -> Any:
    """Factory for a persistent :class:`BaseCheckpointer`.

    Resolution order:
    1. explicit ``backend`` argument
    2. ``settings.checkpoint_backend`` env (``CHECKPOINTER``)

    Backends:
    - ``memory`` (default, offline) — in-memory, per-process
    - ``sqlite`` — local file (`.checkpoints/nereus.sqlite3` by default)
    - ``redis`` — shared/network; auto-falls back to SQLite if unreachable

    The resolved saver reuses the Nereus msgpack allowlist so all pydantic
    models in :data:`nereus.core.state` persist cleanly.
    """
    serde = _serde()
    backend = CheckpointBackend(backend or settings.checkpoint_backend)

    if backend is CheckpointBackend.MEMORY:
        from langgraph.checkpoint.memory import MemorySaver

        return MemorySaver(serde=serde)
    if backend is CheckpointBackend.SQLITE:
        return _sqlite(serde=serde)
    if backend is CheckpointBackend.REDIS:
        return _redis(serde=serde)
    raise ValueError(f"Unknown checkpoint backend: {backend!r}")
