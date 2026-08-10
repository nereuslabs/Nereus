"""Tests for adaptive roadmap generation (Issue #7)."""

from __future__ import annotations

import json

from nereus.agents.coach import CoachAgent
from nereus.core.state import (
    UserLevel,
    UserProfile,
    WeaknessReport,
)
from nereus.llm.stub import StubLLMProvider


def _profile() -> UserProfile:
    return UserProfile(
        skill="Python",
        current_level=UserLevel.BEGINNER,
        target_level=UserLevel.INTERMEDIATE,
        hours_per_day=1.0,
        deadline_days=30,
        goal="Learn Python",
    )


def test_stub_roadmap_with_weakness_report_prioritizes_gaps() -> None:
    """Stub mode: weakness_report with recommended_topics biases roadmap."""
    coach = CoachAgent(provider=StubLLMProvider())
    report = WeaknessReport(
        weak_areas=["basics", "syntax"],
        recommended_topics=["1"],
    )
    roadmap = coach.build_roadmap(_profile(), weakness_report=report)

    assert len(roadmap.topics) >= 3
    # First topics should target weak areas
    first_topic = roadmap.topics[0]
    assert "diagnostic" in first_topic.title.lower() or "review" in first_topic.title.lower()
    assert first_topic.difficulty >= 0.5  # targeted weak area = higher difficulty


def test_stub_roadmap_without_weakness_uses_default() -> None:
    """Stub mode without weakness report: default 3-topic roadmap with difficulty=1.0."""
    coach = CoachAgent(provider=StubLLMProvider())
    roadmap = coach.build_roadmap(_profile())

    assert len(roadmap.topics) == 3
    assert roadmap.topics[0].title == "Python: fundamentals"
    assert roadmap.topics[0].difficulty == 1.0  # default
    assert roadmap.topics[0].prerequisites == []


def test_stub_roadmap_respects_prerequisites() -> None:
    """Topics should have prerequisites set correctly."""
    coach = CoachAgent(provider=StubLLMProvider())
    report = WeaknessReport(
        weak_areas=["data types"],
        recommended_topics=["1"],
    )
    roadmap = coach.build_roadmap(_profile(), weakness_report=report)

    for i, topic in enumerate(roadmap.topics):
        if i > 0:
            assert len(topic.prerequisites) >= 1
            assert int(topic.prerequisites[0]) <= i  # prereq is earlier topic


def test_stub_roadmap_has_estimated_hours() -> None:
    """Each topic should have estimated_hours set."""
    coach = CoachAgent(provider=StubLLMProvider())
    roadmap = coach.build_roadmap(_profile())

    for topic in roadmap.topics:
        assert topic.estimated_hours >= 0.0


def test_llm_roardmap_with_weakness_report(user_profile, fake_llm_provider) -> None:
    """Real LLM path: coach prompt includes weakness_report."""

    def responder(messages, **_):
        # Return adaptive roadmap with difficulty and prerequisites
        return json.dumps(
            {
                "topics": [
                    {
                        "id": "1",
                        "title": "Python basics",
                        "description": "Basics",
                        "difficulty": 0.3,
                        "prerequisites": [],
                        "estimated_hours": 2.0,
                    },
                    {
                        "id": "2",
                        "title": "Data types",
                        "description": "Types",
                        "difficulty": 0.6,
                        "prerequisites": ["1"],
                        "estimated_hours": 3.0,
                    },
                ]
            }
        )

    coach = CoachAgent(provider=fake_llm_provider(responder=responder))
    report = WeaknessReport(weak_areas=["syntax"], recommended_topics=["1"])

    roadmap = coach.build_roadmap_llm(_profile(), weakness_report=report)

    assert len(roadmap.topics) == 2
    assert roadmap.topics[0].difficulty == 0.3
    assert roadmap.topics[1].prerequisites == ["1"]
    assert roadmap.topics[1].estimated_hours == 3.0


def test_coach_run_uses_weakness_from_state() -> None:
    """CoachAgent.run() picks up weakness_report from state."""
    coach = CoachAgent(provider=StubLLMProvider())
    profile = _profile()
    report = WeaknessReport(weak_areas=["basics"], recommended_topics=["1"])

    state = {
        "user_profile": profile,
        "weakness_report": report,
    }
    result = coach.run(state)

    assert result["roadmap"].topics[0].difficulty >= 0.5
    assert "roadmap" in result
    assert "current_topic_index" in result
