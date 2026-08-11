"""Integration tests for multi-user session isolation (P1, Issues #8/#57)."""

from __future__ import annotations

from pathlib import Path

from nereus.core.factory import build_nereus_graph
from nereus.core.persistence import CheckpointBackend, build_checkpointer
from nereus.core.session import UserSession
from nereus.core.state import Roadmap, RoadmapTopic, UserLevel, UserProfile
from nereus.llm.stub import StubLLMProvider


def _profile(skill: str = "Python") -> UserProfile:
    return UserProfile(
        skill=skill,
        current_level=UserLevel.BEGINNER,
        target_level=UserLevel.INTERMEDIATE,
        hours_per_day=1.0,
        deadline_days=30,
        goal=f"Learn {skill}",
    )


def test_graph_loads_user_session_from_disk(tmp_path: Path, monkeypatch) -> None:
    """invoke() with session_id restores state from SESSION_ROOT/{user_id}/{sid}.json."""
    from nereus.config.settings import settings

    monkeypatch.setattr(settings, "session_root", str(tmp_path))
    monkeypatch.setattr(settings, "checkpoint_backend", "memory")

    session_id = "test-sess-1"
    user_id = "user-A"
    sess = UserSession(
        session_id=session_id,
        user_id=user_id,
        user_profile=_profile(),
        roadmap=Roadmap(
            topics=[
                RoadmapTopic(id="1", title="Topic A", description="desc", difficulty=0.3),
                RoadmapTopic(id="2", title="Topic B", description="desc", difficulty=0.7),
            ]
        ),
        current_topic_index=1,
    )
    sess.dump(tmp_path / user_id / f"{session_id}.json")

    graph = build_nereus_graph(
        provider=StubLLMProvider(),
        checkpointer=build_checkpointer(CheckpointBackend.MEMORY),
        session_id=session_id,
        user_id=user_id,
        run_diagnostic=False,
    )

    # Verify the graph's _load_user_session picks up the persisted state
    config = {"configurable": {"thread_id": session_id, "session_id": session_id}}
    loaded_state = graph._load_user_session(config)

    # Should have loaded the persisted roadmap, not the default empty one
    assert len(loaded_state["roadmap"].topics) == 2
    assert loaded_state["roadmap"].topics[0].title == "Topic A"
    assert loaded_state["current_topic_index"] == 1  # restored


def test_two_users_isolation(tmp_path: Path, monkeypatch) -> None:
    """User A and User B have independent session files on disk.

    Even with the same session_id, the user_id namespace keeps files separate.
    """
    from nereus.config.settings import settings

    monkeypatch.setattr(settings, "session_root", str(tmp_path))
    monkeypatch.setattr(settings, "checkpoint_backend", "memory")

    # User A session
    sess_a = UserSession(
        session_id="run-1",
        user_id="user-A",
        user_profile=_profile("Python"),
        roadmap=Roadmap(topics=[RoadmapTopic(id="1", title="Python Basics", description="d")]),
        current_topic_index=0,
    )
    path_a = tmp_path / "user-A" / "run-1.json"
    sess_a.dump(path_a)

    # User B session — different skill, same session_id
    sess_b = UserSession(
        session_id="run-1",
        user_id="user-B",
        user_profile=_profile("Golang"),
        roadmap=Roadmap(topics=[RoadmapTopic(id="1", title="Go Intro", description="d")]),
        current_topic_index=0,
    )
    path_b = tmp_path / "user-B" / "run-1.json"
    sess_b.dump(path_b)

    # Load independently — isolation check
    loaded_a = UserSession.load(path_a)
    loaded_b = UserSession.load(path_b)

    assert loaded_a is not None
    assert loaded_b is not None
    assert loaded_a.user_profile.skill == "Python"
    assert loaded_a.roadmap.topics[0].title == "Python Basics"
    assert loaded_b.user_profile.skill == "Golang"
    assert loaded_b.roadmap.topics[0].title == "Go Intro"

    # Verify path sharding
    from nereus.core.session import session_path_for

    assert str(session_path_for("user-A", "run-1")) == str(path_a)
    assert str(session_path_for("user-B", "run-1")) == str(path_b)
    assert path_a != path_b  # different files despite same session_id


def test_session_dumped_after_partial_run(tmp_path: Path, monkeypatch) -> None:
    """After invoke, UserSession is persisted to disk with current state."""
    from nereus.config.settings import settings

    monkeypatch.setattr(settings, "session_root", str(tmp_path))
    monkeypatch.setattr(settings, "checkpoint_backend", "memory")

    session_id = "persist-test"
    user_id = "user-X"
    graph = build_nereus_graph(
        provider=StubLLMProvider(),
        checkpointer=build_checkpointer(CheckpointBackend.MEMORY),
        session_id=session_id,
        user_id=user_id,
        run_diagnostic=False,
    )

    profile = _profile()
    # Use get_state after a partial invoke to trigger _dump_session
    graph.invoke(
        {
            "user_profile": profile,
            "max_retries": 2,
            "user_submission": "good",
            "material": "mat",
            "task": "task",
            "status": "learning",
        },
        config={"configurable": {"thread_id": session_id, "session_id": session_id}},
    )

    # File should exist on disk (dumped after the first invoke boundary)
    path = tmp_path / user_id / f"{session_id}.json"
    assert path.exists()

    # Reloaded session should reflect the state
    loaded = UserSession.load(path)
    assert loaded is not None
    assert loaded.user_profile.skill == "Python"
