from __future__ import annotations

from langgraph.checkpoint.memory import MemorySaver
from langgraph.types import Command

from nereus.agents.examiner import ExaminerAgent
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
    assert "retries" in final["assessment"].feedback.lower()


def test_retry_loop_reinvokes_examiner(base_state) -> None:
    calls: list[str] = []

    def counting_evaluator(submission: str):
        calls.append(submission)
        return 30.0, "Needs work.", ["fundamentals"]

    examiner = ExaminerAgent(evaluator=counting_evaluator)
    graph = NereusGraph(examiner=examiner)
    graph.invoke({**_input(base_state["user_profile"]), "user_submission": "bad"})

    # 3 topics * (2 attempts + 1 forced) => examiner invoked more than once per topic
    assert len(calls) > 3


def test_interactive_human_in_the_loop(base_state) -> None:
    graph = NereusGraph(checkpointer=MemorySaver(), interactive=True)
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
