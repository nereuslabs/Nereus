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
    ("nereus.core.state", "DiagnosticQuestion"),
    ("nereus.core.state", "WeaknessReport"),
    ("nereus.core.session", "LearningSession"),
    ("nereus.core.session", "UserSession"),
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


def _sqlite(serde: Any | None = None, db_path: str | None = None) -> Any:
    """Build a SqliteSaver backed by ``db_path`` or ``settings.checkpoint_db``."""
    from langgraph.checkpoint.sqlite import SqliteSaver

    path = db_path or settings.checkpoint_db
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    # LangGraph executes graph nodes in a thread pool — the connection must
    # therefore be shareable across threads (check_same_thread=False); the
    # saver serializes writes internally via its own lock.
    conn = sqlite3.connect(str(path), check_same_thread=False)
    return SqliteSaver(conn, serde=serde)


def _redis(*, redis_url: str | None = None, db_path: str | None = None) -> Any:
    """Build a RedisSaver with the Nereus msgpack allowlist.

    ``RedisSaver.from_conn_string`` is a contextmanager that closes the client
    on exit, so we construct directly and call ``setup()`` to provision indexes.
    If Redis is unreachable (e.g. invalid host, no server), fall back to SQLite
    so the system remains offline-first.
    """
    from langgraph.checkpoint.redis import RedisSaver

    url = redis_url or settings.redis_url
    try:
        saver = RedisSaver(redis_url=url).with_allowlist(_ALLOWED_MSGPCK)
        saver.setup()
        logger.info("Redis checkpointer configured at %s", url)
        return saver
    except Exception as exc:  # noqa: BLE001
        logger.warning(
            "Redis checkpointer unavailable at %s (%s); falling back to SQLite.",
            url,
            exc,
        )
        return _sqlite(db_path=db_path)


def build_checkpointer(
    backend: str | CheckpointBackend | None = None,
    *,
    db_path: str | None = None,
    redis_url: str | None = None,
) -> Any:
    """Factory for a persistent :class:`BaseCheckpointer`.

    Resolution order:
    1. explicit ``backend`` argument
    2. ``settings.checkpoint_backend`` (``CHECKPOINTER`` env / .env)

    Backends:
    - ``memory`` — in-memory, per-process (default while ``CHECKPOINTER`` unset).
    - ``sqlite`` — local file (``settings.checkpoint_db``, default
      ``.checkpoints/nereus.sqlite3``); offline-first.
    - ``redis`` — shared/network (``settings.redis_url``); auto-falls back to
      SQLite if unreachable. Uses the Nereus msgpack ``allowlist`` via
      ``RedisSaver.with_allowlist``.

    Args:
        backend: optional override (useful in tests); falls back to settings.
        db_path: override SQLite file path (defaults to ``settings.checkpoint_db``).
        redis_url: override Redis URL (defaults to ``settings.redis_url``).
    """
    serde = _serde()
    backend = CheckpointBackend(backend or settings.checkpoint_backend)

    if backend is CheckpointBackend.MEMORY:
        from langgraph.checkpoint.memory import MemorySaver

        return MemorySaver(serde=serde)
    if backend is CheckpointBackend.SQLITE:
        return _sqlite(serde=serde, db_path=db_path)
    if backend is CheckpointBackend.REDIS:
        return _redis(redis_url=redis_url, db_path=db_path)
    raise ValueError(f"Unknown checkpoint backend: {backend!r}")
