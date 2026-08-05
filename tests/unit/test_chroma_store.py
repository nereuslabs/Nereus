from __future__ import annotations

import pytest

from nereus.db.chroma import ChromaStore


def test_chroma_store_endpoint_and_unconnected_guard() -> None:
    store = ChromaStore(host="db", port=8000, collection="nereus")
    assert store.endpoint == "http://db:8000"
    assert store._client is None

    with pytest.raises(NotImplementedError):
        store.search([0.0])
    with pytest.raises(NotImplementedError):
        store.add_documents(["d"], [[0.0]])
