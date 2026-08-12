# Архитектура

```
main.py ──► src/nereus/core/graph.py   (LangGraph StateGraph)
                 │
            ┌────┴────┐
            │ NereusState │   (pydantic TypedDict)
            └────┬────┘
        ┌────────┼────────┐
        │ Coach  │ Tutor  │ Examiner │   (agents/* + llu/*.py)
        │ (roadmap + diag)│ (materials)│ (assessment)│
        └────────┼────────┘
                 ▼
        llm/inference.py   (StructuredInferenceClient, retry x2)
                 │
        LLM_PROVIDER = stub | openrouter
```

> `LLM_PROVIDER=stub` — in‑memory, без сети (тесты / CI / демо).
> `LLM_PROVIDER=openrouter` — chat + embeddings через OpenRouter.

## Слои

1. **Automaton** — `src/nereus/core/graph.py` собирает `StateGraph` и фиксирует
   allowlist сериализуемых объектов; `core/factory.py:build_nereus_graph()` —
   централизованная сборка.
2. **LLM runtime** — `llm/prompts.py` (реестр ролей), `llm/schema.py`
   (Pydantic‑контракты ответов), `llm/params.py` (per‑role), `llm/inference.py`
   (клиент с ретраями, `LLMUnavailableError`).
3. **Память** — `core/session.py:LearningSession` (агрегация прогресса и слабых
   мест, `dump`/`load`, `trim_context`), `core/context.py` truncate/summarize.
4. **RAG** — `db/chroma.py:ChromaStore` (upsert/search по темам),
   `llm/retriever.py`, retrieval‑узел в графе.
5. **Хранилище пользователей** — `core/user_store.py`: `UserStore` (SQLite + memory)
   и `UserStoreRedis` (Redis hash, graceful fallback на memory); фабрика
   `build_user_store()` диспатчит на `USER_STORAGE`.
6. **Чекпоинтер** — `core/persistence.py:build_checkpointer` (`memory`/`sqlite`/`redis`)
   для cross‑restart resume в CLI и Chainlit.
7. **UI** — `src/nereus/ui/app.py` (Chainlit).

## Конфиг

Все пути/модели/таймауты читаются через Pydantic‑settings
(`src/nereus/config/settings.py`); переменные можно переопределять через `.env`
(см. `.env.example` и [Development](Development.md)).
