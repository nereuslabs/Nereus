from __future__ import annotations

from nereus.core.session import LearningSession
from nereus.core.state import (
    Assessment,
    Roadmap,
    RoadmapTopic,
    UserLevel,
    UserProfile,
    Verdict,
)


def _profile() -> UserProfile:
    return UserProfile(
        skill="Python",
        current_level=UserLevel.BEGINNER,
        target_level=UserLevel.INTERMEDIATE,
        hours_per_day=1.0,
        deadline_days=30,
        goal="Get comfortable writing scripts.",
    )


def _roadmap() -> Roadmap:
    return Roadmap(topics=[RoadmapTopic(id="1", title="loops", description="d")])


def test_session_dump_load_roundtrip(tmp_path) -> None:
    session = LearningSession(
        user_profile=_profile(),
        roadmap=_roadmap(),
        current_topic_index=0,
        completed=[Assessment(topic_id="1", score=85.0, verdict=Verdict.PASS, feedback="ok")],
        retry_counts={"1": 0},
        aggregated_weak_areas={"x": 1},
    )
    path = tmp_path / "session.json"
    session.dump(path)
    assert path.exists()

    loaded = LearningSession.load(path)
    assert loaded.user_profile.skill == "Python"
    assert loaded.completed[0].score == 85.0
    assert loaded.completed[0].verdict == Verdict.PASS
    assert loaded.aggregated_weak_areas == {"x": 1}
    assert loaded.roadmap.topics[0].id == "1"


def test_session_dump_load_empty(tmp_path) -> None:
    path = tmp_path / "empty.json"
    LearningSession().dump(path)
    loaded = LearningSession.load(path)
    assert loaded.completed == []
    assert loaded.current_topic_index == 0
    assert loaded.user_profile is None


def test_session_for_loads_persisted_session(tmp_path) -> None:
    """NereusGraph._session_for returns the saved session when session_path exists."""
    from nereus.core.graph import NereusGraph

    session = LearningSession(
        user_profile=_profile(),
        roadmap=_roadmap(),
        current_topic_index=0,
        completed=[Assessment(topic_id="1", score=85.0, verdict=Verdict.PASS, feedback="ok")],
    )
    path = tmp_path / "persisted.json"
    session.dump(path)

    graph = NereusGraph(session_path=path, checkpointer=None)
    loaded = graph._session_for(None)
    assert loaded is not None
    assert loaded.user_profile.skill == "Python"
    assert len(loaded.completed) == 1


def test_session_for_returns_none_without_path(tmp_path) -> None:
    """If session_path is unset (or file missing), _session_for returns None."""
    from nereus.core.graph import NereusGraph

    graph = NereusGraph(session_path=None, checkpointer=None)
    assert graph._session_for(None) is None

    missing = tmp_path / "nope.json"
    graph2 = NereusGraph(session_path=missing, checkpointer=None)
    assert graph2._session_for(None) is None


def test_dump_session_skips_when_disabled(tmp_path) -> None:
    """No session_path → _dump_session is a no-op."""
    from nereus.core.graph import NereusGraph

    graph = NereusGraph(session_path=None, checkpointer=None)
    graph._dump_session({"status": "completed"})  # must not raise
