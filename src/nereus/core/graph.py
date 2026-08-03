from __future__ import annotations

from langgraph.graph import END, StateGraph
from langgraph.types import interrupt

from nereus.agents.base import BaseAgent
from nereus.agents.coach import CoachAgent
from nereus.agents.examiner import ExaminerAgent
from nereus.agents.tutor import TutorAgent
from nereus.core.router import ADVANCE_TUTOR, RETRY_TUTOR, route_after_exam
from nereus.core.state import Assessment, NereusState, Roadmap, Verdict

_DEFAULT_STATE: dict = {
    "user_profile": None,
    "roadmap": Roadmap(),
    "current_topic_index": 0,
    "material": "",
    "task": "",
    "user_submission": None,
    "assessment": None,
    "retry_count": 0,
    "max_retries": 2,
    "status": "coaching",
    "messages": [],
}


class NereusGraph:
    """Assembles the Nereus learning automaton as a LangGraph ``StateGraph``.

    Node flow::

        START -> coach -> tutor_new -> examiner -> router -> (tutor_retry | tutor_advance | END)

    When ``interactive`` is enabled the examiner node pauses on ``interrupt()``
    waiting for the human's answer (human-in-the-loop). Compiling with a
    checkpointer allows the CLI to resume the run with ``Command(resume=...)``.
    """

    def __init__(
        self,
        coach: BaseAgent | None = None,
        tutor: BaseAgent | None = None,
        examiner: BaseAgent | None = None,
        checkpointer=None,
        interactive: bool = False,
    ) -> None:
        self._coach_agent = coach or CoachAgent()
        self._tutor_agent = tutor or TutorAgent()
        self._examiner_agent = examiner or ExaminerAgent()
        self._interactive = interactive
        self._graph = self._build(checkpointer)

    def _advance_topic(self, state: NereusState) -> dict:
        return {
            "current_topic_index": state["current_topic_index"] + 1,
            "retry_count": 0,
        }

    def _tutor_new(self, state: NereusState) -> dict:
        return self._tutor_agent.run(state)

    def _tutor_retry(self, state: NereusState) -> dict:
        return {
            **self._tutor_agent.run(state),
            "retry_count": state["retry_count"] + 1,
        }

    def _tutor_advance(self, state: NereusState) -> dict:
        updates = self._advance_topic(state)
        advanced_state = {**state, **updates}
        return {**updates, **self._tutor_agent.run(advanced_state)}

    def _examiner(self, state: NereusState) -> dict:
        if state["retry_count"] >= state["max_retries"]:
            forced = Assessment(
                topic_id=state["roadmap"].topics[state["current_topic_index"]].id,
                score=70.0,
                verdict=Verdict.PASS,
                feedback="Maximum retries reached; advancing automatically.",
                weak_areas=[],
            )
            return {"assessment": forced}

        if self._interactive:
            submission = interrupt({"task": state["task"]})
            return self._examiner_agent.assess(submission, state)

        return self._examiner_agent.run(state)

    def _complete(self, state: NereusState) -> dict:
        return {"status": "completed"}

    def _build(self, checkpointer) -> StateGraph:
        builder = StateGraph(NereusState)

        builder.add_node("coach", self._coach_agent.run)
        builder.add_node("tutor_new", self._tutor_new)
        builder.add_node("tutor_retry", self._tutor_retry)
        builder.add_node("tutor_advance", self._tutor_advance)
        builder.add_node("examiner", self._examiner)
        builder.add_node("complete", self._complete)

        builder.set_entry_point("coach")
        builder.add_edge("coach", "tutor_new")
        builder.add_edge("tutor_new", "examiner")
        builder.add_edge("tutor_retry", "examiner")
        builder.add_edge("tutor_advance", "examiner")
        builder.add_edge("complete", END)

        builder.add_conditional_edges(
            "examiner",
            route_after_exam,
            {
                RETRY_TUTOR: "tutor_retry",
                ADVANCE_TUTOR: "tutor_advance",
                "end": "complete",
            },
        )

        if checkpointer is not None:
            return builder.compile(checkpointer=checkpointer)
        return builder.compile()

    @property
    def app(self) -> StateGraph:
        return self._graph

    def invoke(self, state: object, config: dict | None = None) -> dict:
        if isinstance(state, dict):
            state = {**_DEFAULT_STATE, **state}
        if config:
            return self._graph.invoke(state, config=config)
        return self._graph.invoke(state)