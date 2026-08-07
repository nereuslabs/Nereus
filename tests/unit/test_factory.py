from __future__ import annotations

from nereus.core.factory import build_nereus_graph
from nereus.llm.ollama import OllamaProvider
from nereus.llm.stub import StubLLMProvider


def test_factory_defaults_to_stub_provider() -> None:
    # Relies on the autouse _force_stub_offline fixture in tests/conftest.py
    # (settings.llm_provider == "stub") so this stays hermetic with a .env present.
    graph = build_nereus_graph(interactive=False)
    provider = graph._coach_agent._inference.provider
    assert isinstance(provider, StubLLMProvider)


def test_factory_accepts_injected_provider() -> None:
    provider = StubLLMProvider()
    graph = build_nereus_graph(interactive=False, provider=provider)
    assert graph._coach_agent._inference.provider is provider
    assert graph._tutor_agent._inference.provider is provider
    assert graph._examiner_agent._inference.provider is provider


def test_factory_resolves_ollama_provider(monkeypatch) -> None:
    from nereus.config import settings as settings_module

    monkeypatch.setattr(settings_module.settings, "llm_provider", "ollama")
    monkeypatch.setattr(settings_module.settings, "ollama_base_url", "http://localhost:11434")
    monkeypatch.setattr(settings_module.settings, "ollama_model", "gemma-test")
    monkeypatch.setattr(settings_module.settings, "ollama_api_key", "k")
    monkeypatch.setattr(settings_module.settings, "ollama_timeout", 42.0)
    graph = build_nereus_graph(interactive=False)
    provider = graph._tutor_agent._inference.provider
    assert isinstance(provider, OllamaProvider)
    assert provider.model == "gemma-test"
    assert provider.base_url == "http://localhost:11434"
    assert provider.timeout == 42


def test_factory_resolves_openrouter_provider(monkeypatch) -> None:
    from nereus.config import settings as settings_module
    from nereus.llm.openrouter import OpenRouterProvider

    monkeypatch.setattr(settings_module.settings, "llm_provider", "openrouter")
    monkeypatch.setattr(settings_module.settings, "openrouter_api_key", "test-key")
    monkeypatch.setattr(settings_module.settings, "openrouter_model", "openrouter/free")
    monkeypatch.setattr(
        settings_module.settings, "openrouter_base_url", "https://openrouter.ai/api/v1"
    )
    monkeypatch.setattr(settings_module.settings, "openrouter_timeout", 60.0)
    graph = build_nereus_graph(interactive=False)
    provider = graph._coach_agent._inference.provider
    assert isinstance(provider, OpenRouterProvider)
    assert provider.model == "openrouter/free"
    assert provider.base_url == "https://openrouter.ai/api/v1"
    assert provider.timeout == 60


def test_factory_openrouter_without_key_falls_back_to_stub(monkeypatch) -> None:
    """Selecting openrouter with an empty key must not crash boot.

    Mirrors the docker-compose default (LLM_PROVIDER=openrouter) on a device
    without OPENROUTER_API_KEY: the UI/CLI should still start offline via stub.
    """
    from nereus.config import settings as settings_module
    from nereus.llm.stub import StubLLMProvider

    monkeypatch.setattr(settings_module.settings, "llm_provider", "openrouter")
    monkeypatch.setattr(settings_module.settings, "openrouter_api_key", "")
    graph = build_nereus_graph(interactive=False)
    provider = graph._coach_agent._inference.provider
    assert isinstance(provider, StubLLMProvider)
