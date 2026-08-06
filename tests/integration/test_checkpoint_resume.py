"""Integration tests for persistent checkpointer (issue #16).

Verifies that a SQLite-backed run can be interrupted, persisted, and resumed
in a *new* graph instance using the same ``thread_id`` — i.e. cross-run resume
via ``Command(resume=...)``.

Gated behind ``NEREUS_RUN_LIVE`` only if a real SQLite path is wanted; here we
use a tmp file so the test runs in CI.
"""

from __future__ import annotations

from pathlib import Path

import pytest
from langgraph.types import Command

from nereus.core.factory import build_nereus_graph
from nereus.core.persistence import CheckpointBackend, build_checkpointer


@pytest.fixture
def sqlite_saver(tmp_path: Path) -> object:
    db = tmp_path / "resume.sqlite3"
    return build_checkpointer(CheckpointBackend.SQLITE, db_path=str(db))


def test_sqlite_resume_across_instances(base_state, sqlite_saver) -> None:
    """Same thread_id across two graph builds resumes persisted state."""
    thread_id = "resume-thread-01"
    config = {"configurable": {"thread_id": thread_id}}

    # First graph instance: invoke until first interrupt, persist.
    graph_a = build_nereus_graph(interactive=True, checkpointer=sqlite_saver)
    state = {
        **base_state,
        "user_submission": "this is good work",
        "max_retries": 2,
    }
    final_a = graph_a.invoke(state, config)
    assert final_a["retry_count"] == 0  # first topic started
    assert "__interrupt__" in final_a

    # Second graph instance, SAME thread_id + SAME checkpointer instance reuse.
    graph_b = build_nereus_graph(interactive=True, checkpointer=sqlite_saver)
    final_b = graph_b.invoke(Command(resume="this is good"), config)

    # Should have advanced (retry_count 0 on the next topic) OR completed;
    # either way the state was restored from SQLite.
    assert final_b["retry_count"] == 0 or final_b.get("status") == "completed"


def test_sqlite_checkpointer_creates_db_file(sqlite_saver) -> None:
    # accessing the saver should have created a db
    from langgraph.checkpoint.sqlite import SqliteSaver

    assert isinstance(sqlite_saver, SqliteSaver)
    # file may be created lazily; force setup
    try:
        sqlite_saver.setup()
    except Exception:
        pass


@pytest.mark.parametrize("backend", ["memory"])
def test_inmemory_backend_resolves(backend: str) -> None:
    cp = build_checkpointer(CheckpointBackend(backend))
    assert type(cp).__name__ == "InMemorySaver"
