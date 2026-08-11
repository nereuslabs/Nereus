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
    Verdict,
)
from nereus.llm.inference import StructuredInferenceClient, is_offline_inference
from nereus.llm.params import AgentRole
from nereus.llm.prompts import build_tutor_prompt
from nereus.llm.schema import MaterialOutput

logger = logging.getLogger("nereus.agents.tutor")


class TutorAgent(BaseAgent):
    """Tutor Agent.

    Provides learning material and a task for the current roadmap topic. Offline
    (``StubLLMProvider`` / no inference client) uses deterministic stub
    material; a real provider generates via the model with retries, raising
    :class:`LLMUnavailableError` (surfaced as "service temporarily unavailable")
    when it cannot (#45). On a ``RETRY`` verdict it produces a revision block
    focused on the reported weak areas.
    """

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
    def _topic_material(self, topic: RoadmapTopic) -> str:
        return f"Материал по теме «{topic.title}»: {topic.description}"

    def _task_for(self, topic: RoadmapTopic) -> str:
        return f"Практическое задание: продемонстрируйте умение темы «{topic.title}»."

    def _revision_material(self, state: NereusState) -> str:
        assessment = state.get("assessment")
        weak = ", ".join(assessment.weak_areas) if assessment else "тема"
        return f"Материал для повторения, углубляющий {weak}."

    # ------------------------------------------------------------------ #
    # LLM-backed generation                                             #
    # ------------------------------------------------------------------ #
    def _generate(
        self,
        topic: RoadmapTopic,
        *,
        revision: bool,
        weak_areas: list[str],
        session=None,
    ) -> tuple[str, str]:
        messages = build_tutor_prompt(
            topic, revision=revision, weak_areas=weak_areas, session=session
        )
        result: BaseModel = self._inference.generate(
            messages, role=AgentRole.TUTOR, output_model=MaterialOutput
        )
        return str(result.material), str(result.task)  # type: ignore[attr-defined]

    # ------------------------------------------------------------------ #
    # Agent interface                                                    #
    # ------------------------------------------------------------------ #
    def run(self, state: NereusState) -> dict:
        roadmap: Roadmap = state["roadmap"]
        index = state["current_topic_index"]
        topic = roadmap.topics[index]
        assessment = state.get("assessment")
        retrying = assessment is not None and assessment.verdict == Verdict.RETRY
        weak_areas = assessment.weak_areas if assessment else []
        session = state.get("session")

        if is_offline_inference(self._inference):
            material = self._revision_material(state) if retrying else self._topic_material(topic)
            task = self._task_for(topic)
        else:
            # Real provider: retries happen inside generate(); a persistent
            # failure raises LLMUnavailableError (surfaced as "service
            # unavailable"), never a silent stub (#45).
            material, task = self._generate(
                topic,
                revision=retrying,
                weak_areas=weak_areas,
                session=session,
            )

        result: dict = {
            "material": material,
            "task": task,
            "status": LearningStatus.EXAMINING.value,
        }
        return self._with_session(state, result)

    @staticmethod
    def _with_session(state: NereusState, result: dict) -> dict:
        prev = state.get("session") or LearningSession()
        new_session = prev.update_from_state(state, own_output=result)
        return {
            **result,
            "session": new_session,
            "session_brief": new_session.to_brief(),
            "messages": [
                {"role": "assistant", "content": f"[Tutor] {result['material']}"},
                {"role": "user", "content": f"[Task] {result['task']}"},
            ],
        }
