from __future__ import annotations

import logging

from nereus.config.settings import settings
from nereus.llm.base import LLMProvider
from nereus.llm.openrouter import OpenRouterProvider
from nereus.llm.stub import StubLLMProvider

logger = logging.getLogger("nereus")


def build_llm_provider() -> LLMProvider:
    """Create the provider configured by the current settings.

    * ``llm_provider=openrouter`` -> OpenRouter unified API (cloud)
    * otherwise                  -> in-memory stub (no network)

    Selecting ``openrouter`` without an ``OPENROUTER_API_KEY`` falls back to the
    offline stub (with a logged warning) so the UI/CLI still boot — matching the
    project's "stub is the safe default" contract — instead of crashing at
    graph-construction time.
    """
    if settings.llm_provider == "openrouter":
        if not settings.openrouter_api_key:
            logger.warning(
                "LLM_PROVIDER=openrouter selected but OPENROUTER_API_KEY is empty; "
                "falling back to the offline stub provider. Set OPENROUTER_API_KEY "
                "(see .env.example) to call OpenRouter."
            )
            return StubLLMProvider()
        return OpenRouterProvider(
            api_key=settings.openrouter_api_key,
            base_url=settings.openrouter_base_url,
            model=settings.openrouter_model,
            timeout=settings.openrouter_timeout,
            http_referer=settings.openrouter_http_referer,
            title=settings.openrouter_title,
        )
    return StubLLMProvider()
