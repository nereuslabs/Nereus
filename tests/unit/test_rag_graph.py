from __future__ import annotations

from nereus.core.graph import NereusGraph
from nereus.core.state import RetrievedChunk


def _counting_retriever():
    seen: list[dict] = []

    class CountingRetriever:
        def upsert(self, topic, chunks):
            return []

        def retrieve(self, *, query, topic, top_k=5):
            seen.append({"topic": topic.title})
            return [RetrievedChunk(topic_id=topic.id, content="ctx", score=1.0)]

    return CountingRetriever(), seen


def test_graph_populates_retrieved_chunks_after_run(base_state) -> None:
    graph = NereusGraph()  # stub LLM + default StubRetriever
    final = graph.invoke({**base_state, "user_submission": "this is good"})
    assert final["status"] == "completed"
    assert final["retrieved_chunks"]
    assert isinstance(final["retrieved_chunks"][0], RetrievedChunk)


def test_graph_uses_injected_retriever(base_state) -> None:
    retriever, seen = _counting_retriever()
    graph = NereusGraph(retriever=retriever)  # type: ignore[arg-type]
    final = graph.invoke({**base_state, "user_submission": "this is good"})
    assert seen and seen[0]["topic"]
    assert final["retrieved_chunks"][0].content == "ctx"
    assert final["status"] == "completed"


def test_graph_runs_offline_without_retriever(base_state) -> None:
    graph = NereusGraph()  # defaults: stub LLM + StubRetriever
    final = graph.invoke({**base_state, "user_submission": "this is good"})
    assert final["retrieved_chunks"] is not None
    assert isinstance(final["retrieved_chunks"][0], RetrievedChunk)
