# Nereus Wiki

> Интерактивный ассистент по обучению: LLM‑агент, который разбирает ваш roadmap,
> задаёт вопросы, проверяет знания и адаптирует план — всё через обычный
> диалог в терминале/браузере.

## Содержание
- [Architecture](Architecture.md)
- [Roadmap](Roadmap.md)
- [Development](Development.md)

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
python -m venv .venv && . .venv/bin/activate
pip install -e ".[dev]"
python main.py              # LLM_PROVIDER=stub → полностью офлайн
chainlit run src/nereus/ui/app.py   # Web UI :7457
```

### Конфиг (.env, все опциональны)
| Параметр | Знач. по умолчанию | Описание |
|---|---|---|
| `LLM_PROVIDER` | `stub` | `stub \| ollama \| openai` |
| `EMBEDDING_PROVIDER` | `stub` | `stub \| sentence_transformers \| ollama` |
| `CHECKPOINTER` | `sqlite` | `memory \| sqlite \| redis` |
| `SESSION_PATH` | `.sessions/{thread_id}.json` | файл сессии |
| `CHROMADB_HOST` | `localhost` | ChromaDB для RAG |


