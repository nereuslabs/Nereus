# Архитектура

```
main.py  ──►  core.factory.build_nereus_graph()
                  │
            ┌─────└─────┐
            │  State    │  (pydantic, msgpack-serializable)
            └─────┬─────┘
                  │
     ┌────────────┼────────────┐
     │ Coach      │ Tutor      │ Examiner      │  (agents/llm/*.py)
     │ prompts[]  │ params     │ schema.py     │
     └────────────┼────────────┘
                  ▼
            llm.inference        (StructuredInferenceClient, retry x2)
                     │
        LLM_PROVIDER = stub | ollama | openai
                  ▼
            core/session.py   (LearningSession, truncate/summarize)
```

## Слои
1. **Automaton** — `src/nereus/core/factory.py:build_nereus_graph()` собирает
   LangGraph‑композицию и фиксирует allowlist сериализуемых объектов.
2. **LLM runtime** — `llm/prompts.py` (реестр ролей), `llm/schema.py`
   (Pydantic‑контракты), `llm/params.py` (per‑role), `llm/inference.py`
   (клиент с ретраями).
3. **Память** — `core/session.py:LearningSession`, `core/context.py`
   truncate/summarize.
4. **RAG (Step 4, в планах)** — `db/chroma.py:ChromaStore` (скаффолд) +
   `retriever`‑узел в графе.

## Конфиг
Все пути/модели/таймауты читаются через Pydantic‑settings (`nereus/settings.py`);
переменные можно переопределить через `.env` (см. [Development](Development.md)).
