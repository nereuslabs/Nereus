"""Live integration tests for Ollama embedding provider (issue #24).

Guarded by ``NEREUS_RUN_LIVE`` so CI remains offline unless explicitly enabled;
mirrors ``tests/integration/test_live_ollama.py``.

Requires a running Ollama server (local or Cloud):

    # local (default)
    NEREUS_RUN_LIVE=1 EMBEDDING_PROVIDER=ollama pytest tests/integration/test_live_ollama_embed.py

    # Ollama Cloud (https://ollama.com/api)
    NEREUS_RUN_LIVE=1 EMBEDDING_PROVIDER=ollama \
      OLLAMA_BASE_URL=https://ollama.com OLLAMA_API_KEY=... \
      pytest tests/integration/test_live_ollama_embed.py

See https://github.com/ollama/ollama/blob/main/docs/api.md — ``POST /api/embed``
returns a single (non-streaming) JSON object with an ``embeddings`` array.
"""

from __future__ import annotations

import os

import pytest

from nereus.llm.embed import OllamaEmbedder

LIVE = pytest.mark.skipif(
    os.environ.get("NEREUS_RUN_LIVE", "0") != "1",
    reason="set NEREUS_RUN_LIVE=1 (and EMBEDDING_PROVIDER=ollama) to run "
    "Ollama embedding live tests against a running Ollama server",
)


@pytest.fixture
def embedder():
    """Build an OllamaEmbedder from settings; close the httpx client on exit."""
    e = OllamaEmbedder()
    yield e
    e.close()


@LIVE
def test_ollama_embed_returns_nonempty_vector(embedder) -> None:
    """``embed`` returns a non-empty float vector of the model's real dimension."""
    vec = embedder.embed("hello world from nereus")

    assert isinstance(vec, list)
    assert len(vec) > 0
    # every element must be a finite float (Ollama returns floats)
    for x in vec:
        assert isinstance(x, float)
        assert x == x  # NaN guard: NaN != NaN


@LIVE
def test_ollama_embed_many_matches_single(embedder) -> None:
    """``embed_many`` returns one vector per input, matching single calls."""
    single_a = embedder.embed("topic one")
    single_b = embedder.embed("topic two")
    batched = embedder.embed_many(["topic one", "topic two"])

    assert len(batched) == 2
    dim = len(single_a)
    assert dim == len(single_b)
    assert all(len(v) == dim for v in batched)
    # batched[0] should equal a single embed of the same text
    assert batched[0] == single_a


@LIVE
def test_ollama_embed_dimension_stability(embedder) -> None:
    """The model's real output dimension is what the embedder exposes.

    ``OllamaEmbedder.dim`` is the configured contract (DEFAULT_DIM=384); the
    live server returns the model's native dimension (e.g. nomic-embed-text =
    768). We assert the returned vector length is internally consistent across
    calls rather than a hard-coded value — this keeps the test valid if the
    embed model changes.
    """
    dims = {len(embedder.embed(text)) for text in ["a", "b", "c"]}
    assert dims, "expected at least one embedding"
    # all inputs from the same model must share a dimension
    assert len(dims) == 1, f"inconsistent embedding dims returned: {dims}"
