from __future__ import annotations

from nereus.agents.base import BaseAgent
from nereus.core.state import (
    LearningStatus,
    NereusState,
    Roadmap,
    RoadmapTopic,
    Verdict,
)


class TutorAgent(BaseAgent):
    """Tutor Agent (MVP stub).

    Provides learning material and a task for the current roadmap topic.
    When the examiner returned a ``RETRY`` verdict it produces a revision
    block focused on the reported weak areas instead of moving forward.
    """

    def _topic_material(self, topic: RoadmapTopic) -> str:
        return f"Material for '{topic.title}': {topic.description}"

    def _task_for(self, topic: RoadmapTopic) -> str:
        return f"Practical task: demonstrate mastery of '{topic.title}'."

    def _revision_material(self, state: NereusState) -> str:
        assessment = state.get("assessment")
        weak = ", ".join(assessment.weak_areas) if assessment else "the topic"
        return f"Revision material deepening {weak}."

    def run(self, state: NereusState) -> dict:
        roadmap: Roadmap = state["roadmap"]
        index = state["current_topic_index"]
        retrying = state.get("assessment") is not None and (
            state.get("assessment").verdict == Verdict.RETRY
        )

        if retrying:
            material = self._revision_material(state)
            topic = roadmap.topics[index]
            task = self._task_for(topic)
        else:
            topic = roadmap.topics[index]
            material = self._topic_material(topic)
            task = self._task_for(topic)

        return {
            "material": material,
            "task": task,
            "status": LearningStatus.EXAMINING,
        }


class MockTutorAgent(TutorAgent):
    """Identical to TutorAgent; exists to make the mock nature explicit."""
