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


def test_collection_name_attr_does_not_shadow_method() -> None:
    """Regression: ``self._collection`` (name) must not shadow the ``_collection()``
    method — otherwise ``add_documents``/``search`` raise ``'str' object is not
    callable``. The name lives in ``_collection_name``; the resolver is a method."""
    store = ChromaStore(host="db", port=8000, collection="nereus")

    # The collection *name* is an attribute …
    assert store._collection_name == "nereus"
    # … and the resolver is a *method* (callable), not the string.
    assert callable(store._collection)
    assert store._collection.__name__ == "_collection"

    # With _client unset, the method should still raise at connect() — not
    # TypeError from calling a string.
    with pytest.raises((NotImplementedError, Exception)) as exc_info:  # noqa: PT011
        store._collection()
    assert not isinstance(exc_info.value, TypeError)
    # Must NOT be the shadowing TypeError "'str' object is not callable".
    assert "not callable" not in str(exc_info.value)
