from __future__ import annotations

import logging
import time
from dataclasses import asdict
from typing import Any, Sequence

from pydantic import BaseModel

from nereus.llm.base import LLMProvider
from nereus.llm.params import AgentRole, params_for

logger = logging.getLogger("nereus.llm")


class LLMOutputError(RuntimeError):
    """Raised when the LLM output cannot be coerced into the required schema
    after all retry attempts are exhausted."""


class StructuredInferenceClient:
    """Typed LLM inference with schema validation and bounded retries.

    Wraps a raw :class:`LLMProvider` and returns validated Pydantic models.
    On parse/validation failure it retries (up to ``max_retries``) with an
    extra system hint that forces strict JSON; when retries are exhausted it
    raises :class:`LLMOutputError` so callers can fall back to deterministic
    stubs.
    """

    def __init__(
        self,
        provider: LLMProvider,
        *,
        max_retries: int = 2,
    ) -> None:
        self._provider = provider
        self._max_retries = max_retries
        self.calls: list[dict[str, Any]] = []

    @property
    def provider(self) -> LLMProvider:
        return self._provider

    def model_name(self) -> str:
        p = getattr(self._provider, "model", None)
        return str(p) if p is not None else self._provider.__class__.__name__

    def generate(
        self,
        messages: Sequence[dict[str, Any]],
        *,
        role: AgentRole,
        output_model: type[BaseModel],
    ) -> BaseModel:
        params = params_for(role)
        hint = self._refocus_hint(output_model)

        attempts = 1 + self._max_retries
        last_raw = ""
        for attempt in range(1, attempts + 1):
            if attempt > 1:
                messages = list(messages) + [
                    {
                        "role": "user",
                        "content": (
                            "ERROR: the previous response was not valid JSON matching the "
                            f"required schema. Retry and return ONLY valid JSON. {hint}"
                        ),
                    }
                ]

            t0 = time.perf_counter()
            raw = self._provider.complete(
                messages,
                temperature=params.temperature,
                max_tokens=params.max_tokens,
                json_mode=params.json_mode,
            )
            latency = time.perf_counter() - t0
            last_raw = raw
            self.calls.append(
                {
                    "role": role.value,
                    "model": self.model_name(),
                    "params": asdict(params),
                    "attempt": attempt,
                    "latency_ms": round(latency * 1000, 1),
                    "raw_fragment": raw[:500],
                }
            )
            logger.debug(
                "llm %s model=%s attempt=%d ok=parsing latency_ms=%.0f",
                role.value,
                self.model_name(),
                attempt,
                latency * 1000,
            )

            try:
                from nereus.llm.schema import parse_structured

                return parse_structured(raw, output_model)
            except Exception as exc:  # noqa: BLE001
                logger.debug(
                    "llm %s parse fail attempt=%d err=%s",
                    role.value,
                    attempt,
                    type(exc).__name__,
                )
                continue

        logger.info(
            "llm %s exhausted %d attempts; raising LLMOutputError",
            role.value,
            attempts,
        )
        raise LLMOutputError(
            f"{role.value} output could not be parsed as {output_model.__name__} "
            f"after {attempts} attempts. Last raw: {last_raw[:300]!r}"
        )

    @staticmethod
    def _refocus_hint(output_model: type[BaseModel]) -> str:
        fields = ", ".join(f'"{f}"' for f in output_model.model_fields)
        return f"Required JSON fields: {fields}."
