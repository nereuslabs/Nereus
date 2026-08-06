"""Unit tests for the persistent checkpointer factory (issue #16)."""

from __future__ import annotations

from pathlib import Path

from nereus.core.persistence import (
    _ALLOWED_MSGPCK,
    CheckpointBackend,
    build_checkpointer,
)


def test_build_checkpointer_memory() -> None:
    cp = build_checkpointer(CheckpointBackend.MEMORY)
    assert type(cp).__name__ == "InMemorySaver"


def test_build_checkpointer_sqlite(tmp_path: Path) -> None:
    db = tmp_path / "test.sqlite3"
    cp = build_checkpointer(CheckpointBackend.SQLITE, db_path=str(db))
    assert type(cp).__name__ == "SqliteSaver"
    assert db.exists()


def test_build_checkpointer_sqlite_uses_settings_when_no_override(
    tmp_path: Path, monkeypatch
) -> None:
    """Without db_path override, falls back to settings.checkpoint_db."""
    db = tmp_path / "from-settings.sqlite3"
    monkeypatch.setattr("nereus.config.settings.settings.checkpoint_db", str(db))
    cp = build_checkpointer(CheckpointBackend.SQLITE)
    assert type(cp).__name__ == "SqliteSaver"
    assert db.exists()


def test_build_checkpointer_redis_fallback_sqlite(tmp_path: Path) -> None:
    """When Redis is unreachable, falls back to SQLite with a warning."""
    db = tmp_path / "fallback.sqlite3"
    # point redis at an invalid URL so RedisSaver.from_conn_string raises
    cp = build_checkpointer(
        CheckpointBackend.REDIS, redis_url="redis://invalid-host:6379/0", db_path=str(db)
    )
    assert type(cp).__name__ == "SqliteSaver"
    assert db.exists()


def test_allowed_msgpack_models() -> None:
    """Verify the allowlist contains all expected models for serialization."""
    allowed_names = {name for _module, name in _ALLOWED_MSGPCK}
    expected = {
        "UserProfile",
        "Roadmap",
        "RoadmapTopic",
        "Assessment",
        "Verdict",
        "UserLevel",
        "LearningStatus",
        "RetrievedChunk",
        "LearningSession",
    }
    assert allowed_names == expected
