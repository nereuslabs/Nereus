from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any


class LLMProvider(ABC):
    """Unified interface to an LLM backend.

    Providers receive OpenAI-style messages and return the assistant text.
    Agents depend on this abstraction so the backend can be swapped without
    touching business logic (OpenRouter cloud, local stub, ...).
    """

    # True only for offline stub providers (no network). Agents use this to pick
    # deterministic stub generation instead of calling the model, so an offline
    # boot never raises "service unavailable" (#44/#45).
    is_offline: bool = False

    @abstractmethod
    def complete(
        self,
        messages: list[dict[str, Any]],
        *,
        temperature: float = 0.2,
        max_tokens: int = 4096,
        json_mode: bool = False,
    ) -> str:
        """Send a chat completion request and return the assistant message."""

    @abstractmethod
    def close(self) -> None:
        """Release any network resources held by the provider."""
