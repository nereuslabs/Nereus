from __future__ import annotations

from nereus.core.state import RetrievedChunk, RoadmapTopic
from nereus.llm.embed import StubEmbedder
from nereus.llm.retriever import ChromaRetriever, Retriever, StubRetriever


def _topic() -> RoadmapTopic:
    return RoadmapTopic(id="1", title="Python: loops", description="for and while loops")


def test_stub_retriever_is_retriever_protocol() -> None:
    assert isinstance(StubRetriever(), Retriever)


def test_stub_retriever_returns_fallback_chunk_when_empty() -> None:
    chunks = StubRetriever().retrieve(query="loops", topic=_topic())
    assert len(chunks) == 1
    assert isinstance(chunks[0], RetrievedChunk)
    assert chunks[0].topic_id == "1"
    assert "Python: loops" in chunks[0].content


def test_stub_retriever_returns_upserted_chunks() -> None:
    retriever = StubRetriever()
    retriever.upsert(_topic(), ["chunk A about loops", "chunk B about arrays"])
    chunks = retriever.retrieve(query="loops", topic=_topic(), top_k=5)
    assert len(chunks) == 2
    contents = {c.content for c in chunks}
    assert "chunk A about loops" in contents
    assert "chunk B about arrays" in contents
    assert all(c.topic_id == "1" for c in chunks)


def test_stub_retriever_ranks_by_similarity() -> None:
    class _FakeEmbedder:
        dim = 2

        def embed(self, text: str) -> list[float]:
            return {"a": [1.0, 0.0], "b": [0.0, 1.0]}.get(text, [0.0, 0.0])

        def embed_many(self, texts):
            return [self.embed(t) for t in texts]

    retriever = StubRetriever(embedder=_FakeEmbedder())  # type: ignore[arg-type]
    retriever.upsert(_topic(), ["a", "b"])
    chunks = retriever.retrieve(query="a", topic=_topic(), top_k=2)
    assert [c.content for c in chunks] == ["a", "b"]


def test_chroma_retriever_with_fake_store() -> None:
    class FakeStore:
        def __init__(self) -> None:
            self.added: tuple | None = None
            self.queries: list[tuple] = []

        def add_documents(self, documents, embeddings, *, ids=None, metadatas=None):
            self.added = (documents, embeddings)
            return ids or [f"doc-{i}" for i in range(len(documents))]

        def search(self, query_embedding, *, top_k=5, where=None):
            self.queries.append((list(query_embedding), top_k, where))
            return [
                {"id": "x", "content": "chunk from chroma", "score": 0.9, "metadata": {}}
            ]

    store = FakeStore()
    retriever = ChromaRetriever(store=store, embedder=StubEmbedder(dim=16))
    ids = retriever.upsert(_topic(), ["doc1", "doc2"])
    assert len(ids) == 2
    assert store.added is not None and len(store.added[0]) == 2  # type: ignore[index]

    chunks = retriever.retrieve(query="loops", topic=_topic(), top_k=3)
    assert len(chunks) == 1
    assert chunks[0].content == "chunk from chroma"
    assert chunks[0].score == 0.9
    assert chunks[0].topic_id == "1"
    # topic_id filter is forwarded to the store
    assert store.queries[-1][2] == {"topic_id": "1"}
