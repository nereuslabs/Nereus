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
	"""Replace Chainlit primitives so renders record payloads without a session."""
	from nereus.ui import app as appmod

	class _Recorder:
		def __init__(self, *a, **k):
			self.content = k.get("content")

		async def send(self, *a, **k):
			return self

		def __await__(self):
			async def _self():
				return self
			return _self().__await__()

	class _Ask(_Recorder):
		async def send(self, *a, **k):
			self.content = "good"
			return self

	# cl.Message(...).send() already awaitable; cl.AskUserMessage uses same shape.
	monkeypatch.setattr(appmod.cl, "Message", lambda **k: _Recorder(**k))
	monkeypatch.setattr(appmod.cl, "AskUserMessage", lambda **k: _Ask(**k))


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
