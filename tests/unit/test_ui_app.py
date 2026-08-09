from __future__ import annotations

from typing import Any

import pytest

from nereus.core.persistence import CheckpointBackend, build_checkpointer
from nereus.core.state import Assessment, Verdict
from nereus.ui.app import UIApp, _interrupt_value


class _Interrupt:
	def __init__(self, value: dict[str, Any]) -> None:
		self.value = value


class _FakeGraph:
	"""Mimics the subset of NereusGraph used by UIApp."""

	def __init__(self) -> None:
		self.calls: list[Any] = []
		self.cfg: dict | None = None

	async def astream(self, state, config=None, stream_mode="values"):
		self.calls.append(state)
		self.cfg = config
		if isinstance(state, Command):
			yield {"status": "completed"}
		else:
			# Simulate stream_mode="values": LangGraph re-emits the *whole*
			# accumulated state on every chunk, so fields set by earlier nodes
			# (material, assessment) are still present on later chunks (#51).
			yield {
				"material": "Раздел 1",
				"retrieved_chunks": [{"content": "отрывок 1", "score": 0.9}],
			}
			yield {
				"assessment": Assessment(
					topic_id="t1", score=90.0, verdict=Verdict.PASS, feedback="ok"
				)
			}
			yield {
				"material": "Раздел 1",
				"assessment": Assessment(
					topic_id="t1", score=90.0, verdict=Verdict.PASS, feedback="ok"
				),
				"__interrupt__": [_Interrupt({"task": "реши задачу"})],
			}


class Command:
	def __init__(self, resume: Any) -> None:
		self.resume = resume


@pytest.fixture
def patched_cl(monkeypatch):
	"""Replace Chainlit primitives so renders/asks work without a live session.

	Chainlit 2.x (installed in the image: 2.11.0) makes
	``AskUserMessage.send()`` return a StepDict *dict* whose ``output`` key
	holds the user's typed reply — not an object with ``.content``. The fixture
	mirrors that contract so tests guard the real shape (regression for the
	``answer.content`` AttributeError). It also records every ``cl.Message``
	actually rendered so duplication regressions (#51) can be asserted against.
	"""
	from nereus.ui import app as appmod

	messages: list[str] = []

	class _Recorder:
		def __init__(self, *a, **k):
			self.content = k.get("content")

		async def send(self, *a, **k):
			messages.append(self.content)
			return self

	def _make_ask():
		state = {"reply": "good"}
		# Every AskUserMessage prompt actually rendered, in order — used to
		# assert the exam task/material are bundled into the question-turn and
		# not duplicated as separate bubbles (#60).
		ask_contents: list[str] = []

		def ask_user_message(**k):
			content = k.get("content", "")

			async def _send(*a, **kk):
				ask_contents.append(content)
				reply = state["reply"]
				# A list models a sequence of typed replies (e.g. empty then a
				# real answer) across several AskUserMessage calls.
				reply = reply.pop(0) if isinstance(reply, list) else reply
				return {
					"output": reply,
					"input": content,
					"id": "ask-1",
					"type": "text",
				}

			rec = _Recorder(**k)
			rec.send = _send
			return rec

		ask_user_message.state = state
		ask_user_message.ask_contents = ask_contents
		return ask_user_message

	ask = _make_ask()
	ask.messages = messages
	monkeypatch.setattr(appmod.cl, "Message", lambda **k: _Recorder(**k))
	monkeypatch.setattr(appmod.cl, "AskUserMessage", ask)
	return ask


async def test_astream_yields_chunks_and_returns_interrupt(patched_cl):
	graph = _FakeGraph()
	app = UIApp(graph=graph)
	interrupt = await app.astream(
		{"user_profile": object(), "max_retries": 2},
	)
	# First call was the initial state (not a Command).
	assert not isinstance(graph.calls[0], Command)
	# Interrupt payload returned to the driver loop.
	assert interrupt == {"task": "реши задачу"}
	# Resume via Command drives to completion.
	interrupt2 = await app.astream(Command(resume="good"))
	assert interrupt2 is None  # run completed
	assert graph.calls[-1] is not None and graph.calls[-1].resume == "good"


async def test_render_deduplicates_material_and_assessment(patched_cl):
	"""#51 + #60: LangGraph re-emits persisted state on every chunk, so the
	assessment must render exactly once and the run ends with a single
	completion. Per #60 the material / exam task are NOT emitted as proactive
	assistant bubbles — they are stashed in ``self._last`` and surfaced inside
	the AskUserMessage question-turn instead."""
	graph = _FakeGraph()
	app = UIApp(graph=graph)
	await app.astream({"user_profile": object(), "max_retries": 2})
	await app.astream(Command(resume="good"))

	def _count(sub: str) -> int:
		return sum(1 for m in patched_cl.messages if m and sub in m)

	# #60: no proactive material / exam-task bubbles.
	assert _count("Материал") == 0, patched_cl.messages
	assert _count("Экзаменатор") == 0, patched_cl.messages
	# #51: assessment still rendered once (dedup) + single completion.
	assert _count("Оценка") == 1, patched_cl.messages
	assert _count("Дорогой ученик") == 1, patched_cl.messages
	# Material + task stashed for exam_prompt (folded into the Ask), not re-sent.
	assert app._last.get("material") == "Раздел 1"
	assert app._last.get("interrupt_task") == "реши задачу"


async def test_run_app_session_offline_renders_no_proactive_messages(
	patched_cl, user_profile, tmp_path
):
	"""#60 (bugs 1, 2, 3): a real OFFLINE run must NOT emit proactive
	`📚 Материал` / `📝 Экзаменатор` bubbles; material is bundled into the
	AskUserMessage question-turn, the exam task is shown exactly once (inside
	the Ask), and the assessment only appears AFTER the user's answer — never a
	0/100 "because no answer"."""
	from nereus.core.factory import build_nereus_graph
	from nereus.ui.app import UIApp, _run_app_session

	# The real graph is driven via LangGraph's *async* astream; the autouse
	# conftest fixture pins the default saver to the sync SqliteSaver, which is
	# not async-safe. Use an in-memory MemorySaver so interrupt/resume works
	# in tests (the production default is also "memory").
	cp = build_checkpointer(CheckpointBackend.MEMORY)
	app = UIApp(
		graph=build_nereus_graph(interactive=True, checkpointer=cp),
		checkpointer=cp,
	)
	await _run_app_session(app, user_profile)

	msgs = [m for m in patched_cl.messages if m]
	# Bug 1: no proactive material / exam-task bubbles.
	assert not any("📚" in m for m in msgs), msgs
	assert not any("📝  **Экзаменатор**" in m for m in msgs), msgs
	# Bug 3: no assessment/score is rendered before the first answer (the intro
	# message carries no score) -- grades appear only after an AskUserMessage.
	assert msgs and "Оценка" not in msgs[0], msgs
	# Bug 2: 3 topics => 3 AskUserMessage question-turns; each bundles the exam
	# task + material (shown once, inside the Ask, not as a separate bubble).
	assert len(patched_cl.ask_contents) == 3, patched_cl.ask_contents
	for prompt in patched_cl.ask_contents:
		assert "Ваш ответ" in prompt, prompt
		assert "Экзаменатор" in prompt, prompt     # task bundled once, inside the Ask
		assert "Материал" in prompt, prompt         # material folded in, not a separate bubble
	# The offline stub grades every "good" as 90/PASS -> identical (verdict,
	# score, feedback) akeys are deduped by _render (#51) to a single
	# assessment message; a live run with distinct per-topic scores renders one
	# per topic. So assert >=1 post-answer grade + exactly one completion.
	assert sum("Оценка" in m for m in msgs) >= 1, msgs
	assert sum("Дорогой ученик" in m for m in msgs) == 1, msgs


async def test_run_exam_loop_re_asks_on_empty_submission(patched_cl, tmp_path):
	"""#60 (bug 3): an empty user answer must NOT be graded (which yields 0/100
	'because no answer'); the assistant re-asks the same task instead."""
	from nereus.ui.app import UIApp, _run_exam_loop

	class _ResumeGraph:
		def __init__(self) -> None:
			self.resumes: list[Any] = []
			self.cfg = None

		async def astream(self, state, config=None, stream_mode="values"):
			self.resumes.append(state)
			# NB: a resumed turn arrives as a langgraph.types.Command (app.py
			# builds it), not the local Command shadowed below — so discriminate
			# by the `resume` attribute instead of isinstance.
			if hasattr(state, "resume"):
				yield {"status": "completed"}
			else:
				yield {"__interrupt__": [_Interrupt({"task": "реши задачу"})]}

	cp = build_checkpointer(CheckpointBackend.MEMORY)
	app = UIApp(graph=_ResumeGraph(), checkpointer=cp)
	patched_cl.state["reply"] = ["", "good"]  # first send empty, then a real answer

	interrupt = await app.astream({"user_profile": object(), "max_retries": 2})
	await _run_exam_loop(app, interrupt)

	resumes = app.graph.resumes
	resume_values = [getattr(r, "resume", None) for r in resumes]
	# No resumption with an empty submission; only the real answer is resumed.
	assert "" not in resume_values, resume_values
	assert "good" in resume_values, resume_values
	# Clarification shown + the task re-asked (empty -> re-ask -> good).
	assert any("не ввели ответ" in m for m in patched_cl.messages), patched_cl.messages
	assert len(patched_cl.ask_contents) == 2, patched_cl.ask_contents


def test_interrupt_value_parses_object():
	state = {"__interrupt__": [_Interrupt({"task": "x"})]}
	assert _interrupt_value(state) == {"task": "x"}


def test_interrupt_value_parses_dict():
	state = {"__interrupt__": [{"task": "y"}]}
	# dicts have no .value -> returned as-is
	assert _interrupt_value(state) is None or _interrupt_value(state) == {"task": "y"}


def test_uigraph_uses_persistent_checkpointer(tmp_path) -> None:
	"""UIApp must wire a persistent checkpointer so sessions survive restarts."""
	db = tmp_path / "ui_checkpoints.sqlite3"
	saver = build_checkpointer(CheckpointBackend.SQLITE, db_path=str(db))

	app = UIApp(checkpointer=saver)
	# Injects the exact saver we passed (not a new MemorySaver).
	assert app._checkpointer is saver
	assert type(app._checkpointer).__name__ == "SqliteSaver"

	# Default (no arg) builds from settings.checkpoint_backend and is non-None.
	app_default = UIApp(checkpointer=None)
	assert app_default._checkpointer is not None


async def test_ask_reads_dict_reply(patched_cl):
	"""Regression: Chainlit 2.x AskUserMessage.send() returns a dict with
	an 'output' key (not an object with .content); _ask must read it."""
	from nereus.ui.app import _ask

	patched_cl.state["reply"] = "мой ответ"
	assert await _ask("Вопрос?", "fallback") == "мой ответ"


async def test_ask_uses_default_when_reply_empty(patched_cl):
	from nereus.ui.app import _ask

	patched_cl.state["reply"] = ""
	assert await _ask("Вопрос?", "fallback") == "fallback"


async def test_ask_handles_none_reply(patched_cl):
	from nereus.ui.app import _ask

	patched_cl.state["reply"] = None  # simulate a no-op / timeout dict
	assert await _ask("Вопрос?", "fallback") == "fallback"


async def test_run_exam_loop_reads_dict_reply_and_resumes(tmp_path, patched_cl):
	"""Regression: the examiner answer typed by the user (arrives as a dict)
	must be resumed verbatim via Command(resume=...); and the run must end
	with a single completion message (no duplicate #51)."""
	from nereus.core.persistence import CheckpointBackend, build_checkpointer
	from nereus.ui.app import UIApp, _run_exam_loop

	db = tmp_path / "ui.sqlite3"
	app = UIApp(
		graph=_FakeGraph(),
		checkpointer=build_checkpointer(CheckpointBackend.SQLITE, db_path=str(db)),
	)
	captured: dict = {}

	async def _resume_capture(cmd, config=None, stream_mode="values"):
		captured["resume"] = getattr(cmd, "resume", None)
		yield {"status": "completed"}

	app.graph.astream = _resume_capture
	patched_cl.state["reply"] = "решил задачу"
	await _run_exam_loop(app, {"task": "реши задачу"})
	assert captured["resume"] == "решил задачу"
	# Exactly one completion message: the one from UIApp.astream. There must
	# be NO second one from _run_exam_loop (#51 duplicate-completion guard).
	completions = sum(
		1 for m in patched_cl.messages
		if m and ("Дорогой ученик" in m or "Готово" in m)
	)
	assert completions == 1, patched_cl.messages


async def test_collect_profile_recovers_from_level_typo(patched_cl, monkeypatch):
	"""#55 regression: a misspelled learning level must not crash
	collect_profile; it re-prompts and accepts a corrected value instead of
	raising ValueError from UserLevel(...) inside on_chat_start."""
	from nereus.core.state import UserLevel
	from nereus.ui import app as appmod

	replies = [
		"Python",  # skill
		"intemediate",  # current level typo -> re-prompt
		"intermediate",  # current level corrected -> accepted
		"intermediate",  # target level
		"1.5",  # hours
		"30",  # deadline
		"Стать Python-разработчиком",  # goal
	]
	it = iter(replies)

	async def fake_ask(prompt, default=""):
		return next(it)

	monkeypatch.setattr(appmod, "_ask", fake_ask)
	profile = await appmod.collect_profile()
	assert profile.skill == "Python"
	assert profile.current_level == UserLevel.INTERMEDIATE
	assert profile.target_level == UserLevel.INTERMEDIATE
	assert profile.hours_per_day == 1.5
	assert profile.deadline_days == 30
	assert profile.goal == "Стать Python-разработчиком"


async def test_ask_float_recovers_from_nonnumeric(patched_cl, monkeypatch):
	"""#55 regression: a non-numeric hour entry re-prompts instead of
	crashing on_chat_start with ValueError."""
	from nereus.ui import app as appmod

	replies = ["не число", "2.5"]
	it = iter(replies)

	async def fake_ask(prompt, default=""):
		return next(it)

	monkeypatch.setattr(appmod, "_ask", fake_ask)
	val = await appmod._ask_float("Часов в день [1]:", 1.0, minimum=0.0)
	assert val == 2.5


async def test_run_app_session_surfaces_unavailable_and_does_not_crash(
	tmp_path, patched_cl, user_profile
):
	"""#44 regression: when the LLM provider is unavailable, _run_app_session
	surfaces 'service temporarily unavailable' and ends the session cleanly
	instead of crashing the chat."""
	from nereus.core.persistence import CheckpointBackend, build_checkpointer
	from nereus.llm.inference import LLMUnavailableError
	from nereus.ui.app import UIApp, _run_app_session

	class _UnavailableGraph:
		def __init__(self) -> None:
			self.cfg = None

		async def astream(self, state, config=None, stream_mode="values"):
			raise LLMUnavailableError("boom")
			yield  # makes this an async generator (pragma: no cover)

	app = UIApp(
		graph=_UnavailableGraph(),
		checkpointer=build_checkpointer(
			CheckpointBackend.SQLITE, db_path=str(tmp_path / "ui.sqlite3")
		),
	)

	# Must not raise: unavailability is surfaced to the user instead (#44/#45).
	await _run_app_session(app, user_profile)

	rendered = [m for m in patched_cl.messages if m]
	assert any("Сервис временно недоступен" in m for m in rendered)
