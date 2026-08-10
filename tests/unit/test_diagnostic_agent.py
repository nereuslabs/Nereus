"""Unit tests for DiagnosticAgent (Issue #7)."""

from __future__ import annotations

import json

import pytest

from nereus.agents.diagnostic import DiagnosticAgent
from nereus.core.state import DiagnosticQuestion, UserLevel, UserProfile, WeaknessReport
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


def test_stub_generates_default_questions() -> None:
    """StubLLMProvider (offline) generates predictable diagnostic questions."""
    agent = DiagnosticAgent(provider=StubLLMProvider())
    questions = agent.generate_questions(_profile())

    assert len(questions) >= 3  # default is 5
    assert all(isinstance(q, DiagnosticQuestion) for q in questions)
    assert all(q.question for q in questions)  # non-empty question text
    assert all(len(q.options) >= 2 for q in questions)  # at least 2 options
    assert all(q.id for q in questions)


def test_stub_evaluates_answers_produces_weakness_report() -> None:
    """Stub evaluation produces a WeaknessReport from answers."""
    agent = DiagnosticAgent(provider=StubLLMProvider())
    questions = agent.generate_questions(_profile())
    answers = {q.id: "1" for q in questions}

    report = agent.evaluate_answers(_profile(), questions, answers)
    assert isinstance(report, WeaknessReport)
    assert isinstance(report.weak_areas, list)
    assert isinstance(report.recommended_topics, list)


def test_stub_weakness_identifies_knowledge_gaps() -> None:
    """Stub evaluation should identify weak areas for bad answers."""
    agent = DiagnosticAgent(provider=StubLLMProvider())
    questions = agent.generate_questions(_profile())

    # For q4: "4" = "bool" → not the correct "string" type → identifies data type confusion
    # For q1: "2" = "вид спорта" → wrong definition → identifies basics
    # For q3: "2" = "Правильный код" → wrong error identification → debugging
    # For q5: "2" = "Функция" → wrong concept → variables
    answers = {}
    for q in questions:
        if q.id == "q4":
            answers[q.id] = "4"  # "bool" → data types weak area
        elif q.id == "q1":
            answers[q.id] = "2"  # "вид спорта" → basics weak area
        elif q.id == "q3":
            answers[q.id] = "2"  # "Правильный код" → debugging weak area
        elif q.id == "q5":
            answers[q.id] = "2"  # "Функция" → variables weak area
        else:
            answers[q.id] = "1"  # correct answer

    report = agent.evaluate_answers(_profile(), questions, answers)
    assert "python data types" in report.weak_areas
    assert "python basics" in report.weak_areas
    assert "python debugging" in report.weak_areas
    assert "python variables and memory" in report.weak_areas


def test_stub_weakness_identifies_variable_knowledge() -> None:
    """Stub evaluation detects weak areas from "function" answer."""
    agent = DiagnosticAgent(provider=StubLLMProvider())
    questions = agent.generate_questions(_profile())

    answers = {q.id: "2" for q in questions}
    report = agent.evaluate_answers(_profile(), questions, answers)
    # At least one weak area should be identified
    assert len(report.weak_areas) >= 1


def test_llm_generates_questions_via_inference(user_profile, fake_llm_provider) -> None:
    """FakeLLMProvider drives the real LLM path for question generation."""

    def responder(messages, **_):
        return json.dumps(
            {
                "questions": [
                    {"id": "q1", "question": "What is Python?", "options": ["A", "B", "C", "D"]},
                    {
                        "id": "q2",
                        "question": "What is a variable?",
                        "options": ["A", "B", "C", "D"],
                    },
                ]
            }
        )

    agent = DiagnosticAgent(provider=fake_llm_provider(responder=responder))
    questions = agent.generate_questions(user_profile)

    assert len(questions) == 2
    assert questions[0].id == "q1"
    assert questions[0].question == "What is Python?"
    assert questions[0].options == ["A", "B", "C", "D"]


def test_llm_evaluates_answers_via_inference(user_profile, fake_llm_provider) -> None:
    """FakeLLMProvider drives the real LLM path for weakness evaluation."""

    def responder(messages, **_):
        return json.dumps(
            {
                "weak_areas": ["syntax", "data types"],
                "recommended_topics": ["1", "2"],
            }
        )

    agent = DiagnosticAgent(provider=fake_llm_provider(responder=responder))
    questions = [
        DiagnosticQuestion(id="q1", question="Q1", options=["A", "B"]),
        DiagnosticQuestion(id="q2", question="Q2", options=["A", "B"]),
    ]
    report = agent.evaluate_answers(user_profile, questions, {"q1": "A", "q2": "B"})

    assert isinstance(report, WeaknessReport)
    assert report.weak_areas == ["syntax", "data types"]
    assert report.recommended_topics == ["1", "2"]


def test_llm_raises_unavailable_on_provider_failure(user_profile, fake_llm_provider) -> None:
    """Provider errors during diagnostic should raise LLMUnavailableError (#44/#45)."""
    from nereus.llm.inference import LLMUnavailableError
    from nereus.llm.openrouter import OpenRouterError

    def responder(messages, **_):
        raise OpenRouterError("bad key")

    agent = DiagnosticAgent(provider=fake_llm_provider(responder=responder))
    with pytest.raises(LLMUnavailableError):
        agent.generate_questions(user_profile)


def test_diagnostic_agent_requires_profile() -> None:
    """DiagnosticAgent.run() should raise if user_profile is missing."""
    agent = DiagnosticAgent(provider=StubLLMProvider())
    with pytest.raises(ValueError, match="user_profile"):
        agent.run({"user_profile": None})


def test_run_produces_questions_in_state() -> None:
    """DiagnosticAgent.run() stores questions in state."""
    agent = DiagnosticAgent(provider=StubLLMProvider())
    state = {"user_profile": _profile()}
    result = agent.run(state)

    assert "diagnostic_questions" in result
    assert len(result["diagnostic_questions"]) > 0
    assert result["status"] == "coaching"


def test_question_count_setting_respected() -> None:
    """DiagnosticAgent respects question_count setting."""
    agent = DiagnosticAgent(provider=StubLLMProvider(), question_count=3)
    questions = agent.generate_questions(_profile())
    assert len(questions) == 3
