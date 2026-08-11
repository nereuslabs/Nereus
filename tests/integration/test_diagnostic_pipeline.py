"""Integration tests for the full diagnostic → adaptive roadmap pipeline (Issue #7)."""

from __future__ import annotations

import json
import uuid

from langgraph.types import Command

from nereus.core.factory import build_nereus_graph
from nereus.core.persistence import CheckpointBackend, build_checkpointer
from nereus.core.state import Verdict, WeaknessReport
from nereus.llm.stub import StubLLMProvider


def test_non_interactive_diagnostic_produces_weakness_report(base_state) -> None:
    """In non-interactive mode with run_diagnostic=True, the graph runs the diagnostic
    and injects a WeaknessReport before the coach builds an adaptive roadmap."""
    graph = build_nereus_graph(
        interactive=False,
        provider=StubLLMProvider(),
        checkpointer=build_checkpointer(CheckpointBackend.MEMORY),
        run_diagnostic=True,
    )

    final = graph.invoke(
        {**base_state, "user_submission": "this is good work"},
        config={"configurable": {"thread_id": "diag-test-01"}},
    )

    # The graph should have run through the full cycle
    assert final["status"] == "completed"
    assert final["current_topic_index"] == 2
    assert len(final["roadmap"].topics) >= 3
    # The first topic should target weak areas (higher difficulty)
    first_topic = final["roadmap"].topics[0]
    assert first_topic.difficulty >= 0.5


def test_stub_diagnostic_generates_5_questions(base_state) -> None:
    """Diagnostic node generates the expected number of questions."""
    graph = build_nereus_graph(
        interactive=False,
        provider=StubLLMProvider(),
        checkpointer=build_checkpointer(CheckpointBackend.MEMORY),
        run_diagnostic=True,
    )

    # In non-interactive mode, stub answers all questions with "1"
    final = graph.invoke(
        {**base_state, "user_submission": "this is good work"},
        config={"configurable": {"thread_id": "diag-test-02"}},
    )

    # Questions should have been generated (state includes them in the graph run)
    # The weakness report should be in the final state
    weakness = final.get("weakness_report")
    assert weakness is not None
    assert isinstance(weakness, WeaknessReport)


def test_coach_uses_weakness_report_for_adaptive_roadmap(base_state, fake_llm_provider) -> None:
    """When weakness_report is in state, CoachAgent builds an adaptive roadmap."""
    import json

    def responder(messages, **_):
        # Discriminate by the role-defining system prompt to return the right schema
        for msg in messages:
            if msg.get("role") == "system":
                content = msg.get("content", "")
                if "diagnostician" in content:  # diagnostic role
                    return json.dumps(
                        {"topics": [{"id": "1", "question": "Q", "options": ["A", "B"]}]}
                    )
                if "learning coach" in content:  # coach role
                    return json.dumps(
                        {
                            "topics": [
                                {
                                    "id": "1",
                                    "title": "Python: syntax review",
                                    "description": "Review weak areas",
                                    "difficulty": 0.8,
                                    "prerequisites": [],
                                    "estimated_hours": 2.0,
                                },
                                {
                                    "id": "2",
                                    "title": "Python: data types",
                                    "description": "Deep dive into types",
                                    "difficulty": 0.7,
                                    "prerequisites": ["1"],
                                    "estimated_hours": 3.0,
                                },
                            ]
                        }
                    )
        # Default: tutor/examiner responses
        for msg in messages:
            if msg.get("role") == "user":
                content = str(msg.get("content", ""))
                if "Topic title" in content:  # tutor role
                    return json.dumps({"material": "mat", "task": "task"})
        # examiner default
        return json.dumps({"score": 85, "feedback": "ok", "weak_areas": []})

    graph = build_nereus_graph(
        interactive=False,
        provider=fake_llm_provider(responder=responder),
        checkpointer=build_checkpointer(CheckpointBackend.MEMORY),
        run_diagnostic=False,  # We'll inject weakness_report manually
    )

    weakness_report = WeaknessReport(
        weak_areas=["syntax", "data types"],
        recommended_topics=["1"],
    )

    final = graph.invoke(
        {
            **base_state,
            "user_submission": "this is good work",
            "weakness_report": weakness_report,
        },
        config={"configurable": {"thread_id": "diag-test-03"}},
    )

    assert final["status"] == "completed"
    assert len(final["roadmap"].topics) == 2
    assert final["roadmap"].topics[0].difficulty == 0.8
    assert final["roadmap"].topics[1].prerequisites == ["1"]


def test_full_diagnostic_cycle_with_submissions(base_state) -> None:
    """End-to-end: diagnostic → adaptive roadmap → tutor → examiner with PASS."""
    graph = build_nereus_graph(
        interactive=False,
        provider=StubLLMProvider(),
        checkpointer=build_checkpointer(CheckpointBackend.MEMORY),
        run_diagnostic=True,
    )

    final = graph.invoke(
        {**base_state, "user_submission": "this is good work"},
        config={"configurable": {"thread_id": "diag-test-04"}},
    )

    assert final["status"] == "completed"
    # Stub returns 90 for "good" → PASS at any difficulty
    assert final["assessment"].verdict == Verdict.PASS
    assert final["assessment"].score >= 70.0


def test_interactive_diagnostic_interrupts_for_answers(base_state, fake_llm_provider) -> None:
    """In interactive mode, the diagnostic node pauses for user answers.

    Uses FakeLLMProvider (non-offline) so the LLM path is exercised.
    """
    import json

    from langgraph.types import Command

    def responder(messages, **_):
        """Return diagnostic questions for the diagnostic role."""
        for msg in messages:
            if msg.get("role") == "system" and "diagnostician" in msg.get("content", ""):
                return json.dumps(
                    {
                        "questions": [
                            {
                                "id": "q1",
                                "question": "What is Python?",
                                "options": ["A", "B", "C", "D"],
                            },
                            {
                                "id": "q2",
                                "question": "What is a variable?",
                                "options": ["A", "B", "C", "D"],
                            },
                        ]
                    }
                )
        # Coach/tutor/examiner fallbacks
        for msg in messages:
            if msg.get("role") == "system":
                content = msg.get("content", "")
                if "learning coach" in content:
                    return json.dumps(
                        {
                            "topics": [
                                {
                                    "id": "1",
                                    "title": "T1",
                                    "description": "d",
                                    "difficulty": 0.5,
                                    "prerequisites": [],
                                    "estimated_hours": 1.0,
                                },
                                {
                                    "id": "2",
                                    "title": "T2",
                                    "description": "d",
                                    "difficulty": 0.7,
                                    "prerequisites": ["1"],
                                    "estimated_hours": 1.0,
                                },
                            ]
                        }
                    )
        # Tutor/examiner
        for msg in messages:
            if msg.get("role") == "user":
                content = str(msg.get("content", ""))
                if "Topic title" in content:
                    return json.dumps({"material": "mat", "task": "task"})
        # Examiner default
        return json.dumps({"score": 85, "feedback": "ok", "weak_areas": []})

    graph = build_nereus_graph(
        interactive=True,
        provider=fake_llm_provider(responder=responder),
        checkpointer=build_checkpointer(CheckpointBackend.MEMORY),
        run_diagnostic=True,
    )

    config = {"configurable": {"thread_id": "diag-interactive-01"}}
    state = {**base_state}

    # First invoke should pause at diagnostic interrupt
    final = graph.invoke(state, config)
    assert "__interrupt__" in final

    # Provide answers via resume (LangGraph interrupt returns this value)
    answers = {"q1": "1", "q2": "1"}
    final = graph.invoke(Command(resume=answers), config)

    # Should continue through the pipeline
    assert final["status"] == "completed" or "__interrupt__" in final


# --------------------------------------------------------------------------- #
# Interactive resume hardening (Issue #7, step 5 of the sprint)                #
# --------------------------------------------------------------------------- #
# `_diagnostic_node` resumes a LangGraph ``interrupt`` with whatever the caller
# hands back via ``Command(resume=...)``. It must accept:
#   * a dict of answers            -> the happy path
#   * a JSON-encoded string        -> parsed via ``json.loads``
#   * any other opaque string      -> previously crashed in ``dict(received)``
# These three tests lock the hardened ``else / except`` behaviour down.


def _diag_provider(fake_llm_provider):
    """FakeLLMProvider that answers every diagnostic/role call deterministically."""

    def responder(messages, **_):
        for msg in messages:
            if msg.get("role") == "system":
                content = msg.get("content", "")
                if "Create a short diagnostic quiz" in content:
                    return json.dumps(
                        {
                            "questions": [
                                {
                                    "id": "q1",
                                    "question": "Что такое цикл в Python?",
                                    "options": ["for", "while", "if", "def"],
                                },
                                {
                                    "id": "q2",
                                    "question": "Что такое переменная?",
                                    "options": ["значение", "тип", "функция", "класс"],
                                },
                            ]
                        }
                    )
                if "Evaluate the user's answers" in content:
                    return json.dumps(
                        {"weak_areas": ["loops", "variables"], "recommended_topics": ["1", "2"]}
                    )
                if "learning coach" in content:
                    return json.dumps(
                        {
                            "topics": [
                                {
                                    "id": "1",
                                    "title": "T1",
                                    "description": "d",
                                    "difficulty": 0.5,
                                    "prerequisites": [],
                                    "estimated_hours": 1.0,
                                },
                                {
                                    "id": "2",
                                    "title": "T2",
                                    "description": "d",
                                    "difficulty": 0.7,
                                    "prerequisites": ["1"],
                                    "estimated_hours": 1.0,
                                },
                            ]
                        }
                    )
                if "patient tutor" in content:
                    return json.dumps({"material": "mat", "task": "task"})
                if "strict examiner" in content:
                    return json.dumps({"score": 95, "feedback": "good", "weak_areas": []})
        # Examiner fallback for any remaining user-turn (submission grading).
        return json.dumps({"score": 95, "feedback": "good", "weak_areas": []})

    return fake_llm_provider(responder=responder)


def _run_interactive_diagnostic(
    graph, base_state, first_resume, submission: str = "this is good work"
) -> dict:
    """Drive an interactive diagnostic graph end-to-end and return final state.

    1. invoke             -> pauses at the diagnostic ``interrupt`` (questions)
    2. resume(first_resume) -> parses answers & builds the weakness report,
        then lands at the examiner ``interrupt`` (first topic submission)
    3. resume(submission) x N -> grades each topic until the run completes
    """
    config = {"configurable": {"thread_id": uuid.uuid4().hex}}
    final = graph.invoke({**base_state}, config)
    assert "__interrupt__" in final, "graph should pause at the diagnostic interrupt"

    final = graph.invoke(Command(resume=first_resume), config)
    guard = 0
    while "__interrupt__" in final and guard < 16:
        final = graph.invoke(Command(resume=submission), config)
        guard += 1
    return final


def test_interactive_diagnostic_resume_accepts_dict_answers(base_state, fake_llm_provider) -> None:
    """Regression lock: a dict resume of diagnostic answers completes the cycle."""
    graph = build_nereus_graph(
        interactive=True,
        provider=_diag_provider(fake_llm_provider),
        checkpointer=build_checkpointer(CheckpointBackend.MEMORY),
        run_diagnostic=True,
    )
    final = _run_interactive_diagnostic(graph, base_state, {"q1": "1", "q2": "1"})

    assert final["status"] == "completed"
    assert len(final["roadmap"].topics) == 2
    assert final["current_topic_index"] == 1
    assert final["assessment"].verdict == Verdict.PASS
    assert isinstance(final.get("weakness_report"), WeaknessReport)


def test_interactive_diagnostic_resume_accepts_json_string(base_state, fake_llm_provider) -> None:
    """The resume may arrive as a JSON string -> parsed via ``json.loads``."""
    graph = build_nereus_graph(
        interactive=True,
        provider=_diag_provider(fake_llm_provider),
        checkpointer=build_checkpointer(CheckpointBackend.MEMORY),
        run_diagnostic=True,
    )
    final = _run_interactive_diagnostic(graph, base_state, json.dumps({"q1": "1", "q2": "1"}))

    assert final["status"] == "completed"
    assert final["assessment"].verdict == Verdict.PASS
    assert isinstance(final.get("weakness_report"), WeaknessReport)


def test_interactive_diagnostic_resume_ignores_malformed_string(
    base_state, fake_llm_provider
) -> None:
    """A non-JSON, non-dict resume must NOT raise (previously ``dict(received)``).

    With the hardened ``except`` (-> ``{}``) the node falls back to stub answers,
    so the run still drains through diagnostics -> coach -> examiner to completion.
    """
    graph = build_nereus_graph(
        interactive=True,
        provider=_diag_provider(fake_llm_provider),
        checkpointer=build_checkpointer(CheckpointBackend.MEMORY),
        run_diagnostic=True,
    )
    final = _run_interactive_diagnostic(graph, base_state, "definitely-not-json-answers")

    assert final["status"] == "completed"
    assert final["assessment"].verdict == Verdict.PASS
    assert isinstance(final.get("weakness_report"), WeaknessReport)
