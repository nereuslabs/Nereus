from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any


class LLMProvider(ABC):
    """Unified interface to an LLM backend (Ollama local / Cloud, etc.).

    Providers receive OpenAI-style messages and return the assistant text.
    Agents depend on this abstraction so the backend can be swapped without
    touching business logic (local Ollama, Ollama Cloud Free Tier, ...).
    """

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
