from __future__ import annotations

import json

import pytest

from nereus.agents.coach import CoachAgent
from nereus.agents.examiner import ExaminerAgent
from nereus.agents.tutor import TutorAgent
from nereus.core.state import Verdict
from nereus.llm.inference import LLMUnavailableError
from nereus.llm.openrouter import OpenRouterError
from nereus.llm.stub import StubLLMProvider


def test_coach_builds_roadmap_via_llm(user_profile, fake_llm_provider) -> None:
    def responder(messages, **_):
        return json.dumps(
            {
                "topics": [
                    {"id": "1", "title": "A", "description": "d1"},
                    {"id": "2", "title": "B", "description": "d2"},
                ]
            }
        )

    coach = CoachAgent(provider=fake_llm_provider(responder=responder))
    roadmap = coach.build_roadmap_llm(user_profile)
    assert len(roadmap.topics) == 2
    assert roadmap.topics[0].title == "A"


def test_coach_uses_offline_roadmap_when_stub_provider(user_profile) -> None:
    coach = CoachAgent(provider=StubLLMProvider())
    roadmap = coach.build_roadmap_llm(user_profile)
    assert len(roadmap.topics) == 3


def test_coach_raises_unavailable_on_provider_failure(user_profile, fake_llm_provider) -> None:
    def responder(messages, **_):
        raise OpenRouterError("bad key")

    coach = CoachAgent(provider=fake_llm_provider(responder=responder))
    with pytest.raises(LLMUnavailableError):
        coach.build_roadmap_llm(user_profile)


def test_tutor_generates_material_via_llm(roadmap, base_state, fake_llm_provider) -> None:
    def responder(messages, **_):
        return json.dumps({"material": "llm material", "task": "llm task"})

    tutor = TutorAgent(provider=fake_llm_provider(responder=responder))
    base_state["roadmap"] = roadmap
    result = tutor.run(base_state)
    assert result["material"] == "llm material"
    assert result["task"] == "llm task"


def test_tutor_uses_offline_stub_when_stub_provider(roadmap, base_state) -> None:
    tutor = TutorAgent(provider=StubLLMProvider())
    base_state["roadmap"] = roadmap
    result = tutor.run(base_state)
    topic = roadmap.topics[base_state["current_topic_index"]]
    assert result["material"] == f"Материал по теме «{topic.title}»: {topic.description}"
    assert result["task"] == f"Практическое задание: продемонстрируйте умение темы «{topic.title}»."


def test_tutor_raises_unavailable_on_provider_failure(
    roadmap, base_state, fake_llm_provider
) -> None:
    def responder(messages, **_):
        raise OpenRouterError("bad key")

    tutor = TutorAgent(provider=fake_llm_provider(responder=responder))
    base_state["roadmap"] = roadmap
    with pytest.raises(LLMUnavailableError):
        tutor.run(base_state)


def test_tutor_does_not_swallow_provider_errors(roadmap, base_state, fake_llm_provider) -> None:
    def responder(messages, **_):
        raise RuntimeError("provider failed")

    tutor = TutorAgent(provider=fake_llm_provider(responder=responder))
    base_state["roadmap"] = roadmap

    with pytest.raises(RuntimeError, match="provider failed"):
        tutor.run(base_state)


def test_examiner_llm_evaluator(roadmap, base_state, fake_llm_provider) -> None:
    def responder(messages, **_):
        return json.dumps({"score": 88, "feedback": "nice", "weak_areas": []})

    examiner = ExaminerAgent(provider=fake_llm_provider(responder=responder))
    base_state["roadmap"] = roadmap
    result = examiner.assess("some answer", base_state)
    assert result["assessment"].verdict == Verdict.PASS
    assert result["assessment"].score == 88.0


def test_examiner_llm_evaluator_retry_on_low_score(roadmap, base_state, fake_llm_provider) -> None:
    def responder(messages, **_):
        return json.dumps({"score": 40, "feedback": "weak", "weak_areas": ["x"]})

    examiner = ExaminerAgent(provider=fake_llm_provider(responder=responder))
    base_state["roadmap"] = roadmap
    result = examiner.assess("bad answer", base_state)
    assert result["assessment"].verdict == Verdict.RETRY
    assert result["assessment"].weak_areas == ["x"]


def test_examiner_uses_offline_evaluator_when_stub_provider(roadmap, base_state) -> None:
    examiner = ExaminerAgent(provider=StubLLMProvider())
    base_state["roadmap"] = roadmap
    result = examiner.assess("this is good work", base_state)
    assert result["assessment"].verdict == Verdict.PASS


def test_examiner_raises_unavailable_on_provider_failure(
    roadmap, base_state, fake_llm_provider
) -> None:
    def responder(messages, **_):
        raise OpenRouterError("bad key")

    examiner = ExaminerAgent(provider=fake_llm_provider(responder=responder))
    base_state["roadmap"] = roadmap
    with pytest.raises(LLMUnavailableError):
        examiner.assess("some answer", base_state)
