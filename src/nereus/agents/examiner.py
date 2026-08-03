from __future__ import annotations

import re
from typing import Any, Callable, Optional

from nereus.agents.base import BaseAgent
from nereus.core.state import Assessment, NereusState, Verdict

# score (0-100), feedback, weak areas
SubmissionCheck = tuple[float, str, list[str]]
# (submission, context) -> check; context carries task/topic info
Evaluator = Callable[[str, dict[str, Any]], SubmissionCheck]


def default_evaluator(submission: str, context: dict[str, Any] | None = None) -> SubmissionCheck:
    """Deterministic fallback evaluator based on keywords.

    Deterministic and dependency-free so the automaton can be tested before
    any real LLM integration exists. Word-boundary matching prevents
    accidental matches like "goodness" matching "good". Used when no LLM
    provider is configured.
    """
    text = submission.lower()
    if re.search(r"\bgood\b", text):
        return 90.0, "Well done, topic mastered.", []
    if re.search(r"\bpartial\b", text):
        return 60.0, "Mostly correct, some gaps remain.", ["details"]
    return 35.0, "Needs more work on fundamentals.", ["fundamentals", "practice"]


class LLMEvaluator:
    """Evaluator backed by an LLM provider.

    Asks the model to grade the user's submission against the current task and
    return a JSON object ``{score, feedback, weak_areas}``. Falls back to
    :func:`default_evaluator` if the model output cannot be parsed.
    """

    def __init__(self, provider) -> None:
        self._provider = provider

    def _prompt(self, submission: str, context: dict[str, Any]) -> list[dict[str, Any]]:
        task = context.get("task", "")
        topic = context.get("topic", "")
        return [
            {
                "role": "system",
                "content": (
                    "You are a strict examiner for an AI tutor. Grade the student's "
                    "answer to the given task. Return ONLY valid JSON with keys: "
                    '{"score": <int 0-100>, "feedback": "<str>", "weak_areas": ["<str>", ...]}. '
                    "A passing score is >= 70."
                ),
            },
            {
                "role": "user",
                "content": f"Topic: {topic}\nTask: {task}\n\nStudent's answer: {submission}",
            },
        ]

    def __call__(self, submission: str, context: dict[str, Any]) -> SubmissionCheck:
        from nereus.llm.schema import extract_json

        try:
            raw = self._provider.complete(
                self._prompt(submission, context), temperature=0.0, json_mode=True
            )
            data = extract_json(raw)
            score = float(data["score"])
            feedback = str(data["feedback"])
            weak = [str(w) for w in data.get("weak_areas", [])]
        except Exception:
            return default_evaluator(submission, context)
        return score, feedback, weak


class ExaminerAgent(BaseAgent):
    """Examiner Agent.

    Receives the user's submission for the current task, evaluates it (via an
    injected evaluator — LLM or deterministic fallback) and produces an
    ``Assessment`` with a ``PASS``/``RETRY`` verdict plus feedback and weak
    areas used by the router to decide the next step.
    """

    def __init__(self, evaluator: Optional[Evaluator] = None, provider=None) -> None:
        self._evaluator = evaluator
        if self._evaluator is None:
            self._evaluator = (
                LLMEvaluator(provider) if provider is not None else default_evaluator
            )

    def assess(self, submission: str, state: NereusState) -> dict:
        topic = state["roadmap"].topics[state["current_topic_index"]]
        context = {"task": state["task"], "topic": topic.title}
        score, feedback, weak_areas = self._evaluator(submission, context)
        verdict = Verdict.PASS if score >= 70.0 else Verdict.RETRY

        assessment = Assessment(
            topic_id=topic.id,
            score=score,
            verdict=verdict,
            feedback=feedback,
            weak_areas=weak_areas,
        )

        return {"user_submission": submission, "assessment": assessment}

    def run(self, state: NereusState) -> dict:
        submission = state.get("user_submission")
        if submission is None:
            raise ValueError("ExaminerAgent requires a user_submission in state")
        return self.assess(submission, state)


class MockExaminerAgent(ExaminerAgent):
    """Identical to ExaminerAgent; exists to make the mock nature explicit."""
