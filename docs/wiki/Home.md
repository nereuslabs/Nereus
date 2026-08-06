# Nereus Wiki

> Интерактивный ассистент по обучению: LLM‑агент, который разбирает ваш roadmap,
> задаёт вопросы, проверяет знания и адаптирует план — всё через обычный
> диалог в терминале/браузере.

## Содержание
- [Architecture](Architecture.md)
- [Roadmap](Roadmap.md)
- [Development](Development.md)
- [Migration guide](MIGRATE.md) (перенести `docs/wiki/` в настоящий GitHub Wiki)

## Статус
- ✅ **Step 1** — MVP learning automaton (LangGraph)
- ✅ **Step 2** — LLM provider abstraction + Ollama
- ✅ **Step 3** — LLM runtime, prompt registry, schema validation, session memory
- ✅ **Step 4** — ChromaDB RAG (embeddings, retriever, tutor integrate)
- ✅ **Step 5** — Chainlit Web UI (`src/nereus/ui/app.py`)
- ✅ **Step 6** — Persistent session dump/load (`LearningSession` ↔ JSON, runtime-wired #22)
  + persistent checkpointer (sqlite default, redis fallback)

Первая полностью рабочая версия — **MVP 1.0 GA** — отслеживается в
[MVP 1.0](https://github.com/Yan123-tech/Nereus/milestones/1).

## Быстрый старт
```bash
python -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"

# CLI (human-in-the-loop через input; офлайн по умолчанию)
python main.py
python main.py --resume nereus-demo          # возобновить сессию

# Web UI на базе Chainlit (:7457)
chainlit run src/nereus/ui/app.py

# Через Docker (сервис nereus-ui на http://localhost:7457)
docker compose up -d --build ui
```

## Тесты
```bash
ruff check . && ruff format --check .
pytest                                    # 99 passed, 2 skipped (live)
NEREUS_RUN_LIVE=1 pytest                  # + Ollama/Redis/SQLite live
```

Ссылки: [Architecture](Architecture.md) | [Roadmap](Roadmap.md) | [Development](Development.md) | [Migration guide](MIGRATE.md)

### Конфиг (.env, все опциональны)
| Параметр | Знач. по умолчанию | Описание |
|---|---|---|
| `LLM_PROVIDER` | `stub` | `stub \| ollama \| openai` |
| `EMBEDDING_PROVIDER` | `stub` | `stub \| sentence_transformers \| ollama` |
| `CHECKPOINTER` | `memory` | `memory \| sqlite \| redis` (default `memory`, офлайн) |
| `SESSION_PATH` | `.sessions/{thread_id}.json` | файл сессии |
| `CHROMADB_HOST` | `localhost` | ChromaDB для RAG |


