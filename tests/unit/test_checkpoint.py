"""Unit tests for the persistent checkpointer factory (issue #16)."""

from __future__ import annotations

from pathlib import Path

import pytest

from nereus.core.persistence import (
    _ALLOWED_MSGPCK,
    CheckpointBackend,
    build_checkpointer,
)


@pytest.fixture
def temp_db(tmp_path: Path) -> Path:
    return tmp_path / "nereus.sqlite3"


def test_build_checkpointer_memory() -> None:
    cp = build_checkpointer(CheckpointBackend.MEMORY)
    assert type(cp).__name__ == "InMemorySaver"


def test_build_checkpointer_sqlite(tmp_path: Path, monkeypatch) -> None:
    db = tmp_path / "test.sqlite3"
    monkeypatch.setenv("CHECKPOINT_DB", str(db))
    cp = build_checkpointer(CheckpointBackend.SQLITE)
    assert type(cp).__name__ == "SqliteSaver"


def test_build_checkpointer_redis_fallback_sqlite(
    monkeypatch, tmp_path: Path
) -> None:
    monkeypatch.setenv("REDIS_HOST", "invalid-host")
    monkeypatch.setenv("CHECKPOINT_DB", str(tmp_path / "fallback.sqlite3"))

    cp = build_checkpointer(CheckpointBackend.REDIS)
    assert type(cp).__name__ == "SqliteSaver"


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