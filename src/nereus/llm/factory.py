from __future__ import annotations

from nereus.config.settings import settings
from nereus.llm.base import LLMProvider
from nereus.llm.ollama import OllamaProvider
from nereus.llm.stub import StubLLMProvider


def build_llm_provider() -> LLMProvider:
    """Create the provider configured by the current settings.

    * ``llm_provider=ollama`` -> real native Ollama HTTP client
    * otherwise                -> in-memory stub (no network)
    """
    if settings.llm_provider == "ollama":
        return OllamaProvider(
            base_url=settings.ollama_base_url,
            model=settings.ollama_model,
            api_key=settings.ollama_api_key,
        )
    return StubLLMProvider()