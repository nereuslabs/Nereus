"""Live (integration) tests for persistent checkpointer resume (issue #16).

Guarded by ``NEREUS_RUN_LIVE`` so CI remains fast/offline unless explicitly
enabled; mirrors the OpenRouter live suite (``tests/integration/test_live_openrouter.py``).
"""

from __future__ import annotations

import os

import pytest
from langgraph.types import Command

from nereus.core.factory import build_nereus_graph  # noqa: E402
from nereus.core.persistence import CheckpointBackend, build_checkpointer  # noqa: E402
from nereus.core.state import UserLevel, UserProfile  # noqa: E402

pytestmark = pytest.mark.skipif(
    os.environ.get("NEREUS_RUN_LIVE") != "1",
    reason="set NEREUS_RUN_LIVE=1 to run persistent-checkpoint integration tests",
)


def _profile() -> UserProfile:
    return UserProfile(
        skill="Python",
        current_level=UserLevel.BEGINNER,
        target_level=UserLevel.INTERMEDIATE,
        hours_per_day=1.0,
        deadline_days=30,
        goal="Master Python",
    )


def test_sqlite_resume_persists_progress(tmp_path) -> None:
    """End-to-end: run -> persist -> resume in new instance -> state restored."""
    db = tmp_path / "live.sqlite3"
    checkpointer = build_checkpointer(
        CheckpointBackend.SQLITE, db_path=str(db)
    )

    # first instance: run until the first examiner interrupt
    graph_a = build_nereus_graph(interactive=True, checkpointer=checkpointer)
    config = {"configurable": {"thread_id": "live-thread-1"}}
    state = {
        "user_profile": _profile(),
        "max_retries": 2,
    }
    final_a = graph_a.invoke(state, config)
    assert "__interrupt__" in final_a
    first_topic_index = final_a["current_topic_index"]
    assert db.exists()

    # new instance, SAME checkpointer + thread_id -> should resume from interrupt
    graph_b = build_nereus_graph(interactive=True, checkpointer=checkpointer)
    final_b = graph_b.invoke(Command(resume="this is good"), config)

    # resume must continue from the persisted checkpoint: either the topic
    # advanced past the interrupted one, or a retry occurred.
    progressed = (
        final_b["current_topic_index"] != first_topic_index
        or final_b["retry_count"] != 0
        or final_b.get("status") == "completed"
    )
    assert progressed, "resume did not continue from the persisted checkpoint"
