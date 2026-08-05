from __future__ import annotations

import logging
from typing import Protocol, Sequence, runtime_checkable

from nereus.core.state import RetrievedChunk, RoadmapTopic
from nereus.db.chroma import ChromaStore
from nereus.llm.embed import DEFAULT_DIM, Embedder, StubEmbedder

logger = logging.getLogger("nereus.llm.retriever")


@runtime_checkable
class Retriever(Protocol):
    """RAG retrieval abstraction: chunk documents by topic and fetch context."""

    def upsert(self, topic: RoadmapTopic, chunks: Sequence[str]) -> list[str]: ...

    def retrieve(
        self, *, query: str, topic: RoadmapTopic, top_k: int = 5
    ) -> list[RetrievedChunk]: ...


class StubRetriever:
    """Deterministic, network-free retriever for offline/CI.

    Returns canned context derived from the topic so the automaton stays fully
    runnable without a ChromaDB server or embedding backend.
    """

    def __init__(self, embedder: Embedder | None = None) -> None:
        self._store: dict[str, list[str]] = {}
        self._embedder: Embedder = embedder or StubEmbedder()

    def upsert(self, topic: RoadmapTopic, chunks: Sequence[str]) -> list[str]:
        stored = self._store.setdefault(topic.id, [])
        stored.extend(chunks)
        return list(chunks)

    def retrieve(
        self, *, query: str, topic: RoadmapTopic, top_k: int = 5
    ) -> list[RetrievedChunk]:
        stored = self._store.get(topic.id, [])
        if stored:
            query_vec = self._embedder.embed(query)
            rank: list[tuple[float, str]] = []
            for chunk in stored:
                chunk_vec = self._embedder.embed(chunk)
                score = _cosine(query_vec, chunk_vec)
                rank.append((score, chunk))
            rank.sort(key=lambda pair: pair[0], reverse=True)
            chosen = [c for _, c in rank[:top_k]]
        else:
            chosen = [
                (
                    f"[RAG-stub] Context for topic '{topic.title}': "
                    f"{topic.description}. Core concepts summarized."
                )
            ]
        return [
            RetrievedChunk(
                topic_id=topic.id, content=content, score=1.0
            )
            for content in chosen
        ]


class ChromaRetriever:
    """Retriever backed by a ``ChromaStore`` + ``Embedder`` pair."""

    def __init__(self, store: ChromaStore, embedder: Embedder, *, dim: int = DEFAULT_DIM) -> None:
        self._store = store
        self._embedder = embedder
        self._dim = dim

    def upsert(self, topic: RoadmapTopic, chunks: Sequence[str]) -> list[str]:
        embeddings = self._embedder.embed_many(list(chunks))
        metadatas = [{"topic_id": topic.id, "topic_title": topic.title} for _ in chunks]
        ids = [f"{topic.id}-{i}" for i in range(len(chunks))]
        return self._store.add_documents(
            list(chunks), embeddings, ids=ids, metadatas=metadatas
        )

    def retrieve(
        self, *, query: str, topic: RoadmapTopic, top_k: int = 5
    ) -> list[RetrievedChunk]:
        query_vec = self._embedder.embed(query)
        hits = self._store.search(
            query_vec, top_k=top_k, where={"topic_id": topic.id}
        )
        if not hits:
            return []
        return [
            RetrievedChunk(
                topic_id=topic.id,
                content=hit["content"],
                score=float(hit["score"]),
            )
            for hit in hits
        ]


def _cosine(a: Sequence[float], b: Sequence[float]) -> float:
    """Simple cosine similarity (no extra deps); vectors must be equal length."""
    la = sum(x * x for x in a) ** 0.5
    lb = sum(x * x for x in b) ** 0.5
    if la == 0 or lb == 0:
        return 0.0
    return sum(x * y for x, y in zip(a, b, strict=True)) / (la * lb)
