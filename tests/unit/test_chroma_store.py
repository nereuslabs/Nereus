from __future__ import annotations

from nereus.db.chroma import ChromaStore


def test_chroma_store_endpoint_and_unconnected_guard() -> None:
    store = ChromaStore(host="db", port=8000)
    assert store.endpoint == "http://db:8000"
    assert store._client is None
    import pytest

    with pytest.raises(NotImplementedError):
        store.search("c", "q")
    with pytest.raises(NotImplementedError):
        store.add_documents("c", ["d"], [[0.0]])
