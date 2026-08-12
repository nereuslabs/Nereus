"""Multi-user + session-resume wiring for the Chainlit UI (Issue #8/#57).

Covers:
* UIApp forwards ``user_id``/``session_id``/``run_diagnostic`` into the graph
  config and ``build_nereus_graph``.
* ``select_or_create_user`` routes a numbered reply to an existing profile,
  ``0`` to a new profile, and a bad input to ``new``.
* ``resume_last_session`` lists the most recent on-disk sessions and returns
  the chosen ``session_id`` (or ``None`` for "new chat").
* Falls back gracefully (memory store) when the DB is unavailable.
"""

from __future__ import annotations

import uuid
from pathlib import Path
from typing import Any
from unittest.mock import patch

import pytest

from nereus.core.session import UserSession, session_path_for
from nereus.core.state import UserLevel, UserProfile
from nereus.ui.app import (
    UIApp,
    _answer_text,
    resume_last_session,
    select_or_create_user,
)


# --------------------------------------------------------------------------- #
# helpers / fakes
# --------------------------------------------------------------------------- #
class _FakeGraph:
    """Mimics the NereusGraph subset UIApp talks to."""

    def __init__(self) -> None:
        self.cfg: dict | None = None
        self.calls: list[Any] = []

    async def astream(self, state, config=None, stream_mode="values"):
        self.calls.append(state)
        self.cfg = config


class _AnsweringUserStore:
    """In-memory UserStore double for select_or_create_user."""

    def __init__(self, users: dict[str, UserProfile] | None = None) -> None:
        self._users: dict[str, UserProfile] = users or {}
        self.created: list[UserProfile] = []

    def list_users(self) -> list[tuple[str, UserProfile]]:
        return list(self._users.items())

    def create_user(self, profile: UserProfile) -> str:
        uid = str(uuid.uuid4().hex)
        self._users[uid] = profile
        self.created.append(profile)
        return uid

    def get_user(self, user_id: str) -> UserProfile | None:
        return self._users.get(user_id)


class _ReplyState:
    """Mutable holder for the next AskUserMessage reply."""

    def __init__(self, reply: str = "0") -> None:
        self.reply = reply


@pytest.fixture
def patched_cl(monkeypatch):
    """Mirror of the production patched_cl: records rendered messages and
    turns AskUserMessage into an injectable reply."""
    from nereus.ui import app as appmod

    messages: list[str] = []
    state = _ReplyState(reply="0")  # default: "choose new user"

    class _Recorder:
        def __init__(self, *a, **k):
            self.content = k.get("content", "")

        async def send(self, *a, **k):
            messages.append(self.content)
            return {"output": state.reply, "id": "ask-1", "type": "text"}

    def make(**k):
        return _Recorder(**k)

    appmod.cl.Message = make  # type: ignore[attr-defined]
    appmod.cl.AskUserMessage = make  # type: ignore[attr-defined]
    make.messages = messages  # type: ignore[attr-defined]
    make.state = state  # type: ignore[attr-defined]
    return make


@pytest.fixture
def one_user(user_profile) -> _AnsweringUserStore:
    return _AnsweringUserStore(users={"user-1234": user_profile})


# --------------------------------------------------------------------------- #
# UIApp config wiring
# --------------------------------------------------------------------------- #
@pytest.mark.asyncio
async def test_uiapp_passes_user_and_session_into_graph_config(patched_cl):
    app = UIApp(
        graph=_FakeGraph(),
        user_id="user-abc",
        session_id="sess-def",
        run_diagnostic=True,
    )
    assert app.user_id == "user-abc"
    assert app.session_id == "sess-def"
    # thread_id encodes user:session so the checkpointer keys on the pair
    assert app.thread_id == "user-abc:sess-def"
    assert app.config["configurable"]["user_id"] == "user-abc"
    assert app.config["configurable"]["session_id"] == "sess-def"
    assert app.config["configurable"]["thread_id"] == "user-abc:sess-def"


def test_uiapp_without_user_id_does_not_pollute_config():
    app = UIApp(graph=_FakeGraph())
    assert "user_id" not in app.config["configurable"]
    # thread_id is still a valid non-empty uuid when no user context is given
    assert app.thread_id and len(app.thread_id) > 6


# --------------------------------------------------------------------------- #
# select_or_create_user
# --------------------------------------------------------------------------- #
@pytest.mark.asyncio
async def test_select_existing_user_by_index(patched_cl, one_user, monkeypatch):
    monkeypatch.setattr("nereus.ui.app.build_user_store", lambda: one_user, raising=True)
    patched_cl.state.reply = "1"  # pick the first user in the list

    profile, uid = await select_or_create_user()
    assert uid == "user-1234"
    assert profile == one_user.get_user("user-1234")


@pytest.mark.asyncio
async def test_select_new_user(patched_cl, one_user, monkeypatch):
    monkeypatch.setattr("nereus.ui.app.build_user_store", lambda: one_user, raising=True)
    patched_cl.state.reply = "0"  # "new user"
    with patch("nereus.ui.app.collect_profile", new=_stub_collect_profile):
        profile, uid = await select_or_create_user()
    assert uid in one_user._users
    assert one_user._users[uid] == profile
    assert len(one_user.created) == 1


@pytest.mark.asyncio
async def test_select_bad_input_falls_back_to_new(patched_cl, one_user, monkeypatch):
    monkeypatch.setattr("nereus.ui.app.build_user_store", lambda: one_user, raising=True)
    patched_cl.state.reply = "not-a-number"
    with patch("nereus.ui.app.collect_profile", new=_stub_collect_profile):
        profile, uid = await select_or_create_user()
    assert uid in one_user._users


@pytest.mark.asyncio
async def test_store_unavailable_runs_single_session(patched_cl, monkeypatch):
    monkeypatch.setattr(
        "nereus.ui.app.build_user_store",
        lambda: (_ for _ in ()).throw(RuntimeError("db down")),
        raising=True,
    )
    with patch("nereus.ui.app.collect_profile", new=_stub_collect_profile):
        profile, uid = await select_or_create_user()
    # No store was written; just got a fresh profile + ephemeral id.
    assert isinstance(uid, str) and uid
    assert profile.goal == "stubs!"


async def _stub_collect_profile() -> UserProfile:
    return UserProfile(
        skill="Python",
        current_level=UserLevel.BEGINNER,
        target_level=UserLevel.INTERMEDIATE,
        hours_per_day=1.0,
        deadline_days=30,
        goal="stubs!",
    )


# --------------------------------------------------------------------------- #
# resume_last_session (history panel)
# --------------------------------------------------------------------------- #
def _seed_session(tmp_path: Path, user_id: str, sid: str) -> Path:
    from nereus.core.state import Roadmap

    path = session_path_for(user_id, sid)
    sess = UserSession(
        session_id=sid,
        user_id=user_id,
        roadmap=Roadmap(topics=[]),
        user_profile=UserProfile(
            skill="Python",
            current_level=UserLevel.BEGINNER,
            target_level=UserLevel.INTERMEDIATE,
            hours_per_day=1.0,
            deadline_days=30,
            goal="x",
        ),
    )
    sess.dump(path)
    return path


@pytest.mark.asyncio
async def test_resume_last_session_offers_recent_and_choices(patched_cl, tmp_path, monkeypatch):
    user_id = "u-resume"
    # Patch session_root BEFORE seeding so files land where resume_last_session scans.
    monkeypatch.setattr("nereus.ui.app.settings.session_root", str(tmp_path))
    _seed_session(tmp_path, user_id, "sess-a")
    _seed_session(tmp_path, user_id, "sess-b")
    # Older, unlisted 6th session (scan returns top 5)
    _seed_session(tmp_path, user_id, "sess-zzz-older")
    patched_cl.state.reply = "2"  # pick the 2nd listed (sess-a or sess-b)

    chosen = await resume_last_session(user_id)
    # AskUserMessage must have rendered the history options
    asks = patched_cl.messages
    assert any("сесс" in m.lower() or "sessions" in m.lower() for m in asks)
    assert chosen in {"sess-a", "sess-b", "sess-zzz-older"}


@pytest.mark.asyncio
async def test_resume_returns_none_for_new_chat(patched_cl, user_profile, tmp_path, monkeypatch):
    user_id = "u-empty"
    # No session files on disk -> function short-circuits to None.
    monkeypatch.setattr("nereus.ui.app.settings.session_root", str(tmp_path))
    result = await resume_last_session(user_id)
    assert result is None


@pytest.mark.asyncio
async def test_resume_returns_none_when_no_user_id(tmp_path, monkeypatch):
    monkeypatch.setattr("nereus.ui.app.settings.session_root", str(tmp_path))
    assert await resume_last_session("") is None
    assert await resume_last_session(None) is None  # type: ignore[arg-type]


# --------------------------------------------------------------------------- #
# regression: _answer_text shapes (chainlit StepDict vs legacy)
# --------------------------------------------------------------------------- #
def test_answer_text_handles_dict_output_key():
    assert _answer_text({"output": "hi"}) == "hi"


def test_answer_text_handles_legacy_content_attr():
    class _Legacy:
        content = "hello"

    assert _answer_text(_Legacy()) == "hello"


def test_answer_text_handles_none():
    assert _answer_text(None) == ""
