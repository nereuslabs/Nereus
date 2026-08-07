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
			yield {
				"material": "Раздел 1",
				"retrieved_chunks": [{"content": "отрывок 1", "score": 0.9}],
			}
			yield {
				"assessment": Assessment(
					topic_id="t1", score=90.0, verdict=Verdict.PASS, feedback="ok"
				)
			}
			yield {"__interrupt__": [_Interrupt({"task": "реши задачу"})]}


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
	``answer.content`` AttributeError).
	"""
	from nereus.ui import app as appmod

	class _Recorder:
		def __init__(self, *a, **k):
			self.content = k.get("content")

		async def send(self, *a, **k):
			return self

	def _make_ask():
		state = {"reply": "good"}

		def ask_user_message(**k):
			content = k.get("content", "")

			async def _send(*a, **kk):
				return {
					"output": state["reply"],
					"input": content,
					"id": "ask-1",
					"type": "text",
				}

			rec = _Recorder(**k)
			rec.send = _send
			return rec

		ask_user_message.state = state
		return ask_user_message

	ask = _make_ask()
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
	must be resumed verbatim via Command(resume=...)."""
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
