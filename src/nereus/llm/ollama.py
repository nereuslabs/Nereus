from __future__ import annotations

from typing import Any

import httpx

from nereus.llm.base import LLMProvider


class OllamaProvider(LLMProvider):
    """LLM provider for a native Ollama server (local or Cloud).

    Talks to the ``/api/chat`` endpoint. Models are pulled through the regular
    ``ollama pull <model>`` command and referenced by their tag.
    """

    def __init__(
        self,
        base_url: str,
        model: str,
        *,
        api_key: str = "",
        timeout: float = 120.0,
        client: httpx.Client | None = None,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._model = model
        self._api_key = api_key
        self._timeout = timeout
        self._owns_client = client is None
        self._client = client or httpx.Client(timeout=timeout)

    @property
    def base_url(self) -> str:
        return self._base_url

    @property
    def model(self) -> str:
        return self._model

    @property
    def timeout(self) -> float:
        return self._timeout

    def _headers(self) -> dict[str, str]:
        headers = {"Content-Type": "application/json"}
        if self._api_key:
            headers["Authorization"] = f"Bearer {self._api_key}"
        return headers

    def complete(
        self,
        messages: list[dict[str, Any]],
        *,
        temperature: float = 0.2,
        max_tokens: int = 4096,
        json_mode: bool = False,
    ) -> str:
        payload: dict[str, Any] = {
            "model": self._model,
            "messages": messages,
            "stream": False,
            "options": {
                "temperature": temperature,
                "num_predict": max_tokens,
            },
        }
        if json_mode:
            payload["format"] = "json"

        response = self._client.post(
            f"{self._base_url}/api/chat",
            headers=self._headers(),
            json=payload,
        )
        response.raise_for_status()

        data = response.json()
        return str(data["message"]["content"])

    def close(self) -> None:
        if self._owns_client:
            self._client.close()
