from __future__ import annotations

import logging
from typing import Any

logger = logging.getLogger("nereus.db.chroma")


class ChromaStore:
    """Thin adapter over a ChromaDB server for RAG over learning material.

    Step 4 scaffold. Constructed with host/port (mirroring ``settings``);
    methods are stubs that raise ``NotImplementedError`` until the full RAG
    wiring lands. ``chromadb`` is imported lazily so the import graph stays
    light and offline tests are unaffected.
    """

    def __init__(self, host: str = "localhost", port: int = 8000) -> None:
        self._host = host
        self._port = port
        self._client: Any = None

    @property
    def endpoint(self) -> str:
        return f"http://{self._host}:{self._port}"

    def connect(self) -> None:
        try:
            import chromadb  # noqa: F401
        except Exception as exc:  # noqa: BLE001
            logger.warning("chromadb backend unavailable: %s", exc)
            return
        self._client = self._build_client()

    def _build_client(self) -> Any:
        import chromadb

        return chromadb.HttpClient(host=self._host, port=self._port)

    def add_documents(
        self, collection: str, documents: list[str], embeddings: list[list[float]]
    ) -> list[str]:
        if self._client is None:
            raise NotImplementedError("ChromaStore.connect() first")
        col = self._client.get_or_create_collection(name=collection)
        ids = [f"{collection}-{i}" for i in range(len(documents))]
        col.add(ids=ids, embeddings=embeddings, documents=documents)
        return ids

    def search(self, collection: str, query: str, *, top_k: int = 5) -> list[dict[str, Any]]:
        if self._client is None:
            raise NotImplementedError("ChromaStore.connect() first")
        col = self._client.get_collection(name=collection)
        results = col.query(query_texts=[query], n_results=top_k)
        return [
            {"id": rid, "score": float(dist), "content": doc}
            for rid, dist, doc in zip(
                results["ids"][0],
                results["distances"][0],
                results["documents"][0],
                strict=True,
            )
        ]
