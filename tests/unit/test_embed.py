from __future__ import annotations

from nereus.config.settings import settings
from nereus.llm.embed import (
    DEFAULT_DIM,
    Embedder,
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
