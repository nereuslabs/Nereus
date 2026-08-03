from __future__ import annotations

from nereus.agents.base import BaseAgent
from nereus.core.state import (
    LearningStatus,
    NereusState,
    Roadmap,
    RoadmapTopic,
    UserProfile,
)


class CoachAgent(BaseAgent):
    """Coach Agent (MVP stub).

    Collects the user's skill, goals and constraints into a ``UserProfile``
    and builds a basic ``Roadmap`` of topics to master.
    """

    def build_roadmap(self, profile: UserProfile) -> Roadmap:
        topics = [
            RoadmapTopic(
                id="1",
                title=f"{profile.skill}: fundamentals",
                description="Core concepts and terminology.",
            ),
            RoadmapTopic(
                id="2",
                title=f"{profile.skill}: practice",
                description="Hands-on exercises and applied tasks.",
            ),
            RoadmapTopic(
                id="3",
                title=f"{profile.skill}: advanced",
                description="Advanced topics and best practices.",
            ),
        ]
        return Roadmap(topics=topics)

    def run(self, state: NereusState) -> dict:
        profile = state["user_profile"]
        if profile is None:
            raise ValueError("CoachAgent requires a user_profile in state")

        roadmap = self.build_roadmap(profile)
        return {
            "roadmap": roadmap,
            "current_topic_index": 0,
            "retry_count": 0,
            "assessment": None,
            "material": "",
            "task": "",
            "status": LearningStatus.LEARNING,
        }


class MockCoachAgent(CoachAgent):
    """Identical to CoachAgent; exists to make the mock nature explicit."""
