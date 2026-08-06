"""Integration tests for runtime LearningSession persistence (issue #22).

Verifies that a completed run writes ``LearningSession.dump`` to
``session_path`` and that a fresh graph instance can reload it on resume,
even when only the JSON file (not the checkpointer) survives.

Runs in CI (file-based), no external services required.
"""

from __future__ import annotations

from pathlib import Path

import pytest
from langgraph.types import Command

from nereus.core.factory import build_nereus_graph
from nereus.core.persistence import CheckpointBackend, build_checkpointer
from nereus.core.session import LearningSession


@pytest.fixture
def sqlite_saver(tmp_path: Path) -> object:
    return build_checkpointer(CheckpointBackend.SQLITE, db_path=str(tmp_path / "sess.sqlite3"))


def test_session_dumps_after_run(base_state, sqlite_saver, tmp_path: Path) -> None:
    """After the tutor node runs the LearningSession JSON is written to disk.

    In interactive mode the graph pauses at the first examiner question, so we
    drive one submission ("good") to produce an Assessment before asserting the
    persisted session reflects it.
    """
    session_file = tmp_path / "sessions" / "sess-01.json"
    graph = build_nereus_graph(
        interactive=True, checkpointer=sqlite_saver, session_path=session_file
    )
    config = {"configurable": {"thread_id": "sess-01"}}
    state = {
        "user_profile": base_state["user_profile"],
        "max_retries": 2,
    }

    final = graph.invoke(state, config)
    assert "__interrupt__" in final  # paused at first examiner

    # submit "good" -> examiner records a PASS assessment
    final = graph.invoke(Command(resume="this is good work"), config)

    assert session_file.exists()
    loaded = LearningSession.load(session_file)
    assert loaded.roadmap.topics  # coach built the roadmap
    # at least one topic was assessed
    assert len(loaded.completed) >= 1
    assert loaded.completed[0].verdict.value in ("pass", "retry")


def test_session_resumes_from_json(base_state, sqlite_saver, tmp_path: Path) -> None:
    """A recovered LearningSession JSON is merged into state so a fresh graph
    instance can bootstrap a profile/roadmap when the checkpoint is missing.

    Exercises the JSON fallback path used when ``--session-path`` is the only
    surviving artifact (e.g. checkpoint cleared but session JSON kept).
    """
    session_file = tmp_path / "sess-resume.json"

    # --- Phase A: run + persist ---
    graph_a = build_nereus_graph(
        interactive=True, checkpointer=sqlite_saver, session_path=session_file
    )
    config = {"configurable": {"thread_id": "resume-02"}}
    state_a = {
        "user_profile": base_state["user_profile"],
        "max_retries": 2,
    }
    final_a = graph_a.invoke(state_a, config)
    assert "__interrupt__" in final_a  # paused at examiner

    # submit "good" -> an Assessment lands in the session JSON
    graph_a.invoke(Command(resume="this is good work"), config)
    assert session_file.exists()
    phase_a_session = LearningSession.load(session_file)
    assert len(phase_a_session.completed) >= 1

    # --- Phase B: simulate lost checkpoint but surviving JSON ---
    graph_b = build_nereus_graph(
        interactive=True,
        checkpointer=build_checkpointer(CheckpointBackend.MEMORY),
        session_path=session_file,
    )
    injected = graph_b._session_for(config)
    assert injected is not None
    assert injected.user_profile.skill == "Python"
    assert len(injected.completed) >= 1

    fresh = {
        "user_profile": None,
        "roadmap": None,
        "current_topic_index": None,
        "session": None,
        "messages": [],
    }
    graph_b._merge_session(fresh, injected)
    assert fresh["user_profile"].skill == "Python"
    assert len(fresh["roadmap"].topics) >= 1
    assert isinstance(fresh["current_topic_index"], int)
    # completed one topic (index advanced from 0 to >=1)
    assert fresh["current_topic_index"] >= 1


def test_no_session_path_no_dump(base_state, sqlite_saver, tmp_path: Path, caplog) -> None:
    """Without session_path set, the graph must not write JSON."""
    graph = build_nereus_graph(interactive=True, checkpointer=sqlite_saver)
    config = {"configurable": {"thread_id": "no-dump-03"}}
    state = {"user_profile": base_state["user_profile"], "max_retries": 2}

    graph.invoke(state, config)

    sess_dir = Path(".sessions")
    # should NOT have created a session file for this thread
    assert not sess_dir.exists() or not list(sess_dir.glob("no-dump-03.json"))
