"""Chainlit Web UI entrypoint for the Nereus learning automaton (Step 5).

Runs the LangGraph pipeline in an async driver so each node's output can be
rendered incrementally: coach → tutor (material + retrieved_chunks) →
examiner (interrupt → cl.AskUserMessage → Command(resume=...)) → assessment.
"""

from __future__ import annotations

import logging
import uuid
from typing import Any, Mapping

import chainlit as cl
from langgraph.types import Command

from nereus.config.settings import settings
from nereus.core.factory import build_nereus_graph
from nereus.core.graph import NereusGraph
from nereus.core.persistence import CheckpointBackend, build_checkpointer
from nereus.core.state import UserLevel, UserProfile

logger = logging.getLogger("nereus.ui")

DEFAULT_MAX_RETRIES = 2
_INTERRUPT_KEY = "__interrupt__"


def _attr(obj: Any, key: str, default: Any = None) -> Any:
    """Attribute access that also works for plain dict payloads."""
    if isinstance(obj, dict):
        return obj.get(key, default)
    return getattr(obj, key, default)


def _answer_text(answer: Any) -> str:
    """Extract the typed reply from a Chainlit ``AskUserMessage`` response.

    Chainlit 2.x returns a ``StepDict`` (plain dict) holding the reply in the
    ``output`` key, while older runtimes returned an object exposing ``.content``.
    Both shapes are handled so the UI keeps working across versions.
    """
    if answer is None:
        return ""
    return str(_attr(answer, "output") or _attr(answer, "content", "") or "")


def _interrupt_value(state: Mapping[str, Any]) -> dict | None:
    """Extract the interrupt payload from a streamed state chunk."""
    interrupts = state.get(_INTERRUPT_KEY) or []
    for interrupt in interrupts:
        try:
            return interrupt.value  # langgraph.Interrupt
        except AttributeError:
            return interrupt  # already a dict
    return None


async def _ask(prompt: str, default: str = "") -> str:
    """Ask the user for a single line of text during profile collection."""
    answer = await cl.AskUserMessage(
        content=prompt,
        timeout=600,
    ).send()
    value = _answer_text(answer).strip()
    return value or default


async def collect_profile() -> UserProfile:
    """Interactively build a :class:`UserProfile` via chat prompts."""
    await cl.Message(content="🤖  Я ваш персональный AI‑тютор. Давайте настроим профиль.").send()

    skill = await _ask("Какой навык выучить? [Python]:", "Python")
    current = await _ask(
        "Текущий уровень (beginner/intermediate/advanced) [beginner]:",
        "beginner",
    )
    target = await _ask(
        "Целевой уровень (beginner/intermediate/advanced) [intermediate]:",
        "intermediate",
    )
    hours = float(await _ask("Часов в день выделяете на обучение [1]:", "1"))
    deadline = int(await _ask("За сколько дней хотите завершить [30]:", "30"))
    goal = await _ask(f"Цель обучения [{skill}]:", f"Oсвоить {skill}")

    return UserProfile(
        skill=skill,
        current_level=UserLevel(current),
        target_level=UserLevel(target),
        hours_per_day=hours,
        deadline_days=deadline,
        goal=goal,
    )


class UIApp:
    """Async driver that bridges Chainlit <-> the Nereus LangGraph."""

    def __init__(
        self,
        graph: "NereusGraph | None" = None,
        thread_id: str | None = None,
        checkpointer=None,
    ) -> None:
        if checkpointer is not None:
            self._checkpointer = checkpointer
        else:
            cp_backend = settings.checkpoint_backend
            self._checkpointer = build_checkpointer(
                CheckpointBackend(cp_backend) if isinstance(cp_backend, str) else cp_backend
            )
            logger.info("UIApp checkpointer: %s", type(self._checkpointer).__name__)
        self.graph: NereusGraph = (
            graph
            if graph is not None
            else build_nereus_graph(interactive=True, checkpointer=self._checkpointer)
        )
        self.thread_id = thread_id or str(uuid.uuid4())
        self.config: dict[str, Any] = {"configurable": {"thread_id": self.thread_id}}
        self.profile: UserProfile | None = None
        # Last rendered value per logical field; used to deduplicate chat
        # messages under stream_mode="values" (LangGraph re-emits the whole
        # accumulated state after every node, which would otherwise re-send
        # material / assessment / exam task on each chunk — #51).
        self._last: dict[str, Any] = {}

    async def astream(self, msg: Mapping[str, Any] | Command) -> dict | None:
        """Stream the graph forward (or resume) and render each chunk.

        Returns the interrupt payload if the run paused for human input, or
        ``None`` if the run completed.
        """
        interrupt: dict | None = None
        async for chunk in self.graph.astream(msg, self.config, stream_mode="values"):
            await self._render(chunk)
            value = _interrupt_value(chunk)
            if value is not None:
                interrupt = value

        completed = chunk.get("status") == "completed" if chunk else False
        if completed:
            await cl.Message(
                content="🎓  Дорогой ученик! Вы прошли всю дорожную карту. "
                "👏 Поздравляем! Чтобы пройти курс ещё раз — начните новый чат."
            ).send()
        return interrupt

    async def _render(self, state: Mapping[str, Any]) -> None:
        """Render a single state chunk, sending only fields that changed.

        LangGraph ``stream_mode="values"`` re-emits the *whole* accumulated
        state after each node, so ``material``/``assessment``/``task`` would
        otherwise be re-sent on every chunk (#51). Each field is rendered only
        when its value differs from the last rendered value for this session.
        """
        material = state.get("material")
        if material and material != self._last.get("material"):
            self._last["material"] = material
            chunks = state.get("retrieved_chunks") or []
            if chunks:
                refs = "\n".join(f"- {c.get('content', '')[:200]}" for c in chunks)
                await cl.Message(
                    content=f"📚  **Материал:**\n{material}\n\n**Полезные отрывки:**\n{refs}"
                ).send()
            else:
                await cl.Message(content=f"📚  **Материал:**\n{material}").send()

        assessment = state.get("assessment")
        if assessment is not None:
            akey = (
                str(_attr(assessment, "verdict", "")),
                float(_attr(assessment, "score", 0.0) or 0.0),
                str(_attr(assessment, "feedback", "") or ""),
            )
            if akey != self._last.get("assessment"):
                self._last["assessment"] = akey
                verdict = _attr(assessment, "verdict")
                verdict = verdict.value if hasattr(verdict, "value") else str(verdict)
                score = float(_attr(assessment, "score", 0.0) or 0.0)
                feedback = _attr(assessment, "feedback", "") or ""
                verdict_ru = "✅  Зачёт" if verdict == "pass" else "🔁  Нужно повторить"
                await cl.Message(
                    content=f"{verdict_ru}  **Оценка:** {score:.0f}/100\n{feedback or '—'}"
                ).send()

        interrupt = _interrupt_value(state)
        if interrupt is not None:
            task = interrupt.get("task", "")
            if task != self._last.get("interrupt_task"):
                self._last["interrupt_task"] = task
                await cl.Message(content=f"📝  **Экзаменатор:** {task}").send()


@cl.on_chat_start
async def on_chat_start() -> None:
    profile = await collect_profile()
    thread_id = cl.user_session.get("thread_id") or str(uuid.uuid4())
    cl.user_session.set("thread_id", thread_id)
    app = UIApp(thread_id=thread_id)
    app.profile = profile
    cl.user_session.set("app", app)

    await cl.Message(
        content=f"🗺️  Строю дорожную карту для «{profile.goal}» "
        f"(уровни: {profile.current_level.value} → {profile.target_level.value})…"
    ).send()

    interrupt = await app.astream({"user_profile": profile, "max_retries": DEFAULT_MAX_RETRIES})
    await _run_exam_loop(app, interrupt)


async def _run_exam_loop(app: UIApp, interrupt: dict | None) -> None:
    """Drive the human-in-the-loop examiner until the roadmap completes."""
    while interrupt is not None:
        task = interrupt.get("task", "")
        answer = await cl.AskUserMessage(
            content=f"[Экзаменатор] {task}\nВаш ответ (или 'good', чтобы сдать):",
            timeout=900,
        ).send()
        submission = _answer_text(answer).strip()
        interrupt = await app.astream(Command(resume=submission))

    # Completion is announced once by UIApp.astream (on the final completed
    # chunk); nothing to say here — avoids the duplicate "you passed / start a
    # new chat" pair that previously appeared twice (#51).
