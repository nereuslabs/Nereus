from __future__ import annotations

import json

from nereus.core.context import (
    approx_token_count,
    summarize_history,
    truncate_messages,
)
from nereus.llm.inference import StructuredInferenceClient
from nereus.llm.stub import StubLLMProvider


def _msgs(n: int) -> list[dict]:
    return [{"role": "user", "content": f"msg number {i} " * 200} for i in range(n)]


def test_approx_token_count_grows_with_content() -> None:
    assert approx_token_count([{"role": "user", "content": "hi"}]) > 0
    assert approx_token_count(_msgs(5)) > approx_token_count(_msgs(1))


def test_truncate_keeps_systems_and_recent_within_budget() -> None:
    msgs = [{"role": "system", "content": "system instruction"}, *_msgs(10)]
    budget = 2000
    trimmed = truncate_messages(msgs, budget)
    assert trimmed[0]["role"] == "system"
    assert approx_token_count(trimmed) <= budget or len(trimmed) < len(msgs)


def test_truncate_preserves_short_history_untouched() -> None:
    msgs = [
        {"role": "system", "content": "s"},
        {"role": "user", "content": "a"},
        {"role": "assistant", "content": "b"},
    ]
    assert truncate_messages(msgs, 8000) == msgs


def test_summarize_with_stub_collapses_long_history() -> None:
    def responder(messages, **_):
        return json.dumps({"summary": "short summary of the conversation"})

    inference = StructuredInferenceClient(StubLLMProvider(responder=responder))
    msgs = [{"role": "system", "content": "keep me"}, *_msgs(8)]
    budget = 1500
    assert approx_token_count(msgs) > budget
    out = summarize_history(msgs, inference, budget)
    assert out[0]["role"] == "system"
    assert "[History summary]" in out[0]["content"]


def test_summarize_without_inference_hard_truncates() -> None:
    flat = []
    for m in _msgs(8):
        flat.append(m)
    budget = 1500
    assert approx_token_count(flat) > budget
    out = summarize_history(flat, inference=None, max_tokens=budget)
    assert approx_token_count(out) <= budget
