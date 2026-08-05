from __future__ import annotations

import hashlib
import logging
from typing import Protocol, Sequence, runtime_checkable

from nereus.config.settings import settings

logger = logging.getLogger("nereus.llm.embed")

# Default dimensionality used by the stub adapter (matches
# all-MiniLM-L6-v2 so real/stub vectors stay interchangeable in tests).
DEFAULT_DIM = 384


@runtime_checkable
class Embedder(Protocol):
    """Maps text -> dense vector. Backends are swappable via ``EMBEDDING_PROVIDER``."""

    @property
    def dim(self) -> int: ...

    def embed(self, text: str) -> list[float]: ...

    def embed_many(self, texts: Sequence[str]) -> list[list[float]]: ...


class StubEmbedder:
    """Deterministic, network-free embedder for offline/CI use.

    The vector is derived from a SHA-256 digest so identical input always
    yields identical output (reproducible retrieval over a stub store).
    """

    def __init__(self, dim: int = DEFAULT_DIM) -> None:
        self.dim = dim

    def embed(self, text: str) -> list[float]:
        return self._vector(text)

    def embed_many(self, texts: Sequence[str]) -> list[list[float]]:
        return [self._vector(t) for t in texts]

    def _vector(self, text: str) -> list[float]:
        digest = hashlib.sha256(text.encode("utf-8")).digest()
        reps = self.dim // len(digest) + 1
        data = digest * reps
        return [(byte / 128.0) - 1.0 for byte in data[: self.dim]]


class SentenceTransformerEmbedder:
    """Embedder backed by a local `sentence-transformers` model.

    ``sentence-transformers`` is imported lazily so the module stays importable
    in offline/CI environments that only use the stub.
    """

    def __init__(self, model_name: str | None = None) -> None:
        from sentence_transformers import SentenceTransformer  # type: ignore

        name = model_name or settings.sentence_transformers_model
        self._model = SentenceTransformer(name)
        self.dim: int = int(self._model.get_sentence_embedding_dimension() or DEFAULT_DIM)
        self.model_name: str = name

    def embed(self, text: str) -> list[float]:
        vec = self._model.encode([text], normalize_embeddings=True)[0]
        return [float(x) for x in vec]

    def embed_many(self, texts: Sequence[str]) -> list[list[float]]:
        vecs = self._model.encode(list(texts), normalize_embeddings=True)
        return [[float(x) for x in vec] for vec in vecs]


class OllamaEmbedder:
    """Embedder that routes through a local/remote Ollama server."""

    def __init__(
        self,
        base_url: str | None = None,
        model: str | None = None,
        timeout: float = 30.0,
    ) -> None:
        import httpx  # local import -> optional at runtime

        self._client = httpx.Client(timeout=timeout)
        self._base_url = (base_url or settings.ollama_base_url).rstrip("/")
        self.model = model or settings.ollama_embed_model
        self.dim: int = DEFAULT_DIM

    def embed(self, text: str) -> list[float]:
        return self._request([text])[0]

    def embed_many(self, texts: Sequence[str]) -> list[list[float]]:
        return self._request(list(texts))

    def _request(self, texts: list[str]) -> list[list[float]]:
        resp = self._client.post(
            f"{self._base_url}/api/embed",
            json={"model": self.model, "input": texts},
        )
        resp.raise_for_status()
        data = resp.json()
        embeddings = data.get("embeddings") or []
        if len(embeddings) != len(texts):
            raise RuntimeError(
                f"Ollama embed returned {len(embeddings)} vectors for {len(texts)} inputs"
            )
        return [[float(x) for x in vec] for vec in embeddings]

    def close(self) -> None:
        self._client.close()


def build_embedder() -> Embedder:
    """Factory driven by ``SETTINGS.embedding_provider``."""
    kind = settings.embedding_provider
    if kind == "sentence_transformers":
        logger.info("embedding provider: sentence_transformers")
        return SentenceTransformerEmbedder()
    if kind == "ollama":
        logger.info("embedding provider: ollama (%s)", settings.ollama_embed_model)
        return OllamaEmbedder()
    logger.info("embedding provider: stub (offline)")
    return StubEmbedder()
