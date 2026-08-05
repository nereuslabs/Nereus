from __future__ import annotations

import logging
from typing import Any, Sequence

logger = logging.getLogger("nereus.db.chroma")


class ChromaStore:
    """Thin adapter over a ChromaDB HTTP server for the RAG store.

    The real ``chromadb`` client is imported lazily so the module (and the
    ``ChromaRetriever`` that wraps it) stays importable in offline/CI
    environments; ``connect()`` raises only when actually used without a server.
    """

    def __init__(
        self,
        host: str = "localhost",
        port: int = 8000,
        *,
        collection: str = "nereus",
    ) -> None:
        self._host = host
        self._port = port
        self._collection = collection
        self._client: Any = None

    @property
    def endpoint(self) -> str:
        return f"http://{self._host}:{self._port}"

    def connect(self) -> Any:
        """Lazily build (and cache) the ``chromadb`` HTTP client."""
        if self._client is not None:
            return self._client
        try:
            import chromadb  # type: ignore
        except Exception as exc:  # noqa: BLE001
            logger.warning("chromadb backend unavailable: %s", exc)
            raise
        self._client = chromadb.HttpClient(host=self._host, port=self._port)
        return self._client

    def _collection(self) -> Any:
        return self.connect().get_or_create_collection(name=self._collection)

    def add_documents(
        self,
        documents: Sequence[str],
        embeddings: Sequence[Sequence[float]],
        *,
        ids: Sequence[str] | None = None,
        metadatas: Sequence[dict[str, Any]] | None = None,
    ) -> list[str]:
        """Upsert documents together with their pre-computed embeddings."""
        if self._client is None:
            raise NotImplementedError("ChromaStore.connect() first")
        embeddings = [list(e) for e in embeddings]
        if ids is None:
            ids = [f"doc-{i}" for i in range(len(documents))]
        if metadatas is None:
            metadatas = [{} for _ in documents]
        col = self._collection()
        col.upsert(
            ids=list(ids),
            embeddings=embeddings,
            documents=list(documents),
            metadatas=list(metadatas),
        )
        return list(ids)

    def search(
        self,
        query_embedding: Sequence[float],
        *,
        top_k: int = 5,
        where: dict[str, Any] | None = None,
    ) -> list[dict[str, Any]]:
        """Return the nearest documents for a query embedding vector.

        ``where`` filters documents by metadata (e.g. ``{"topic_id": "1"}``).
        """
        if self._client is None:
            raise NotImplementedError("ChromaStore.connect() first")
        col = self._collection()
        kwargs: dict[str, Any] = {"query_embeddings": [list(query_embedding)], "n_results": top_k}
        if where is not None:
            kwargs["where"] = where
        results = col.query(include=["documents", "metadatas", "distances"], **kwargs)
        ids = results.get("ids", [[]])[0]
        docs = results.get("documents", [[]])[0]
        dists = results.get("distances", [[]])[0]
        metas = results.get("metadatas", [[]])[0]
        out: list[dict[str, Any]] = []
        for idx, doc in enumerate(docs):
            out.append(
                {
                    "id": ids[idx] if idx < len(ids) else None,
                    "content": doc,
                    "score": float(1.0 - (dists[idx] if idx < len(dists) else 0.0)),
                    "metadata": metas[idx] if idx < len(metas) else {},
                }
            )
        return out
