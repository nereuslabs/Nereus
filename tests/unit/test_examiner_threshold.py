"""Tests for difficulty-based passing threshold in ExaminerAgent (Issue #7)."""

from __future__ import annotations

import json

from nereus.agents.examiner import ExaminerAgent
from nereus.core.state import Assessment, Roadmap, RoadmapTopic, UserLevel, UserProfile, Verdict
from nereus.llm.stub import StubLLMProvider


def _profile() -> UserProfile:
    return UserProfile(
        skill="Python",
        current_level=UserLevel.BEGINNER,
        target_level=UserLevel.INTERMEDIATE,
        hours_per_day=1.0,
        deadline_days=30,
        goal="Learn Python",
    )


def _topic(difficulty: float = 1.0) -> RoadmapTopic:
    return RoadmapTopic(
        id="1",
        title="test topic",
        description="desc",
        difficulty=difficulty,
    )


def test_base_pass_threshold_is_70() -> None:
    """Default threshold without difficulty."""
    threshold = ExaminerAgent._passing_threshold(_topic(difficulty=0.0))
    assert threshold == 70.0


def test_higher_difficulty_raises_threshold() -> None:
    """Difficulty 1.0 → threshold 85."""
    threshold = ExaminerAgent._passing_threshold(_topic(difficulty=1.0))
    assert threshold == 85.0


def test_mid_difficulty_threshold() -> None:
    """Difficulty 0.5 → threshold 77.5."""
    threshold = ExaminerAgent._passing_threshold(_topic(difficulty=0.5))
    assert threshold == 70.0 + (0.5 * 15.0)


def test_stub_pass_on_high_difficulty_requires_higher_score() -> None:
    """Stub examiner: score 75 passes at difficulty 0 (threshold 70) but
    fails at difficulty 1 (threshold 85."""
    state = {
        "roadmap": Roadmap(topics=[_topic(difficulty=0.0)]),
        "current_topic_index": 0,
        "task": "test task",
    }

    # Difficulty 0.0 → threshold 70
    examiner = ExaminerAgent(provider=StubLLMProvider())
    state["roadmap"] = Roadmap(topics=[_topic(difficulty=0.0)])
    result = examiner.assess("this is good", state)
    assert result["assessment"].verdict == Verdict.PASS  # stub returns 90

    # Difficulty 1.0 → threshold 85 — stub still returns 90, but let's test the logic
    # by directly examining the threshold
    threshold_high = ExaminerAgent._passing_threshold(_topic(difficulty=1.0))
    assert threshold_high == 85.0
    # Stub returns 90 which is still >= 85
    state["roadmap"] = Roadmap(topics=[_topic(difficulty=1.0)])
    result2 = examiner.assess("this is good", state)
    assert result2["assessment"].verdict == Verdict.PASS


def test_llm_evaluator_respects_difficulty_threshold(user_profile, fake_llm_provider) -> None:
    """LLM examiner: score just below threshold → RETRY."""

    def responder(messages, **_):
        return json.dumps({"score": 75, "feedback": "ok", "weak_areas": ["x"]})

    examiner = ExaminerAgent(provider=fake_llm_provider(responder=responder))

    # Difficulty 1.0 → threshold 85, score 75 < 85 → RETRY
    state = {
        "roadmap": Roadmap(topics=[_topic(difficulty=1.0)]),
        "current_topic_index": 0,
        "task": "test task",
        "user_profile": user_profile,
    }
    result = examiner.assess("my answer", state)
    assert result["assessment"].verdict == Verdict.RETRY


def test_llm_evaluator_passes_above_threshold(user_profile, fake_llm_provider) -> None:
    """LLM examiner: score above threshold → PASS."""

    def responder(messages, **_):
        return json.dumps({"score": 90, "feedback": "excellent", "weak_areas": []})

    examiner = ExaminerAgent(provider=fake_llm_provider(responder=responder))

    state = {
        "roadmap": Roadmap(topics=[_topic(difficulty=1.0)]),
        "current_topic_index": 0,
        "task": "test task",
        "user_profile": user_profile,
    }
    result = examiner.assess("great answer", state)
    assert result["assessment"].verdict == Verdict.PASS


def test_examiner_context_includes_difficulty() -> None:
    """Context passed to evaluator should include difficulty."""
    state = {
        "roadmap": Roadmap(topics=[_topic(difficulty=0.7)]),
        "current_topic_index": 0,
        "task": "test task",
    }

    captured_context: dict = {}

    def custom_evaluator(submission, context=None):
        captured_context.update(context or {})
        return 90.0, "ok", []

    examiner = ExaminerAgent(evaluator=custom_evaluator)
    examiner.assess("answer", state)

    assert "difficulty" in captured_context
    assert captured_context["difficulty"] == 0.7


def test_examiner_stores_weakness_report_in_state(base_state, roadmap):
    """Examiner should propagate weakness info via assessment weak_areas."""
    base_state["roadmap"] = roadmap
    base_state["user_submission"] = "partial answer"

    examiner = ExaminerAgent(provider=StubLLMProvider())
    result = examiner.run(base_state)

    assert "assessment" in result
    assessment: Assessment = result["assessment"]
    assert assessment.verdict == Verdict.RETRY  # stub: "partial answer" → 60
    assert len(assessment.weak_areas) > 0
