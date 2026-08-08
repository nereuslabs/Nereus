from __future__ import annotations

from typing import Any, Callable

from nereus.llm.base import LLMProvider

# (messages, temperature, max_tokens, json_mode) -> response text
StubResponder = Callable[..., str]


def echo_responder(messages: list[dict[str, Any]], **_: Any) -> str:
    """Default stub: echoes the last user message.

    Good enough to exercise the plumbing without any network or model.
    """
    last = messages[-1] if messages else {}
    return str(last.get("content", ""))


class StubLLMProvider(LLMProvider):
    """In-memory provider used for tests and when no real API is configured.

    The response logic is pluggable via ``responder`` so tests can simulate
    structured JSON output from a model. Marked ``is_offline=True`` so agents
    route to deterministic stub generation instead of calling a model (#44/#45).
    """

    is_offline: bool = True

    def __init__(self, responder: StubResponder | None = None) -> None:
        self._responder = responder or echo_responder
        self.calls: list[list[dict[str, Any]]] = []

    def complete(
        self,
        messages: list[dict[str, Any]],
        *,
        temperature: float = 0.2,
        max_tokens: int = 4096,
        json_mode: bool = False,
    ) -> str:
        self.calls.append(messages)
        return self._responder(
            messages, temperature=temperature, max_tokens=max_tokens, json_mode=json_mode
        )

    def close(self) -> None:
        pass
