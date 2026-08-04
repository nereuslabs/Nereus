from __future__ import annotations

import re
from typing import Any, Callable, Optional

from nereus.agents.base import BaseAgent
from nereus.core.session import LearningSession
from nereus.core.state import Assessment, NereusState, Verdict
from nereus.llm.inference import LLMOutputError, StructuredInferenceClient
from nereus.llm.params import AgentRole
from nereus.llm.prompts import build_examiner_prompt
from nereus.llm.schema import AssessmentOutput

# score (0-100), feedback, weak areas
SubmissionCheck = tuple[float, str, list[str]]
# (submission, context) -> check; context carries task/topic info
Evaluator = Callable[[str, dict[str, Any]], SubmissionCheck]


def default_evaluator(
    submission: str, context: dict[str, Any] | None = None
) -> SubmissionCheck:
    """Deterministic fallback evaluator based on keywords.

    Dependency-free so the automaton can be tested before any real LLM
    integration exists. Word-boundary matching prevents accidental matches
    like "goodness" matching "good". Used when no LLM provider is configured.
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
    return a validated ``AssessmentOutput``. Falls back to
    :func:`default_evaluator` if the model output cannot be parsed after
    retries.
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
            topic_title=topic, task=task, submission=submission, session=session
        )
        try:
            result: AssessmentOutput = self._inference.generate(
                messages, role=AgentRole.EXAMINER, output_model=AssessmentOutput
            )
            return float(result.score), str(result.feedback), list(result.weak_areas)
        except LLMOutputError:
            return default_evaluator(submission, context)


class ExaminerAgent(BaseAgent):
    """Examiner Agent.

    Receives the user's submission for the current task, evaluates it (via an
    injected evaluator — LLM or deterministic fallback) and produces an
    ``Assessment`` with a ``PASS``/``RETRY`` verdict plus feedback and weak
    areas used by the router to decide the next step.
    """

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

    def assess(self, submission: str, state: NereusState) -> dict:
        topic = state["roadmap"].topics[state["current_topic_index"]]
        context = {
            "task": state["task"],
            "topic": topic.title,
            "session": state.get("session"),
        }
        score, feedback, weak_areas = self._evaluator(submission, context)
        verdict = Verdict.PASS if score >= 70.0 else Verdict.RETRY

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
