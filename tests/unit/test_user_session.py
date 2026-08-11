"""Unit tests for UserSession and UserStore (P1, Issues #8/#57)."""

from __future__ import annotations

from pathlib import Path

from nereus.core.session import UserSession, session_path_for
from nereus.core.state import (
    DiagnosticQuestion,
    Roadmap,
    RoadmapTopic,
    UserLevel,
    UserProfile,
    WeaknessReport,
)
from nereus.core.user_store import UserStore


def _profile() -> UserProfile:
    return UserProfile(
        skill="Python",
        current_level=UserLevel.BEGINNER,
        target_level=UserLevel.INTERMEDIATE,
        hours_per_day=1.0,
        deadline_days=30,
        goal="Learn Python",
    )


def test_session_dump_and_load_roundtrip(tmp_path: Path) -> None:
    sess = UserSession(
        session_id="abc123",
        user_id="u1",
        user_profile=_profile(),
        roadmap=Roadmap(topics=[RoadmapTopic(id="1", title="T1", description="d", difficulty=0.3)]),
        current_topic_index=1,
        weakness_report=WeaknessReport(weak_areas=["syntax"]),
    )
    path = tmp_path / "abc123.json"
    sess.dump(path)

    loaded = UserSession.load(path)
    assert loaded is not None
    assert loaded.session_id == "abc123"
    assert loaded.user_profile.skill == "Python"
    assert loaded.roadmap.topics[0].title == "T1"
    assert loaded.current_topic_index == 1
    assert loaded.weakness_report is not None
    assert loaded.weakness_report.weak_areas == ["syntax"]


def test_session_load_missing_returns_none(tmp_path: Path) -> None:
    assert UserSession.load(tmp_path / "missing.json") is None


def test_session_to_state_dict() -> None:
    sess = UserSession(
        session_id="s1",
        user_id="u1",
        user_profile=_profile(),
        roadmap=Roadmap(topics=[RoadmapTopic(id="1", title="T1", description="d")]),
        current_topic_index=0,
        diagnostic_questions=[DiagnosticQuestion(id="q1", question="Q?", options=["a"])],
        user_diagnostic_answers={"q1": "a"},
        weakness_report=WeaknessReport(weak_areas=["x"]),
    )
    state = sess.to_state_dict()
    assert state["user_profile"].skill == "Python"
    assert state["roadmap"].topics[0].id == "1"
    assert state["diagnostic_questions"][0].id == "q1"
    assert state["user_diagnostic_answers"] == {"q1": "a"}
    assert state["weakness_report"].weak_areas == ["x"]
    assert state["current_topic_index"] == 0
    assert state["messages"] == []


def test_session_new_id_when_missing() -> None:
    sess = UserSession(session_id="")
    assert len(sess.session_id) > 0  # auto-generated UUID


def test_session_path_for_sharding(monkeypatch) -> None:
    from nereus.config.settings import settings

    monkeypatch.setattr(settings, "session_root", "/tmp/sessions-test")

    path = session_path_for("user1", "sess1")
    assert str(path) == "/tmp/sessions-test/user1/sess1.json"

    path_no_user = session_path_for(None, "sess1")
    assert str(path_no_user) == "/tmp/sessions-test/sess1.json"


def test_user_store_crud_sqlite(tmp_path: Path) -> None:
    store = UserStore(db_path=tmp_path / "users.db")
    uid = store.create_user(_profile())
    assert uid

    fetched = store.get_user(uid)
    assert fetched is not None
    assert fetched.skill == "Python"

    users = store.list_users()
    assert len(users) == 1
    assert users[0][0] == uid

    assert store.delete_user(uid) is True
    assert store.get_user(uid) is None


def test_user_store_falls_back_to_memory(tmp_path: Path) -> None:
    """When SQLite path is unwritable, the store still works via memory."""
    bad = tmp_path / "nonexistent" / "sub" / "users.db"
    store = UserStore(db_path=bad)
    uid = store.create_user(_profile())
    fetched = store.get_user(uid)
    assert fetched is not None
    assert fetched.skill == "Python"


def test_user_store_get_user_nonexistent() -> None:
    store = UserStore()
    assert store.get_user("nope") is None
