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
)
from nereus.llm.inference import StructuredInferenceClient, is_offline_inference
from nereus.llm.params import AgentRole
from nereus.llm.prompts import build_coach_prompt
from nereus.llm.schema import RoadmapOutput


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

    # ------------------------------------------------------------------ #
    # LLM-backed roadmap                                                #
    # ------------------------------------------------------------------ #
    def build_roadmap_llm(self, profile: UserProfile, session=None) -> Roadmap:
        if is_offline_inference(self._inference):
            return self.build_roadmap(profile)
        messages = build_coach_prompt(session=session)
        result: BaseModel = self._inference.generate(
            messages, role=AgentRole.COACH, output_model=RoadmapOutput
        )
        topics = [
            RoadmapTopic(
                id=topic.id, title=topic.title, description=topic.description
            )
            for topic in result.topics  # type: ignore[attr-defined]
        ]
        if not topics:
            return self.build_roadmap(profile)
        return Roadmap(topics=topics)

    # ------------------------------------------------------------------ #
    # Agent interface                                                    #
    # ------------------------------------------------------------------ #
    def run(self, state: NereusState) -> dict:
        profile = state["user_profile"]
        if profile is None:
            raise ValueError("CoachAgent requires a user_profile in state")

        session = state.get("session")
        roadmap = self.build_roadmap_llm(profile, session=session)

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
