from __future__ import annotations

import json

from nereus.agents.coach import CoachAgent
from nereus.agents.examiner import ExaminerAgent
from nereus.agents.tutor import TutorAgent
from nereus.core.state import Verdict
from nereus.llm.stub import StubLLMProvider


def test_coach_builds_roadmap_via_llm(user_profile) -> None:
    def responder(messages, **_):
        return json.dumps(
            {
                "topics": [
                    {"id": "1", "title": "A", "description": "d1"},
                    {"id": "2", "title": "B", "description": "d2"},
                ]
            }
        )

    coach = CoachAgent(provider=StubLLMProvider(responder=responder))
    roadmap = coach.build_roadmap_llm(user_profile)
    assert len(roadmap.topics) == 2
    assert roadmap.topics[0].title == "A"


def test_coach_falls_back_when_llm_returns_bad_json(user_profile) -> None:
    coach = CoachAgent(provider=StubLLMProvider())
    roadmap = coach.build_roadmap_llm(user_profile)
    assert len(roadmap.topics) == 3


def test_tutor_generates_material_via_llm(roadmap, base_state) -> None:
    def responder(messages, **_):
        return json.dumps({"material": "llm material", "task": "llm task"})

    tutor = TutorAgent(provider=StubLLMProvider(responder=responder))
    base_state["roadmap"] = roadmap
    result = tutor.run(base_state)
    assert result["material"] == "llm material"
    assert result["task"] == "llm task"


def test_examiner_llm_evaluator(roadmap, base_state) -> None:
    def responder(messages, **_):
        return json.dumps({"score": 88, "feedback": "nice", "weak_areas": []})

    examiner = ExaminerAgent(provider=StubLLMProvider(responder=responder))
    base_state["roadmap"] = roadmap
    result = examiner.assess("some answer", base_state)
    assert result["assessment"].verdict == Verdict.PASS
    assert result["assessment"].score == 88.0


def test_examiner_llm_evaluator_retry_on_low_score(roadmap, base_state) -> None:
    def responder(messages, **_):
        return json.dumps({"score": 40, "feedback": "weak", "weak_areas": ["x"]})

    examiner = ExaminerAgent(provider=StubLLMProvider(responder=responder))
    base_state["roadmap"] = roadmap
    result = examiner.assess("bad answer", base_state)
    assert result["assessment"].verdict == Verdict.RETRY
    assert result["assessment"].weak_areas == ["x"]


def test_examiner_llm_evaluator_falls_back_on_bad_json(roadmap, base_state) -> None:
    examiner = ExaminerAgent(provider=StubLLMProvider())
    base_state["roadmap"] = roadmap
    result = examiner.assess("this is good work", base_state)
    assert result["assessment"].verdict == Verdict.PASS