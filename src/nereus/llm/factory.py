from __future__ import annotations

from nereus.config.settings import settings
from nereus.llm.base import LLMProvider
from nereus.llm.ollama import OllamaProvider
from nereus.llm.openrouter import OpenRouterProvider
from nereus.llm.stub import StubLLMProvider


def build_llm_provider() -> LLMProvider:
    """Create the provider configured by the current settings.

    * ``llm_provider=openrouter`` -> OpenRouter unified API (cloud)
    * ``llm_provider=ollama``     -> real native Ollama HTTP client (legacy)
    * otherwise                   -> in-memory stub (no network)
    """
    if settings.llm_provider == "openrouter":
        return OpenRouterProvider(
            api_key=settings.openrouter_api_key,
            base_url=settings.openrouter_base_url,
            model=settings.openrouter_model,
            timeout=settings.openrouter_timeout,
            http_referer=settings.openrouter_http_referer,
            title=settings.openrouter_title,
        )
    if settings.llm_provider == "ollama":
        return OllamaProvider(
            base_url=settings.ollama_base_url,
            model=settings.ollama_model,
            api_key=settings.ollama_api_key,
            timeout=settings.ollama_timeout,
        )
    return StubLLMProvider()