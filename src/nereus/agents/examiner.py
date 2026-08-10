from __future__ import annotations

import logging
import re
from typing import Any, Callable, Optional

from nereus.agents.base import BaseAgent
from nereus.core.session import LearningSession
from nereus.core.state import Assessment, NereusState, Verdict
from nereus.llm.inference import StructuredInferenceClient, is_offline_inference
from nereus.llm.params import AgentRole
from nereus.llm.prompts import build_examiner_prompt
from nereus.llm.schema import AssessmentOutput

logger = logging.getLogger("nereus.agents.examiner")

# score (0-100), feedback, weak areas
SubmissionCheck = tuple[float, str, list[str]]
# (submission, context) -> check; context carries task/topic info
Evaluator = Callable[[str, dict[str, Any]], SubmissionCheck]


def default_evaluator(submission: str, context: dict[str, Any] | None = None) -> SubmissionCheck:
    """Deterministic evaluator used for OFFLINE runs (no real provider).

    Dependency-free so the automaton can be exercised without any network.
    Word-boundary matching prevents accidental matches like "goodness"
    matching "good". Only used when the provider is an offline ``StubLLMProvider``
    (#44); a real, unreachable provider raises :class:`LLMUnavailableError`
    instead of faking a score.
    """
    text = submission.lower()
    if re.search(r"\bgood\b", text):
        return 90.0, "Well done, topic mastered.", []
    if re.search(r"\bpartial\b", text):
        return 60.0, "Mostly correct, some gaps remain.", ["details"]
    return 35.0, "Needs more work on fundamentals.", ["fundamentals", "practice"]


class LLMEvaluator:
    """Evaluator backed by a structured inference client.

    Asks the model to grade the user's submission against the current task and
    return a validated ``AssessmentOutput``. When the inference client is None
    or offline (``StubLLMProvider``) it falls back to :func:`default_evaluator`;
    a real provider that cannot answer after retries raises
    :class:`LLMUnavailableError`, which the UI surfaces as "service temporarily
    unavailable" instead of faking a score (#44/#45).
    """

    def __init__(self, inference: StructuredInferenceClient) -> None:
        self._inference = inference

    def __call__(
        self,
        submission: str,
        context: dict[str, Any],
    ) -> SubmissionCheck:
        topic = context.get("topic", "")
        task = context.get("task", "")
        session = context.get("session")
        messages = build_examiner_prompt(
            topic_title=topic,
            task=task,
            submission=submission,
            session=session,
            retrieved=context.get("retrieved"),
        )
        if is_offline_inference(self._inference):
            return default_evaluator(submission, context)
        result: AssessmentOutput = self._inference.generate(
            messages, role=AgentRole.EXAMINER, output_model=AssessmentOutput
        )
        return float(result.score), str(result.feedback), list(result.weak_areas)


class ExaminerAgent(BaseAgent):
    """Examiner Agent.

    Receives the user's submission for the current task, evaluates it (via an
    injected evaluator — LLM or the offline deterministic evaluator) and
    produces an ``Assessment`` with a ``PASS``/``RETRY`` verdict plus feedback
    and weak areas used by the router to decide the next step.
    """

    # Base passing threshold; adjusted by topic difficulty (Issue #7)
    BASE_PASS_THRESHOLD: float = 70.0
    MAX_DIFFICULTY_BONUS: float = 15.0  # difficulty 1.0 → threshold 85

    def __init__(
        self,
        evaluator: Optional[Evaluator] = None,
        *,
        inference: StructuredInferenceClient | None = None,
        provider: Any = None,
    ) -> None:
        if evaluator is not None:
            self._evaluator = evaluator
            self._inference: StructuredInferenceClient | None = None
        else:
            self._inference = inference
            if self._inference is None and provider is not None:
                self._inference = StructuredInferenceClient(provider)
            if self._inference is not None:
                self._evaluator = LLMEvaluator(self._inference)
            else:
                self._evaluator = default_evaluator

    @classmethod
    def _passing_threshold(cls, topic: Any) -> float:
        """Adaptive passing threshold based on topic difficulty.

        Higher difficulty topics require a higher score to pass.
        """
        difficulty = getattr(topic, "difficulty", 1.0)
        return cls.BASE_PASS_THRESHOLD + (difficulty * cls.MAX_DIFFICULTY_BONUS)

    def assess(self, submission: str, state: NereusState) -> dict:
        topic = state["roadmap"].topics[state["current_topic_index"]]
        context = {
            "task": state["task"],
            "topic": topic.title,
            "session": state.get("session"),
            "retrieved": state.get("retrieved_chunks") or [],
            "difficulty": getattr(topic, "difficulty", 1.0),
        }
        score, feedback, weak_areas = self._evaluator(submission, context)
        pass_threshold = self._passing_threshold(topic)
        verdict = Verdict.PASS if score >= pass_threshold else Verdict.RETRY

        assessment = Assessment(
            topic_id=topic.id,
            score=score,
            verdict=verdict,
            feedback=feedback,
            weak_areas=weak_areas,
        )

        result: dict = {"user_submission": submission, "assessment": assessment}
        prev = state.get("session") or LearningSession()
        new_session = prev.update_from_state(state, own_output=result)
        return {
            **result,
            "session": new_session,
            "session_brief": new_session.to_brief(),
            "messages": [
                {"role": "user", "content": f"[Submission] {submission}"},
                {
                    "role": "assistant",
                    "content": (
                        f"[Examiner] {feedback} | score={score:g} "
                        f"| verdict={verdict.value} | weak={weak_areas}"
                    ),
                },
            ],
        }

    def run(self, state: NereusState) -> dict:
        submission = state.get("user_submission")
        if submission is None:
            raise ValueError("ExaminerAgent requires a user_submission in state")
        return self.assess(submission, state)
