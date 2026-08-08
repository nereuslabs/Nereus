#!/usr/bin/env python3
"""RAG material ingestion script for Nereus.

Walks ``--materials`` dir, chunks each file, embeds the chunks via the
configured ``Embedder`` (``EMBEDDING_PROVIDER``: stub/sentence_transformers/
openrouter), and upserts them into ChromaDB (``ChromaStore``) tagged with the
topic_id derived from the filename stem (e.g. ``1.md`` -> topic ``"1"``).

```bash
# offline demo (stub embeddings — vectors are fake but structurally valid)
python scripts/ingest_materials.py --materials materials

# real embeddings + live ChromaDB
EMBEDDING_PROVIDER=sentence_transformers CHROMADB_HOST=localhost \
  python scripts/ingest_materials.py --materials materials --clear
```

Designed to run against the compose ``chromadb`` service:

    docker compose run --rm ingest
"""

from __future__ import annotations

import argparse
import logging
import sys
from pathlib import Path

from nereus.config.settings import settings
from nereus.db.chroma import ChromaStore
from nereus.llm.embed import build_embedder

logger = logging.getLogger("nereus.ingest")

# Target characters per chunk (simple fixed-size splitter; good enough for the
# demo materials which are short).
CHUNK_SIZE = 500
CHUNK_OVERLAP = 50
COLLECTION = "nereus"


def _chunk_text(text: str, *, size: int = CHUNK_SIZE, overlap: int = CHUNK_OVERLAP) -> list[str]:
    """Greedy whitespace-aware splitter into ~``size``-char chunks."""
    text = text.strip()
    if len(text) <= size:
        return [text]
    chunks: list[str] = []
    start = 0
    while start < len(text):
        end = min(start + size, len(text))
        # prefer to break on a space near the boundary for readability
        if end < len(text) and text[end] != " ":
            space = text.rfind(" ", start, end)
            if space > start:
                end = space
        chunk = text[start:end].strip()
        if chunk:
            chunks.append(chunk)
        start = end - overlap if end - overlap > start else end
    return chunks or [text]


def _topic_id(path: Path) -> str:
    """Derive topic_id from filename stem (e.g. ``1-fundamentals.md`` -> ``1``).

    Falls back to the full stem if no leading number."""
    name = path.stem
    # take a leading numeric token if present, else the raw stem
    digits = ""
    for ch in name:
        if ch.isdigit():
            digits += ch
        elif digits:
            break
    return digits or name


def ingest(materials_dir: Path, *, clear: bool = False) -> int:
    """Ingest all ``.md`` files under *materials_dir* into ChromaDB.

    Returns the number of chunks upserted. Raises if ChromaDB is unreachable.
    """
    files = sorted(p for p in materials_dir.iterdir() if p.suffix == ".md")
    if not files:
        raise SystemExit(f"no .md materials found in {materials_dir}")

    store = ChromaStore(host=settings.chromadb_host, port=settings.chromadb_port)
    embedder = build_embedder()
    logger.info(
        "ingest | materials=%d provider=%s chromadb=%s collection=%s",
        len(files),
        type(embedder).__name__,
        store.endpoint,
        COLLECTION,
    )

    if clear:
        logger.info("clearing existing collection %s", COLLECTION)
        store._collection().delete()  # noqa: SLF001 — demo script

    total = 0
    for path in files:
        topic_id = _topic_id(path)
        text = path.read_text(encoding="utf-8")
        chunks = _chunk_text(text)
        embeddings = embedder.embed_many(chunks)
        metadatas = [{"topic_id": topic_id, "source": path.name} for _ in chunks]
        ids = [f"{topic_id}-{i}" for i in range(len(chunks))]
        ids = store.add_documents(chunks, embeddings, ids=ids, metadatas=metadatas)
        total += len(chunks)
        logger.info("  %-18s topic=%s chunks=%d", path.name, topic_id, len(chunks))

    logger.info("done | upserted %d chunks across %d topics", total, len(files))
    return total


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="ingest_materials",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--materials",
        type=Path,
        default=Path("materials"),
        help="directory of topic .md files (filename stem -> topic_id)",
    )
    parser.add_argument(
        "--clear",
        action="store_true",
        help="delete the existing ChromaDB collection before upserting",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="chunk + embed only; skip writing to ChromaDB",
    )
    parser.add_argument("--log-level", default="INFO")
    args = parser.parse_args(argv)

    logging.basicConfig(
        level=args.log_level,
        format="%(asctime)s %(levelname)s %(name)s | %(message)s",
    )

    if not args.materials.is_dir():
        print(f"error: materials dir not found: {args.materials}", file=sys.stderr)
        return 2

    if args.dry_run:
        from nereus.llm.embed import Embedder

        logger.info("dry-run | chunking + embedding only")
        embedder: Embedder = build_embedder()
        files = sorted(p for p in args.materials.iterdir() if p.suffix == ".md")
        for path in files:
            chunks = _chunk_text(path.read_text(encoding="utf-8"))
            embedder.embed_many(chunks)
            logger.info("  %-18s topic=%s chunks=%d", path.name, _topic_id(path), len(chunks))
        logger.info("dry-run | %d files would be ingested", len(files))
        return 0

    try:
        ingest(args.materials, clear=args.clear)
    except Exception as exc:  # noqa: BLE001
        logger.error("ingest failed: %s", exc)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
