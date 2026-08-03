from __future__ import annotations

import pytest

from nereus.core.state import Roadmap, RoadmapTopic, UserLevel, UserProfile


@pytest.fixture
def user_profile() -> UserProfile:
    return UserProfile(
        skill="Python",
        current_level=UserLevel.BEGINNER,
        target_level=UserLevel.INTERMEDIATE,
        hours_per_day=1.0,
        deadline_days=30,
        goal="Get comfortable writing scripts.",
    )


@pytest.fixture
def roadmap() -> Roadmap:
    return Roadmap(
        topics=[
            RoadmapTopic(
                id="1",
                title="Python: fundamentals",
                description="Syntax and core concepts.",
            ),
            RoadmapTopic(
                id="2",
                title="Python: practice",
                description="Hands-on exercises.",
            ),
            RoadmapTopic(
                id="3",
                title="Python: advanced",
                description="Advanced topics.",
            ),
        ]
    )


@pytest.fixture
def base_state(user_profile: UserProfile) -> dict:
    return {
        "user_profile": user_profile,
        "roadmap": Roadmap(),
        "current_topic_index": 0,
        "material": "",
        "task": "",
        "user_submission": None,
        "assessment": None,
        "retry_count": 0,
        "max_retries": 2,
        "status": "coaching",
        "messages": [],
    }
