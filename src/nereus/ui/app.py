"""Chainlit Web UI entrypoint for the Nereus learning automaton (Step 5).

Runs the LangGraph pipeline in an async driver so each node's output can be
rendered incrementally: coach → tutor (material + retrieved_chunks) →
examiner (interrupt → cl.AskUserMessage → Command(resume=...)) → assessment.
"""

from __future__ import annotations

import logging
import uuid
from pathlib import Path
from typing import Any, Mapping

import chainlit as cl
from langgraph.types import Command

from nereus.config.settings import settings
from nereus.core.factory import build_nereus_graph
from nereus.core.graph import NereusGraph
from nereus.core.persistence import CheckpointBackend, build_checkpointer
from nereus.core.session import UserSession, session_path_for
from nereus.core.state import UserLevel, UserProfile
from nereus.core.user_store import build_user_store
from nereus.llm.inference import LLMUnavailableError

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


async def _ask_level(prompt: str, default: UserLevel) -> UserLevel:
    """Ask for a :class:`UserLevel`, re-prompting on a typo (#55).

    A single misspelled level used to raise ``ValueError`` from
    ``UserLevel(...)`` and crash ``on_chat_start``. We now re-prompt up to 3
    times (showing the valid values) and fall back to the default so a typo
    never breaks the chat.
    """
    valid = {lvl.value: lvl for lvl in UserLevel}
    default_value = default.value
    for _ in range(3):
        raw = (await _ask(prompt, default_value)).strip().lower()
        if raw in valid:
            return valid[raw]
        display = raw or "(пусто)"
        await cl.Message(
            content=f"⚠️  Уровень «{display}» не распознан. "
            f"Допустимые значения: {', '.join(sorted(valid))}. "
            f"Повторите ввод (или оставьте пустым — {default_value})."
        ).send()
    return default


async def _ask_float(prompt: str, default: float, minimum: float | None = None) -> float:
    """Ask for a float, re-prompting on a non-numeric or out-of-range value."""
    default_value = str(default)
    for _ in range(3):
        raw = (await _ask(prompt, default_value)).strip()
        try:
            value = float(raw)
        except ValueError:
            await cl.Message(
                content=f"⚠️  «{raw}» — введите число, например, {default_value}."
            ).send()
            continue
        if minimum is not None and value < minimum:
            await cl.Message(content=f"⚠️  Значение должно быть не меньше {minimum}.").send()
            continue
        return value
    return default


async def _ask_int(prompt: str, default: int, minimum: int | None = None) -> int:
    """Ask for an int, re-prompting on a non-numeric or out-of-range value."""
    default_value = str(default)
    for _ in range(3):
        raw = (await _ask(prompt, default_value)).strip()
        try:
            value = int(raw)
        except ValueError:
            await cl.Message(
                content=f"⚠️  «{raw}» — введите целое число, например, {default_value}."
            ).send()
            continue
        if minimum is not None and value < minimum:
            await cl.Message(content=f"⚠️  Значение должно быть не меньше {minimum}.").send()
            continue
        return value
    return default


async def collect_profile() -> UserProfile:
    """Interactively build a :class:`UserProfile` via chat prompts.

    All free-form inputs are validated and re-prompted (up to 3 attempts) so a
    typo or a stray non-number can never crash ``on_chat_start`` (#55).
    """
    await cl.Message(content="🤖  Я ваш персональный AI‑тютор. Давайте настроим профиль.").send()

    skill = await _ask("Какой навык выучить? [Python]:", "Python")
    current = await _ask_level(
        "Текущий уровень (beginner/intermediate/advanced) [beginner]:",
        UserLevel.BEGINNER,
    )
    target = await _ask_level(
        "Целевой уровень (beginner/intermediate/advanced) [intermediate]:",
        UserLevel.INTERMEDIATE,
    )
    hours = await _ask_float("Часов в день выделяете на обучение [1]:", 1.0, minimum=0.0)
    deadline = await _ask_int("За сколько дней хотите завершить [30]:", 30, minimum=1)
    goal = await _ask(f"Цель обучения [{skill}]:", f"Oсвоить {skill}")

    return UserProfile(
        skill=skill,
        current_level=current,
        target_level=target,
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
        user_id: str | None = None,
        session_id: str | None = None,
        checkpointer=None,
        run_diagnostic: bool = False,
    ) -> None:
        if checkpointer is not None:
            self._checkpointer = checkpointer
        else:
            cp_backend = settings.checkpoint_backend
            self._checkpointer = build_checkpointer(
                CheckpointBackend(cp_backend) if isinstance(cp_backend, str) else cp_backend
            )
            logger.info("UIApp checkpointer: %s", type(self._checkpointer).__name__)
        self.user_id = user_id
        self.session_id = session_id or str(uuid.uuid4())
        self.thread_id = thread_id or (
            f"{user_id}:{self.session_id}" if user_id else str(uuid.uuid4())
        )
        self.graph: NereusGraph = (
            graph
            if graph is not None
            else build_nereus_graph(
                interactive=True,
                checkpointer=self._checkpointer,
                user_id=user_id,
                session_id=self.session_id,
                run_diagnostic=run_diagnostic,
            )
        )
        # config carries user_id/session_id so the graph can resume persisted
        # UserSession snapshots (#8/#57) and write per-user session files.
        configurable: dict[str, Any] = {
            "thread_id": self.thread_id,
            "session_id": self.session_id,
        }
        if user_id is not None:
            configurable["user_id"] = user_id
        self.config: dict[str, Any] = {"configurable": configurable}
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
        if material:
            # #60: material is NOT sent as a standalone "📚" bubble here. It is
            # stashed and folded into the next AskUserMessage question-turn
            # (see :meth:`exam_prompt`) so the assistant only "speaks" as a
            # reply to the user — never proactively mid-stream. Retrieved chunks
            # may be Pydantic ``RetrievedChunk`` objects (no ``.get``), so they
            # are copied verbatim and accessed via :func:`_attr` later.
            self._last["material"] = material
            self._last["retrieved_chunks"] = state.get("retrieved_chunks") or []

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
            # #60: the exam task is surfaced ONCE — inside the AskUserMessage
            # prompt (exam_prompt), NOT as a separate "📝" bubble, which
            # duplicated the task up front (bug #2).
            self._last["interrupt_task"] = task

    def exam_prompt(self, task: str) -> str:
        """Build the AskUserMessage prompt for an exam turn.

        Bundles the current topic material and any retrieved RAG snippets
        with the exam task into a single assistant question-turn -- instead
        of emitting them as standalone proactive messages before the user
        replies (#60, bugs 1 & 2). Retrieved chunks may be Pydantic
        RetrievedChunk objects, so `content` is read via `_attr` (works for
        dicts and objects), which also avoids the `.get` AttributeError on
        Pydantic models.
        """
        book = chr(0x1F4DA)  # material icon (📚 BOOKS)
        memo = chr(0x1F4DD)  # examiner icon
        nl = chr(10)
        parts: list[str] = []
        material = self._last.get("material", "")
        if material:
            parts.append(f"{book}  **Материал:** {material}")
        chunks = self._last.get("retrieved_chunks") or []
        if chunks:
            refs = nl.join(f"- {_attr(c, 'content', '')[:200]}" for c in chunks)
            parts.append(f"**Полезные отрывки:**{nl}{refs}")
        parts.append(f"{memo}  **Экзаменатор:** {task}")
        sep = nl + nl
        return sep.join(parts) + sep + "Ваш ответ (или 'good', чтобы сдать):"


@cl.on_chat_start
async def on_chat_start() -> None:
    profile, user_id = await select_or_create_user()
    # Resume per-user history if available; otherwise start fresh.
    resume_session = await resume_last_session(user_id)
    session_id = resume_session or str(uuid.uuid4())

    cl.user_session.set("user_id", user_id)
    cl.user_session.set("session_id", session_id)
    # thread_id carries user:session so the LangGraph checkpointer resumes the
    # right history across page reloads (#8/#57).
    cl.user_session.set("thread_id", f"{user_id}:{session_id}")

    app = UIApp(
        thread_id=cl.user_session.get("thread_id"),
        user_id=user_id,
        session_id=session_id,
        run_diagnostic=settings.run_diagnostic,
    )
    app.profile = profile
    cl.user_session.set("app", app)

    if resume_session:
        await cl.Message(
            content=f"👋  Возвращаемся! Продолжаю вашу сессию «{session_id[:8]}».\n"
            f"🗺️  Строю дорожную карту для «{profile.goal}» "
            f"(уровни: {profile.current_level.value} → {profile.target_level.value})…"
        ).send()
    else:
        await cl.Message(
            content=f"🗺️  Строю дорожную карту для «{profile.goal}» "
            f"(уровни: {profile.current_level.value} → {profile.target_level.value})…"
        ).send()
    await _run_app_session(app, profile)


async def select_or_create_user() -> tuple[UserProfile, str]:
    """Let the user pick an existing profile or register a new one.

    Backed by :class:`UserStore` (sqlite/redis/memory per ``USER_STORAGE``).
    The very first chat with no registered users auto-creates one so the UI
    stays usable without any setup (#8/#57)."""
    try:
        store = build_user_store()
    except Exception as exc:  # noqa: BLE001  — keep UI alive if DB is misconfigured
        logger.warning("UserStore unavailable (%s); running single-session", exc)
        profile = await collect_profile()
        return profile, str(uuid.uuid4())

    users = store.list_users()
    if not users:
        await cl.Message(content="👤  Первый запуск — создадим ваш профиль.").send()
        profile = await collect_profile()
        uid = store.create_user(profile)
        logger.info("created user %s", uid[:8])
        return profile, uid

    names = [f"{uid[:8]} — {p.skill} ({p.current_level.value})" for uid, p in users]
    await cl.Message(content=f"👥  Зарегистрировано {len(users)} пользователей:").send()
    choice = await cl.AskUserMessage(
        content=(
            "Выберите пользователя:\n"
            + "\n".join(f"{i + 1}) {n}" for i, n in enumerate(names))
            + "\n0) 🌱  Новый пользователь"
        ),
        timeout=600,
    ).send()
    raw = _answer_text(choice).strip()
    try:
        idx = int(raw)
    except ValueError:
        idx = 0
    if idx == 0 or not (0 < idx <= len(users)):
        profile = await collect_profile()
        uid = store.create_user(profile)
        logger.info("created user %s", uid[:8])
        return profile, uid
    uid, profile = users[idx - 1]
    await cl.Message(content=f"👋  Продолжим с «{profile.skill}».").send()
    return profile, uid


async def resume_last_session(user_id: str) -> str | None:
    """Offer to resume the most recent session for ``user_id``.

    Lists the last 5 ``UserSession`` snapshots under
    ``{SESSION_ROOT}/{user_id}/`` and lets the user pick one (or start fresh).
    Returns the chosen ``session_id`` or ``None``."""
    try:
        root = Path(settings.session_root) / user_id if user_id else None
        if root is None or not root.exists():
            return None
        files = sorted(root.glob("*.json"), key=lambda p: p.stat().st_mtime, reverse=True)[:5]
    except Exception as exc:  # noqa: BLE001
        logger.warning("session history scan failed (%s); starting fresh", exc)
        return None
    if not files:
        return None

    options = []
    labels = []
    for f in files:
        sid = f.stem
        path = session_path_for(user_id, sid)
        try:
            sess = UserSession.load(path)
            n = len(sess.roadmap.topics) if sess.roadmap else 0
            title = f.stem[:8]
            labels.append(f"{title} — {n} тем")
        except Exception:  # noqa: BLE001
            labels.append(sid[:8])
        options.append(sid)

    await cl.Message(content="📂  Ваши последние сессии:").send()
    choice = await cl.AskUserMessage(
        content=(
            "Выберите сессию для продолжения:\n"
            + "\n".join(f"{i + 1}) {label}" for i, label in enumerate(labels))
            + "\n0) 🆕  Новое обучение"
        ),
        timeout=600,
    ).send()
    raw = _answer_text(choice).strip()
    try:
        idx = int(raw)
    except ValueError:
        idx = 0
    if 0 < idx <= len(options):
        return options[idx - 1]
    return None


async def _run_app_session(app: UIApp, profile: UserProfile) -> None:
    """Drive a Nereus learning session end-to-end.

    Wraps the coach → tutor → examiner pipeline. If the LLM provider cannot
    serve a request after retries (:class:`LLMUnavailableError`) we surface a
    clear "service temporarily unavailable" message and end the session cleanly
    instead of crashing the chat (#44/#45).
    """
    await cl.Message(
        content=f"🗺️  Строю дорожную карту для «{profile.goal}» "
        f"(уровни: {profile.current_level.value} → {profile.target_level.value})…"
    ).send()
    try:
        interrupt = await app.astream({"user_profile": profile, "max_retries": DEFAULT_MAX_RETRIES})
        await _run_exam_loop(app, interrupt)
    except LLMUnavailableError as exc:
        logger.warning("LLM unavailable during session: %s", exc)
        await cl.Message(
            content=(
                "⚠️  Сервис временно недоступен: LLM недоступен "
                "(проверьте ключ/баланс OpenRouter). "
                "Попробуйте позже или начните новый чат."
            )
        ).send()


async def _run_exam_loop(app: UIApp, interrupt: dict | None) -> None:
    """Drive the human-in-the-loop examiner until the roadmap completes.

    Each assistant turn here IS the question to the user: material + retrieved
    chunks + the exam task are bundled into the :class:`cl.AskUserMessage`
    prompt via :meth:`UIApp.exam_prompt` (instead of being emitted as
    proactive "📚/📝" bubbles up front — #60, bugs 1 & 2). An empty/blank answer
    is re-asked (not graded) so a missing answer never surfaces as a 0/100
    score (#60, bug 3)."""
    while interrupt is not None:
        task = interrupt.get("task", "")
        prompt = app.exam_prompt(task)
        answer = await cl.AskUserMessage(content=prompt, timeout=900).send()
        submission = _answer_text(answer).strip()
        if not submission:
            await cl.Message(content="⚠️  Вы не ввели ответ. Попробуйте ответить на задачу.").send()
            continue
        interrupt = await app.astream(Command(resume=submission))

    # Completion is announced once by UIApp.astream (on the final completed
    # chunk); nothing to say here — avoids the duplicate "you passed / start a
    # new chat" pair that previously appeared twice (#51).
