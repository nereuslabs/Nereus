from __future__ import annotations

from nereus.core.state import RoadmapTopic
from nereus.llm.prompts import (
    SESSION_PREAMBLE,
    build_coach_prompt,
    build_examiner_prompt,
    build_summarizer_prompt,
    build_tutor_prompt,
)


class _FakeSession:
    def to_brief(self) -> str:
        return "Skill: Python | current: 1/2"


def test_coach_prompt_has_system_and_user_without_session() -> None:
    msgs = build_coach_prompt(session=None)
    assert msgs[0]["role"] == "system"
    assert "roadmap" in msgs[0]["content"].lower()
    assert SESSION_PREAMBLE not in msgs[0]["content"]
    assert msgs[1]["role"] == "user"


def test_coach_prompt_includes_session_brief() -> None:
    msgs = build_coach_prompt(session=_FakeSession())
    assert SESSION_PREAMBLE in msgs[0]["content"]
    assert "Skill: Python | current: 1/2" in msgs[0]["content"]


def test_tutor_prompt_revision_flags_weak_areas() -> None:
    topic = RoadmapTopic(id="1", title="T", description="D")
    normal = build_tutor_prompt(topic, revision=False, weak_areas=[])
    revision = build_tutor_prompt(
        topic, revision=True, weak_areas=["syntax"], session=_FakeSession()
    )
    assert "Topic title" in normal[1]["content"]
    assert "syntax" in revision[1]["content"]
    assert SESSION_PREAMBLE in revision[0]["content"]


def test_examiner_prompt_carries_task_and_submission() -> None:
    msgs = build_examiner_prompt(
        topic_title="Python: practice", task="build x", submission="my answer"
    )
    assert msgs[0]["role"] == "system"
    assert "build x" in msgs[1]["content"]
    assert "my answer" in msgs[1]["content"]
    assert "Python: practice" in msgs[1]["content"]


def test_summarizer_prompt_is_two_messages() -> None:
    msgs = build_summarizer_prompt([{"role": "user", "content": "hi"}])
    assert len(msgs) == 2
    assert msgs[0]["role"] == "system"
