from __future__ import annotations

import hashlib
import logging
from typing import Protocol, Sequence, runtime_checkable

import httpx

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


class OpenRouterEmbedder:
    """Embedder backed by the OpenRouter embeddings endpoint.

    POSTs to ``/api/v1/embeddings`` (OpenAI-compatible) with the same
    ``Authorization: Bearer`` token used for chat. Lets us drop the local
    Ollama embedding container entirely (which pulls ~8GB for ``nomic-embed``).

    ``dim`` is discovered lazily from the first response (different embed
    models expose different dimensionalities).
    """

    def __init__(
        self,
        model: str | None = None,
        *,
        api_key: str = "",
        base_url: str = "https://openrouter.ai/api/v1",
        timeout: float = 30.0,
        client: httpx.Client | None = None,
    ) -> None:
        if not api_key:
            raise ValueError(
                "OpenRouterEmbedder requires an api_key (OPENROUTER_API_KEY)"
            )
        self._base_url = base_url.rstrip("/")
        self.model = model or settings.openrouter_embed_model
        self._timeout = timeout
        self._headers: dict[str, str] = {
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        }
        self._owns_client = client is None
        self._client = client or httpx.Client(timeout=timeout)
        self.dim: int = DEFAULT_DIM  # resolved on first embed response

    def embed(self, text: str) -> list[float]:
        return self.embed_many([text])[0]

    def embed_many(self, texts: Sequence[str]) -> list[list[float]]:
        response = self._client.post(
            f"{self._base_url}/embeddings",
            headers=self._headers,
            json={"model": self.model, "input": list(texts)},
        )
        response.raise_for_status()
        data = response.json()
        vectors = [list(vec) for vec in (d["embedding"] for d in data["data"])]
        if vectors and self.dim == DEFAULT_DIM:
            self.dim = len(vectors[0])
        return [[float(x) for x in vec] for vec in vectors]

    def close(self) -> None:
        if self._owns_client:
            self._client.close()


def build_embedder() -> Embedder:
    """Factory driven by ``SETTINGS.embedding_provider``."""
    kind = settings.embedding_provider
    if kind == "sentence_transformers":
        logger.info("embedding provider: sentence_transformers")
        return SentenceTransformerEmbedder()
    if kind == "openrouter":
        if not settings.openrouter_api_key:
            logger.warning(
                "EMBEDDING_PROVIDER=openrouter selected but OPENROUTER_API_KEY is "
                "empty; falling back to the offline stub embedder. Set "
                "OPENROUTER_API_KEY to use OpenRouter embeddings."
            )
            return StubEmbedder()
        logger.info("embedding provider: openrouter (%s)", settings.openrouter_embed_model)
        return OpenRouterEmbedder(api_key=settings.openrouter_api_key)
    logger.info("embedding provider: stub (offline)")
    return StubEmbedder()
