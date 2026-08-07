# Архитектура

```
main.py ──► core.factory.build_nereus_graph()
                 │
            ┌────┴────┐
            │  State   │ (pydantic, msgpack-serializable, NereusState)
            └────┬────┘
                 │
     ┌───────────┼───────────┐
     │ Coach     │ Tutor     │ Examiner   │  (agents/*.py)
     │ prompts[] │ params    │ schema.py  │
     └───────────┼───────────┘
                 ▼
             llm.inference  (StructuredInferenceClient, retry x2)
                 │
        LLM_PROVIDER = stub | openrouter | ollama
                 │
                 ▼
      ┌─────────────────────────────┐
      │ core/session.py             │  LearningSession (dump/load JSON)
      │ core/persistence.py         │  build_checkpointer (sqlite|redis|memory)
      │ db/chroma.py               │  ChromaStore (RAG)
      └─────────────────────────────┘
```

## Слои
1. **Automaton** — `src/nereus/core/factory.py:build_nereus_graph()` собирает
   LangGraph‑композицию и фиксирует allowlist сериализуемых объектов.
2. **LLM runtime** — `llm/prompts.py` (реестр ролей), `llm/schema.py`
   (Pydantic‑контракты), `llm/params.py` (per‑role), `llm/inference.py`
   (клиент с ретраями).
3. **Память** — `core/session.py:LearningSession` (dump/load на
   `.sessions/{thread_id}.json`), `core/context.py` truncate/summarize.
4. **Чекпоинты** — `core/persistence.py` (sqlite/redis/memory), `checkpoint_*` env.
5. **RAG (Step 4)** — `db/chroma.py:ChromaStore` + `retriever`‑узел в графе
   (`_retrieve_chunks`), `llm/embed.py` (stub/sentence-transformers/ollama).
6. **UI (Step 5)** — `src/nereus/ui/app.py` (Chainlit Web UI, `:7457`).

## Конфиг
Все пути/модели/таймауты читаются через Pydantic‑settings
(`nereus/config/settings.py`); переменные можно переопределить через `.env`
(см. [Development](Development.md)).
