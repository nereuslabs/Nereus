from __future__ import annotations

import logging
from typing import Any

from pydantic import BaseModel

from nereus.agents.base import BaseAgent
from nereus.core.session import LearningSession
from nereus.core.state import (
    LearningStatus,
    NereusState,
    Roadmap,
    RoadmapTopic,
    UserProfile,
    WeaknessReport,
)
from nereus.llm.inference import StructuredInferenceClient, is_offline_inference
from nereus.llm.params import AgentRole
from nereus.llm.prompts import build_coach_prompt
from nereus.llm.schema import AdaptiveRoadmapOutput


class CoachAgent(BaseAgent):
    """Coach Agent.

    Collects the user's skill, goals and constraints into a ``UserProfile``
    and builds a ``Roadmap`` of topics. Offline (``StubLLMProvider`` / no
    inference client) uses deterministic stub generation; a real provider
    generates the roadmap via the model with schema validation and bounded
    retries, raising :class:`LLMUnavailableError` (surfaced as
    "service temporarily unavailable") when it cannot produce one (#45).
    """

    logger = logging.getLogger("nereus.agents.coach")

    def __init__(
        self,
        *,
        inference: StructuredInferenceClient | None = None,
        provider: Any = None,
    ) -> None:
        self._provider = provider
        if inference is not None:
            self._inference = inference
        elif provider is not None:
            self._inference = StructuredInferenceClient(provider)
        else:
            self._inference = None

    # ------------------------------------------------------------------ #
    # Deterministic fallback (offline / StubLLMProvider only)            #
    # ------------------------------------------------------------------ #
    def build_roadmap(
        self,
        profile: UserProfile,
        weakness_report: WeaknessReport | None = None,
    ) -> Roadmap:
        """Build a roadmap, optionally biased by diagnostic weakness_report."""
        if weakness_report is not None and weakness_report.recommended_topics:
            # Adaptive stub: prioritize weak areas with harder topics early
            topics = []
            weak_set = set(weakness_report.weak_areas)

            # First, add topics targeting weak areas
            for i, area in enumerate(sorted(weak_set), 1):
                topics.append(
                    RoadmapTopic(
                        id=str(i),
                        title=f"{profile.skill}: {area} (diagnostic review)",
                        description=f"Focus area identified by diagnostic: {area}.",
                        difficulty=0.8,
                        prerequisites=[],
                        estimated_hours=2.0,
                    )
                )

            # Fill remaining slots with standard progression
            remaining = 3 - len(topics)
            if remaining > 0:
                base_titles = [
                    "fundamentals",
                    "practice",
                    "advanced",
                ]
                for j in range(remaining):
                    idx = len(topics) + 1
                    topics.append(
                        RoadmapTopic(
                            id=str(idx),
                            title=f"{profile.skill}: {base_titles[j % len(base_titles)]}",
                            description="Core concepts and applied tasks.",
                            difficulty=0.3 + (j * 0.3),
                            prerequisites=[str(idx - 1)] if idx > 1 else [],
                            estimated_hours=2.0,
                        )
                    )
            # Ensure at least 3 topics
            if len(topics) < 3:
                topics.append(
                    RoadmapTopic(
                        id="3",
                        title=f"{profile.skill}: advanced",
                        description="Advanced topics and best practices.",
                        difficulty=0.9,
                        prerequisites=[str(len(topics))],
                        estimated_hours=2.0,
                    )
                )
            return Roadmap(topics=topics)

        # Default stub roadmap (no diagnostic)
        return Roadmap(
            topics=[
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
        )

    # ------------------------------------------------------------------ #
    # LLM-backed roadmap                                                #
    # ------------------------------------------------------------------ #
    def build_roadmap_llm(
        self,
        profile: UserProfile,
        session=None,
        weakness_report: WeaknessReport | None = None,
    ) -> Roadmap:
        if is_offline_inference(self._inference):
            return self.build_roadmap(profile, weakness_report=weakness_report)
        messages = build_coach_prompt(session=session, weakness_report=weakness_report)
        result: BaseModel = self._inference.generate(
            messages, role=AgentRole.COACH, output_model=AdaptiveRoadmapOutput
        )
        topics = [
            RoadmapTopic(
                id=topic.id,
                title=topic.title,
                description=topic.description,
                difficulty=topic.difficulty,
                prerequisites=topic.prerequisites,
                estimated_hours=topic.estimated_hours,
            )
            for topic in result.topics  # type: ignore[attr-defined]
        ]
        if not topics:
            return self.build_roadmap(profile, weakness_report=weakness_report)
        return Roadmap(topics=topics)

    # ------------------------------------------------------------------ #
    # Agent interface                                                    #
    # ------------------------------------------------------------------ #
    def run(self, state: NereusState) -> dict:
        profile = state["user_profile"]
        if profile is None:
            raise ValueError("CoachAgent requires a user_profile in state")

        session = state.get("session")
        weakness_report = state.get("weakness_report")
        roadmap = self.build_roadmap_llm(profile, session=session, weakness_report=weakness_report)

        result: dict = {
            "roadmap": roadmap,
            "current_topic_index": 0,
            "retry_count": 0,
            "assessment": None,
            "material": "",
            "task": "",
            "status": LearningStatus.LEARNING.value,
        }
        new_session = LearningSession().update_from_state(state, own_output=result)
        return {
            **result,
            "session": new_session,
            "session_brief": new_session.to_brief(),
            "messages": [
                {
                    "role": "system",
                    "content": (
                        "[Coach] Roadmap built with "
                        f"{len(roadmap.topics)} topics: "
                        + ", ".join(t.title for t in roadmap.topics)
                    ),
                }
            ],
        }
