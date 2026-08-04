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
        goal="x",
    )


def _roadmap() -> Roadmap:
    return Roadmap(
        topics=[
            RoadmapTopic(id="1", title="fund", description="d"),
            RoadmapTopic(id="2", title="practice", description="d"),
        ]
    )


def test_update_from_state_seeds_profile_and_roadmap() -> None:
    session = LearningSession().update_from_state(
        {"user_profile": _profile(), "roadmap": _roadmap(), "current_topic_index": 0}
    )
    assert session.user_profile.skill == "Python"
    assert len(session.roadmap.topics) == 2
    assert session.current_topic_index == 0


def test_record_assessment_tracks_retries_and_weak_areas() -> None:
    session = LearningSession()
    state = {"roadmap": _roadmap(), "current_topic_index": 0}
    a1 = Assessment(
        topic_id="1",
        score=40.0,
        verdict=Verdict.RETRY,
        feedback="weak",
        weak_areas=["syntax", "loops"],
    )
    s1 = session.update_from_state({**state, "assessment": a1})
    assert s1.retry_counts["1"] == 1
    assert s1.aggregated_weak_areas == {"syntax": 1, "loops": 1}
    assert s1.completed[0].verdict == Verdict.RETRY

    a2 = Assessment(
        topic_id="1",
        score=85.0,
        verdict=Verdict.PASS,
        feedback="ok",
        weak_areas=[],
    )
    s2 = s1.update_from_state({**state, "assessment": a2})
    assert len(s2.completed) == 1
    assert s2.completed[0].score == 85.0
    assert s2.retry_counts["1"] == 0
    assert s2.aggregated_weak_areas == {"syntax": 1, "loops": 1}


def test_to_brief_contains_profile_progress_and_weak_areas() -> None:
    session = LearningSession(
        user_profile=_profile(), roadmap=_roadmap(), current_topic_index=1
    )
    session.record_assessment(
        Assessment(
            topic_id="1",
            score=40.0,
            verdict=Verdict.RETRY,
            feedback="weak",
            weak_areas=["syntax"],
        )
    )
    brief = session.to_brief()
    assert "Skill: Python" in brief
    assert "beginner -> intermediate" in brief
    assert "Roadmap: 2 topics; current: 2/2" in brief
    assert "syntax (x1)" in brief
    assert "Retries: 1 x1" in brief


def test_to_brief_handles_empty_session() -> None:
    brief = LearningSession().to_brief()
    assert "Profile: not set" in brief
    assert "Completed: none" in brief
