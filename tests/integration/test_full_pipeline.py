from __future__ import annotations

from langgraph.types import Command

from nereus.agents.examiner import ExaminerAgent
from nereus.core.factory import build_nereus_graph
from nereus.core.graph import NereusGraph
from nereus.core.state import Verdict


def _input(profile: dict) -> dict:
    return {"user_profile": profile}


def test_full_pass_completes_roadmap(base_state) -> None:
    graph = NereusGraph()
    final = graph.invoke(
        {**_input(base_state["user_profile"]), "user_submission": "this is good work"}
    )

    assert len(final["roadmap"].topics) == 3
    assert final["current_topic_index"] == 2
    assert final["status"] == "completed"
    assert final["assessment"].verdict == Verdict.PASS


def test_failing_submission_terminates_via_max_retries(base_state) -> None:
    graph = NereusGraph()
    final = graph.invoke(
        {**_input(base_state["user_profile"]), "user_submission": "garbage answer"}
    )

    assert final["current_topic_index"] == 2
    assert final["status"] == "completed"
    assert final["assessment"].verdict == Verdict.PASS
    assert "попыток" in final["assessment"].feedback.lower()


def test_retry_loop_reinvokes_examiner(base_state) -> None:
    calls: list[str] = []

    def counting_evaluator(submission, context=None):
        calls.append(submission)
        return 30.0, "Needs work.", ["fundamentals"]

    examiner = ExaminerAgent(evaluator=counting_evaluator)
    graph = NereusGraph(examiner=examiner)
    graph.invoke({**_input(base_state["user_profile"]), "user_submission": "bad"})

    # 3 topics * (2 attempts + 1 forced) => examiner invoked more than once per topic
    assert len(calls) > 3


def test_full_pipeline_with_llm_provider(base_state, fake_llm_provider) -> None:
    import json

    def responder(messages, **_):
        # Discriminate by the role-defining user message, not by system keywords
        # (the SESSION CONTEXT brief may legitimately contain "roadmap"/"topics").
        user_content = ""
        for msg in messages:
            if msg.get("role") == "user":
                user_content = str(msg.get("content", ""))
                break
        if "build a roadmap" in user_content.lower():
            return json.dumps(
                {
                    "topics": [
                        {"id": "1", "title": "T1", "description": "d"},
                        {"id": "2", "title": "T2", "description": "d"},
                    ]
                }
            )
        if "topic title" in user_content.lower():
            return json.dumps({"material": "mat", "task": "task"})
        return json.dumps({"score": 95, "feedback": "ok", "weak_areas": []})

    graph = NereusGraph(provider=fake_llm_provider(responder=responder))
    final = graph.invoke({**_input(base_state["user_profile"]), "user_submission": "answer"})

    assert len(final["roadmap"].topics) == 2
    assert final["status"] == "completed"
    assert final["assessment"].score == 95.0
    assert final["material"] == "mat"


def test_interactive_human_in_the_loop(base_state) -> None:
    graph = build_nereus_graph(interactive=True)
    config = {"configurable": {"thread_id": "test-thread"}}

    final = graph.invoke(_input(base_state["user_profile"]), config)
    assert "__interrupt__" in final
    assert final["retry_count"] == 0

    final = graph.invoke(Command(resume="i have no idea"), config)
    assert "__interrupt__" in final
    assert final["retry_count"] == 1

    final = graph.invoke(Command(resume="this is good"), config)
    assert "__interrupt__" in final
    assert final["current_topic_index"] == 1
    assert final["retry_count"] == 0
