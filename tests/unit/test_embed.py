from __future__ import annotations

from nereus.config.settings import settings
from nereus.llm.embed import (
    DEFAULT_DIM,
    Embedder,
    OllamaEmbedder,
    StubEmbedder,
    build_embedder,
)


def test_stub_embedder_is_deterministic_and_fixed_dim() -> None:
    embedder = StubEmbedder(dim=16)
    v1 = embedder.embed("hello")
    v2 = embedder.embed("hello")
    assert embedder.dim == 16
    assert v1 == v2
    assert len(v1) == 16
    # deterministic but distinct for distinct text
    assert v1 != embedder.embed("world")


def test_stub_embedder_many_matches_single() -> None:
    embedder = StubEmbedder(dim=8)
    assert embedder.embed_many(["a", "b"]) == [embedder.embed("a"), embedder.embed("b")]


def test_stub_is_embedder_protocol() -> None:
    assert isinstance(StubEmbedder(), Embedder)


def test_default_dim_matches_sentence_transformers_space() -> None:
    assert DEFAULT_DIM == 384


def test_build_embedder_respects_stub_default(monkeypatch) -> None:
    monkeypatch.setattr(settings, "embedding_provider", "stub")
    assert isinstance(build_embedder(), StubEmbedder)


def test_ollama_embedder_forwards_bearer_auth() -> None:
    """Regression for #34: OllamaEmbedder must send the API key as a Bearer
    token so Ollama Cloud (/api/embed) authorizes — mirroring OllamaProvider.
    Local Ollama ignores the header, so always sending it is safe."""
    embedder = OllamaEmbedder(
        base_url="https://ollama.com",
        model="nomic-embed-text",
        api_key="test-key-123",
        timeout=30,
    )
    try:
        headers = embedder._headers()
        assert headers["Authorization"] == "Bearer test-key-123"
        assert headers["Content-Type"] == "application/json"
    finally:
        embedder.close()


def test_ollama_embedder_omits_auth_when_no_key(monkeypatch) -> None:
    """No key -> no Authorization header (local Ollama doesn't need one).

    ``settings.ollama_api_key`` is patched empty so the assertion holds
    regardless of a developer's local .env.
    """
    monkeypatch.setattr(settings, "ollama_api_key", "")
    embedder = OllamaEmbedder(
        base_url="http://localhost:11434",
        model="nomic-embed-text",
        timeout=30,
    )
    try:
        headers = embedder._headers()
        assert "Authorization" not in headers
    finally:
        embedder.close()
