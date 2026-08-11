from __future__ import annotations

import logging
from pathlib import Path
from typing import Mapping, Optional

from langgraph.graph import END, StateGraph
from langgraph.types import interrupt

from nereus.agents.base import BaseAgent
from nereus.agents.coach import CoachAgent
from nereus.agents.diagnostic import DiagnosticAgent
from nereus.agents.examiner import ExaminerAgent
from nereus.agents.tutor import TutorAgent
from nereus.core.router import ADVANCE_TUTOR, RETRY_TUTOR, route_after_exam
from nereus.core.session import LearningSession
from nereus.core.state import Assessment, NereusState, Roadmap, Verdict
from nereus.llm.base import LLMProvider
from nereus.llm.inference import StructuredInferenceClient
from nereus.llm.retriever import Retriever, StubRetriever

logger = logging.getLogger("nereus.graph")

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
    "session": None,
    "session_brief": "",
    "retrieved_chunks": None,
    "messages": [],
}


def _default_retriever() -> Retriever:
    """Build the default retriever from settings (stub by default -> offline)."""
    from nereus.config.settings import settings

    if settings.embedding_provider == "stub":
        return StubRetriever()
    from nereus.db.chroma import ChromaStore
    from nereus.llm.embed import build_embedder
    from nereus.llm.retriever import ChromaRetriever

    store = ChromaStore(host=settings.chromadb_host, port=settings.chromadb_port)
    # Eagerly cache the lazy chromadb HTTP client so retrieval can use it; the
    # call is lazy (no network at construction). If the server is unreachable,
    # ``_retrieve_chunks`` degrades to no-RAG instead of starting the run without
    # a connected store.
    store.connect()
    return ChromaRetriever(store=store, embedder=build_embedder())


class NereusGraph:
    """Assembles the Nereus learning automaton as a LangGraph ``StateGraph``.

        Node flow::

        START -> coach -> tutor_new -> examiner -> router -> (
            tutor_retry -> examiner | tutor_advance -> examiner | END
        )

        Each tutor node enriches state with `retrieved_chunks` (RAG) before the
        examiner runs.

    When ``interactive`` is enabled the examiner node pauses on ``interrupt()``
    waiting for the human's answer (human-in-the-loop). Compiling with a
    checkpointer allows the CLI to resume the run with ``Command(resume=...)``.
    """

    def __init__(
        self,
        coach: BaseAgent | None = None,
        tutor: BaseAgent | None = None,
        examiner: BaseAgent | None = None,
        diagnostic: BaseAgent | None = None,
        provider: LLMProvider | None = None,
        inference: StructuredInferenceClient | None = None,
        retriever: Retriever | None = None,
        checkpointer=None,
        interactive: bool = False,
        session_path: Path | str | None = None,
        session_id: str | None = None,
        user_id: str | None = None,
        run_diagnostic: bool = False,
    ) -> None:
        inference = inference or (StructuredInferenceClient(provider) if provider else None)
        self._coach_agent = coach or CoachAgent(inference=inference, provider=provider)
        self._tutor_agent = tutor or TutorAgent(inference=inference, provider=provider)
        self._examiner_agent = examiner or ExaminerAgent(inference=inference, provider=provider)
        self._diagnostic_agent = diagnostic or DiagnosticAgent(
            inference=inference, provider=provider
        )
        self._inference = inference
        self._retriever: Retriever | None = (
            retriever if retriever is not None else _default_retriever()
        )
        self._interactive = interactive
        self._session_path = Path(session_path) if session_path else None
        self._session_id = session_id
        self._user_id = user_id
        self._run_diagnostic = run_diagnostic
        self._user_store = None
        self._graph = self._build(checkpointer)

    @property
    def user_store(self):
        """Lazy ``UserStore`` (resolved from settings.user_storage)."""
        if self._user_store is None:
            from nereus.config.settings import settings
            from nereus.core.user_store import UserStore

            self._user_store = UserStore(db_path=settings.user_db_path)
        return self._user_store

    # ------------------------------------------------------------------ #
    # Session persistence (runtime wiring for #6/#22)                    #
    def _session_for(self, config: Optional[dict]) -> LearningSession | None:
        """Load the saved LearningSession on a new graph instance (if any).

        Returns the session (or None). The caller is responsible for merging
        key fields (profile, roadmap, index) into state before invoking.
        """
        if self._session_path is None or not self._session_path.exists():
            return None
        try:
            return LearningSession.load(self._session_path)
        except Exception as exc:  # noqa: BLE001
            logger.warning("could not load session from %s: %s", self._session_path, exc)
            return None

    @staticmethod
    def _merge_session(state: dict, session: LearningSession) -> None:
        """Populate domain fields from a recovered session so the graph can
        resume without re-running the coach (which requires a user_profile).

        ``_DEFAULT_STATE`` seeds ``roadmap`` with an empty ``Roadmap()``; when a
        saved session carries real topics we overwrite it so the recovered
        state (not the empty default) drives the graph.
        """
        if session.user_profile is not None and state.get("user_profile") is None:
            state["user_profile"] = session.user_profile
        incoming_roadmap = state.get("roadmap")
        if session.roadmap.topics and (incoming_roadmap is None or not incoming_roadmap.topics):
            state["roadmap"] = session.roadmap
        if state.get("current_topic_index") is None:
            state["current_topic_index"] = session.current_topic_index

    def _dump_session(self, state: Mapping[str, object]) -> None:
        """Persist the live session after a boundary node (examiner/complete).

        Two backends:
        - ``session_path`` (legacy #22 wiring): saves the in-graph ``LearningSession``
          as JSON via ``session.dump``.
        - ``session_id`` (P1 multi-user): saves a :class:`UserSession` snapshot to
          ``SESSION_ROOT/{user_id}/{session_id}.json`` so progress survives
          restarts and isolates per-user.
        """
        from nereus.core.session import UserSession, session_path_for

        if self._session_id is not None:
            try:
                us = UserSession.from_state(state, session_id=self._session_id)
                us.user_id = self._user_id
                path = session_path_for(self._user_id, self._session_id)
                us.dump(path)
                logger.debug("UserSession dumped -> %s", path)
            except Exception as exc:  # noqa: BLE001
                logger.warning("could not persist UserSession: %s", exc)

        if self._session_path is None:
            return
        session = state.get("session")
        if not isinstance(session, LearningSession):
            return
        try:
            self._session_path.parent.mkdir(parents=True, exist_ok=True)
            session.dump(self._session_path)
            logger.debug("session dumped -> %s", self._session_path)
        except Exception as exc:  # noqa: BLE001
            logger.warning("could not persist session to %s: %s", self._session_path, exc)

    # ------------------------------------------------------------------ #

    # ------------------------------------------------------------------ #

    def _advance_topic(self, state: NereusState) -> dict:
        return {
            "current_topic_index": state["current_topic_index"] + 1,
            "retry_count": 0,
        }

    def _retrieve_chunks(self, state: NereusState) -> list:
        """Fetch RAG chunks for the current topic (pre-exam enrichment).

        Returns an empty list when no retriever is configured or the topic is
        not yet known, leaving the rest of the graph unaffected (offline-first).
        """
        retriever = self._retriever
        if retriever is None:
            return []
        try:
            topic = state["roadmap"].topics[state["current_topic_index"]]
        except (KeyError, IndexError, AttributeError, TypeError):
            return []
        from nereus.config.settings import settings

        query = state.get("session_brief") or state.get("task") or ""
        try:
            return retriever.retrieve(query=query, topic=topic, top_k=settings.retriever_top_k)
        except Exception as exc:  # noqa: BLE001  retrieval must never sink a run
            logger.warning(
                "RAG retrieval failed for topic %s; continuing without context: %s",
                getattr(topic, "id", None),
                exc,
            )
            return []

    def _tutor_new(self, state: NereusState) -> dict:
        return {
            **self._tutor_agent.run(state),
            "retrieved_chunks": self._retrieve_chunks(state),
        }

    def _tutor_retry(self, state: NereusState) -> dict:
        return {
            **self._tutor_agent.run(state),
            "retry_count": state["retry_count"] + 1,
            "retrieved_chunks": self._retrieve_chunks(state),
        }

    def _tutor_advance(self, state: NereusState) -> dict:
        updates = self._advance_topic(state)
        advanced_state = {**state, **updates}
        return {
            **updates,
            **self._tutor_agent.run(advanced_state),
            "retrieved_chunks": self._retrieve_chunks(advanced_state),
        }

    def _examiner(self, state: NereusState) -> dict:
        if state["retry_count"] >= state["max_retries"]:
            forced = Assessment(
                topic_id=state["roadmap"].topics[state["current_topic_index"]].id,
                score=70.0,
                verdict=Verdict.PASS,
                feedback="Достигнуто максимальное число попыток; продвигаемся дальше.",
                weak_areas=[],
            )
            return {"assessment": forced}

        if self._interactive:
            submission = interrupt({"task": state["task"]})
            return self._examiner_agent.assess(submission, state)

        return self._examiner_agent.run(state)

    def _complete(self, state: NereusState) -> dict:
        return {"status": "completed"}

    def _diagnostic_node(self, state: NereusState) -> dict:
        """Run diagnostic quiz and store questions in state.

        In interactive mode, interrupts for user answers (first pass).
        On resume with answers, evaluates them and produces a weakness report.
        In non-interactive mode, uses stub answers.
        """
        result = self._diagnostic_agent.run(state)
        questions = result.get("diagnostic_questions", [])

        # Check if we already have answers (resume after interrupt)
        answers = state.get("user_diagnostic_answers")

        if self._interactive and questions and not answers:
            # First pass: interrupt to collect user answers
            interrupt_payload = {"questions": [q.model_dump() for q in questions]}
            # Use LangGraph interrupt to pause and collect answers on resume
            received = interrupt(interrupt_payload)
            # On resume, received contains the user's answers
            if isinstance(received, dict):
                answers = received
            elif isinstance(received, str):
                try:
                    import json

                    answers = json.loads(received)
                except (json.JSONDecodeError, ValueError):
                    answers = dict(received)  # noqa: E731
            else:
                answers = dict(received) if received else {}
            result["user_diagnostic_answers"] = answers

        # Either non-interactive (default), or interactive with answers received
        if questions:
            if not answers:
                # Non-interactive: use stub answers (all option "1")
                answers = {q.id: "1" for q in questions}
                result["user_diagnostic_answers"] = answers

            # Evaluate answers through the diagnostic agent
            profile = state.get("user_profile")
            if profile is not None:
                weakness_report = self._diagnostic_agent.evaluate_answers(
                    profile, questions, answers
                )
                result["weakness_report"] = weakness_report

        return {
            **result,
            "user_diagnostic_answers": answers,
        }

    def _build(self, checkpointer) -> StateGraph:
        builder = StateGraph(NereusState)

        builder.add_node("diagnostic", self._diagnostic_node)
        builder.add_node("coach", self._coach_agent.run)
        builder.add_node("tutor_new", self._tutor_new)
        builder.add_node("tutor_retry", self._tutor_retry)
        builder.add_node("tutor_advance", self._tutor_advance)
        builder.add_node("examiner", self._examiner)
        builder.add_node("complete", self._complete)

        if self._run_diagnostic:
            builder.set_entry_point("diagnostic")
            builder.add_edge("diagnostic", "coach")
        else:
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

    def trim_context(self, state: Mapping[str, object]) -> dict:
        """Bound the checkpointer-bound message history to the token limit.

        Uses LLM summarisation when an inference client is wired, otherwise
        hard-truncates. Purely on the returned ``messages``; does not mutate
        domain fields."""
        from nereus.config.settings import settings
        from nereus.core.context import summarize_history

        messages = list(state.get("messages", []))
        if not messages:
            return {}
        trimmed = summarize_history(messages, self._inference, settings.context_max_tokens)
        return {"messages": trimmed}

    def _load_user_session(self, config: dict | None) -> dict:
        """Load a persisted :class:`UserSession` if ``session_id`` is in config.

        Overrides defaults from ``_DEFAULT_STATE`` so the graph resumes a prior
        user's roadmap / progress instead of restarting coaching.
        """
        from nereus.core.session import UserSession, session_path_for

        if self._session_id is None:
            return {}
        # Allow per-call override via config (e.g. resume a different thread).
        cfg_session_id = None
        if config:
            cfg = config.get("configurable", {}) or {}
            cfg_session_id = cfg.get("session_id")
        target = cfg_session_id or self._session_id
        path = session_path_for(self._user_id, target)
        if not path.exists():
            logger.debug("no UserSession on disk at %s", path)
            return {}
        try:
            us = UserSession.load(path)
            logger.info("resumed UserSession %s -> %s", target, path)
            return us.to_state_dict()
        except Exception as exc:  # noqa: BLE001
            logger.warning("failed to load UserSession %s: %s", target, exc)
            return {}

    def invoke(self, state: object, config: dict | None = None) -> dict:
        if isinstance(state, dict):
            user_state = self._load_user_session(config)
            state = {**_DEFAULT_STATE, **user_state, **state}
            loaded = self._session_for(config)
            if loaded is not None and state.get("session") is None:
                state["session"] = loaded
                self._merge_session(state, loaded)
                logger.info("resumed session from %s", self._session_path)
        if config:
            final = self._graph.invoke(state, config=config)
        else:
            final = self._graph.invoke(state)
        # Bound the persisted message history (keeps checkpointer payloads sane).
        if isinstance(final, dict) and final.get("messages"):
            final.update(self.trim_context(final))
        self._dump_session(final)
        return final

    async def astream(self, state: object, config: dict | None = None, stream_mode: str = "values"):
        if isinstance(state, dict):
            user_state = self._load_user_session(config)
            state = {**_DEFAULT_STATE, **user_state, **state}
            loaded = self._session_for(config)
            if loaded is not None and state.get("session") is None:
                state["session"] = loaded
                self._merge_session(state, loaded)
                logger.info("resumed session from %s", self._session_path)
        args = (state, config) if config else (state,)
        async for chunk in self._graph.astream(*args, stream_mode=stream_mode):
            if isinstance(chunk, dict):
                self._dump_session(chunk)
            yield chunk
