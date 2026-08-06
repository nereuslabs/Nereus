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
