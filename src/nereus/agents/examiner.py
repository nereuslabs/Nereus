from __future__ import annotations

import re
from typing import Callable, Optional

from nereus.agents.base import BaseAgent
from nereus.core.state import Assessment, NereusState, Verdict

# score (0-100), feedback, weak areas
SubmissionCheck = tuple[float, str, list[str]]
Evaluator = Callable[[str], SubmissionCheck]


def default_evaluator(submission: str) -> SubmissionCheck:
    """Simple mock evaluator: keywords decide the outcome.

    Deterministic and dependency-free so the automaton can be tested before
    any real LLM integration exists. Word-boundary matching prevents
    accidental matches like "goodness" matching "good".
    """
    text = submission.lower()
    if re.search(r"\bgood\b", text):
        return 90.0, "Well done, topic mastered.", []
    if re.search(r"\bpartial\b", text):
        return 60.0, "Mostly correct, some gaps remain.", ["details"]
    return 35.0, "Needs more work on fundamentals.", ["fundamentals", "practice"]


class ExaminerAgent(BaseAgent):
    """Examiner Agent (MVP stub).

    Receives the user's submission for the current task, evaluates it and
    produces an ``Assessment`` with a ``PASS``/``RETRY`` verdict plus feedback
    and weak areas used by the router to decide the next step.
    """

    def __init__(self, evaluator: Optional[Evaluator] = None) -> None:
        self._evaluator = evaluator or default_evaluator

    def assess(self, submission: str, state: NereusState) -> dict:
        topic_id = state["roadmap"].topics[state["current_topic_index"]].id
        score, feedback, weak_areas = self._evaluator(submission)
        verdict = Verdict.PASS if score >= 70.0 else Verdict.RETRY

        assessment = Assessment(
            topic_id=topic_id,
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
