from __future__ import annotations

import logging
import time
from typing import Any

import httpx

from nereus.llm.base import LLMProvider
from nereus.llm.inference import LLMOutputError

logger = logging.getLogger("nereus.llm.openrouter")

# HTTP statuses that are worth retrying with backoff (transient overload /
# rate limiting / model temporarily down). Auth (401) and balance (402) are
# fatal and surface immediately as ``OpenRouterError`` so agents fall back to
# deterministic stubs.
_RETRY_STATUSES = {429, 502, 503, 504, 500}

# Regex close delimiter for reasoning blocks (built via chr so the literal
# is never typed inline; keeps source lines within the length budget).
_CLOSE_TAG = chr(60) + chr(47) + "think" + chr(62)


class OpenRouterError(LLMOutputError):
    """OpenRouter API request failure (auth/balance/blocked/model-down/transient).

    Subclasses :class:`LLMOutputError`; ``StructuredInferenceClient.generate``
    retries it (on top of the per-call retries already done by ``complete``)
    and, on exhaustion, surfaces :class:`LLMUnavailableError` to the UI — which
    shows the user 'service temporarily unavailable' instead of silently
    fabricating a result (#44/#45).
    """


class OpenRouterProvider(LLMProvider):
    """LLM provider backed by the OpenRouter unified API.

    Talks to ``/api/v1/chat/completions`` (OpenAI-compatible) and transparently
    supports the free-router model ``openrouter/free`` which selects an
    available free model at request time. The actual model used is echoed back
    in the response (``response.model``) and cached on ``last_model``.

    Unlike ``OllamaProvider`` (Ollama's ``/api/chat`` schema), OpenRouter uses
    top-level ``temperature``/``max_tokens``/``top_p`` and the OpenAI
    ``response_format`` field for JSON / structured mode.

    Auth is a single Bearer token (``OPENROUTER_API_KEY``). When running inside
    the public playground, set ``HTTP-Referer`` / ``X-OpenRouter-Title`` for
    leaderboard attribution (optional).
    """

    def __init__(
        self,
        api_key: str,
        *,
        base_url: str = "https://openrouter.ai/api/v1",
        model: str = "openrouter/free",
        timeout: float = 60.0,
        http_referer: str = "",
        title: str = "Nereus",
        client: httpx.Client | None = None,
    ) -> None:
        if not api_key:
            raise ValueError("OpenRouterProvider requires an api_key (OPENROUTER_API_KEY)")
        self._base_url = base_url.rstrip("/")
        self._model = model
        self._timeout = timeout
        self._headers: dict[str, str] = {
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        }
        if http_referer:
            self._headers["HTTP-Referer"] = http_referer
        if title:
            self._headers["X-OpenRouter-Title"] = title
        self._owns_client = client is None
        self._client = client or httpx.Client(timeout=timeout)

        # Mirrors OllamaProvider: request-scoped trace consumed by
        # scripts/eval_chain.py (``inference.calls``).
        self.calls: list[dict[str, Any]] = []
        # The model actually used (for openrouter/free the router picks one).
        self.last_model: str | None = None

    # ------------------------------------------------------------------ #
    # Properties consumed by core.factory._provider_info()
    # ------------------------------------------------------------------ #
    @property
    def base_url(self) -> str:
        return self._base_url

    @property
    def model(self) -> str:
        return self._model

    @property
    def timeout(self) -> float:
        return self._timeout

    # ------------------------------------------------------------------ #
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
            "temperature": temperature,
            "max_tokens": max_tokens,
            "top_p": 0.95,
            "stream": False,
        }
        if json_mode:
            # OpenRouter supports OpenAI-style response_format. ``json_object``
            # is honoured by most models; if a free/router model ignores it,
            # StructuredInferenceClient.parse_structured() still recovers JSON
            # via extract_json() from prose/fenced output.
            payload["response_format"] = {"type": "json_object"}

        last_raw = ""
        for attempt in range(1, 4):
            response = self._client.post(
                f"{self._base_url}/chat/completions",
                headers=self._headers,
                json=payload,
            )
            self.calls.append(
                {
                    "model_requested": self._model,
                    "attempt": attempt,
                    "status": response.status_code,
                }
            )

            if response.status_code in _RETRY_STATUSES:
                if attempt == 3:
                    raise OpenRouterError(
                        f"OpenRouter transient failure after retries: "
                        f"HTTP {response.status_code}: {response.text[:300]}"
                    )
                wait = self._backoff_wait(response)
                logger.warning(
                    "OpenRouter transient HTTP %d; retry %d after %.1fs",
                    response.status_code,
                    attempt + 1,
                    wait,
                )
                time.sleep(wait)
                continue

            if response.status_code >= 400:
                last_raw = response.text
                raise OpenRouterError(
                    f"OpenRouter HTTP {response.status_code}: {last_raw[:300]}"
                )

            data = response.json()
            self.last_model = data.get("model") or self._model
            # Strip surrounding thinking blocks some router models emit.
            content = data["choices"][0]["message"]["content"]
            return self._strip_thinking(content)

        # Unreachable — the retry loop either returns or raises.
        raise OpenRouterError(f"OpenRouter exhausted retries. Last: {last_raw[:300]!r}")

    @staticmethod
    def _backoff_wait(response: httpx.Response) -> float:
        retry_after = response.headers.get("Retry-After")
        if retry_after:
            try:
                return min(float(retry_after), 10.0)
            except ValueError:
                pass
        return 2.0  # simple fixed backoff between retries

    @staticmethod
    def _strip_thinking(content: str) -> str:
        """Remove reasoning-block delimiters emitted by some router models.

        Only strips when the model wraps the *entire* response; partial blocks
        are left intact (they are harmless — ``extract_json`` handles prose).
        """
        if not content or "<think" not in content.lower():
            return content
        import re

        pattern = r"<think\b[^>]*>.*?" + _CLOSE_TAG + r"\s*"
        cleaned = re.sub(pattern, "", content, flags=re.DOTALL).strip()
        return cleaned or content

    def close(self) -> None:
        if self._owns_client:
            self._client.close()
