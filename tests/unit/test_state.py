from __future__ import annotations

import pytest
from pydantic import ValidationError

from nereus.core.state import (
    LearningStatus,
    Roadmap,
    RoadmapTopic,
    UserLevel,
    UserProfile,
    Verdict,
)


def test_roadmap_defaults_to_empty_topics() -> None:
    assert Roadmap().topics == []


def test_roadmap_topics_validated() -> None:
    roadmap = Roadmap(topics=[RoadmapTopic(id="1", title="T", description="D")])
    assert roadmap.topics[0].title == "T"


def test_profile_validation_rejects_negative_hours() -> None:
    with pytest.raises(ValidationError):
        UserProfile(
            skill="Python",
            current_level=UserLevel.BEGINNER,
            target_level=UserLevel.INTERMEDIATE,
            hours_per_day=-1.0,
            deadline_days=30,
            goal="Learn Python.",
        )


def test_profile_validation_rejects_zero_deadline() -> None:
    with pytest.raises(ValidationError):
        UserProfile(
            skill="Python",
            current_level=UserLevel.BEGINNER,
            target_level=UserLevel.INTERMEDIATE,
            hours_per_day=1.0,
            deadline_days=0,
            goal="Learn Python.",
        )


def test_verdict_and_status_enums() -> None:
    assert Verdict.PASS.value == "pass"
    assert Verdict.RETRY.value == "retry"
    assert LearningStatus.LEARNING.value == "learning"
