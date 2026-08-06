# Nereus

AI-тьютор, работающий по принципу автоматизированного учебного процесса — от Roadmap до **уверенных практических навыков**!

## Архитектура

Nereus построен как зацикленный автомат из трёх агентов, оркеструемых через **LangGraph** (human-in-the-loop):

1. **Агент-коуч** — собирает профиль пользователя (скилл, уровень, сроки) и строит **Roadmap**.
2. **Агент-тьютор** — выдаёт учебные материалы и задания по текущей теме Roadmap, углубляет слабые места.
3. **Агент-экзаменатор** — проверяет ответы, ставит оценку и вердикт `PASS` / `RETRY`, после чего автомат решает: двигаться дальше или повторить материал.

## Статус

**Стадия:** активная разработка MVP. Репо живёт в организации `nereuslabs/Nereus`.

Схема шагов (см. Milestones):
- Шаг 1 ✅ — базовый автомат‑агент (LangGraph, LLM runtime, реестр промптов, память сессии). Слит в `develop`.
- Шаг 2 ✅ — абстракция LLM‑провайдера (`Ollama` + `stub`). Слит в `develop`.
- Шаг 3 ✅ — inference‑клиент с ретраями, схемы/промпты, окно контекста, CLI‑харнес (`scripts/eval_chain.py`). Слит в `develop`.
- Шаг 4 ✅ — RAG‑конвейер: эмбеддинги (`llm/embed.py`), ChromaDB‑хранилище (`db/chroma.py`) + `llm/retriever.py`, retrieval, проинтегрированный в узлы `tutor_*` графа. Слит в `develop`.
- Шаг 5 ✅ — Chainlit Web‑UI (`src/nereus/ui/app.py`) + runtime‑провязка сессии (#23). Слит в `develop`.
- Шаг 6 ✅ — Персистентная сессия: `LearningSession.dump/load` в runtime‑цикле (`core/graph.py`) + `core/persistence.py` (sqlite/redis/memory checkpointer). #6, #16, #22.

Ветвление — GitHub Flow с integration‑веткой `develop`: feature‑ветки → Pull Request (base `develop`) → merge. Текущая работа — в зависимости от задачи (см. Issues/Milestones).

Реализовано на текущем этапе:
- полный циклический граф LangGraph с условным роутингом (`PASS`/`RETRY`/`END`);
- human-in-the-loop через `interrupt` (интерактивный режим);
- **абстракция LLM‑провайдера** (`LLMProvider`: `OllamaProvider` + `StubLLMProvider`) с фабрикой `build_nereus_graph()`; агенты генерируют roadmap/материалы/оценки через модель, с fallback на детерминированные заглушки без сети;
- **сло́й промптов и схем**: реестр ролей/промптов (`llm/prompts.py`), Pydantic‑контракты ответов (`llm/schema.py`), параметры модели (`llm/params.py`), inference‑клиент с ретраями (`llm/inference.py`);
- **RAG‑сло́й**: протоколы `Embedder` и `Retriever` (`llm/embed.py`, `llm/retriever.py`), `ChromaStore` (`db/chroma.py`), retrieval, проинтегрированный в узлы `tutor_*` (`core/graph.py`);
- **память сессии** (`core/session.py` `LearningSession`) с агрегацией слабых мест, `session_brief`‑преамбулой, `dump`/`load`, `messages` и `trim_context`;
- **чекпоинтер** (`core/persistence.py:build_checkpointer` — sqlite/redis/memory) для cross‑restart resume в CLI и Chainlit;
- авто- и опциональные live‑тесты; CI (ruff + pytest); Docker + Docker Compose.

## Стек

- **Python 3.11+**, **LangGraph** — оркестрация агентной цепочки
- **Ollama** (`/api/chat`, локально / Cloud Free Tier) + **ChromaDB** — векторное хранилище и RAG (интегрировано в Шаг 4);
- **Chainlit** — Web UI (`src/nereus/ui/app.py`, Шаг 5 ✅);
- **Docker + Docker Compose** — развёртывание

## Конфигурация LLM

По умолчанию используется `LLM_PROVIDER=stub` (без сети). Для реальной модели в `.env`:

```bash
LLM_PROVIDER=ollama
OLLAMA_BASE_URL=https://<your-ollama-host>
OLLAMA_MODEL=gemma4:31b
OLLAMA_API_KEY=...
```

Дополнительные параметры (опционально, fallback на defaults в `llm/params.py`):
```bash
OLLAMA_TEMPERATURE=0.2      # переопределение temperature
OLLAMA_MAX_TOKENS=4096      # переопределение max_tokens
CONTEXT_MAX_TOKENS=8000     # бюджет окна истории сообщений
```

Параметры RAG (по умолчанию `EMBEDDING_PROVIDER=stub` — без сети, используются fake‑векторы для офлайн‑демо):
```bash
EMBEDDING_PROVIDER=sentence_transformers   # "stub" | "sentence_transformers" | "ollama"
SENTENCE_TRANSFORMERS_MODEL=sentence-transformers/all-MiniLM-L6-v2
OLLAMA_EMBED_MODEL=nomic-embed-text        # только для EMBEDDING_PROVIDER=ollama
RETRIEVER_TOP_K=5
CHROMADB_HOST=localhost
CHROMADB_PORT=8000
```

### Параметры persistence (чекпоинтер)

Граф использует LangGraph checkpointer для сохранения состояния `state` между
паузами (`interrupt`) и перезапусками. По умолчанию — in-memory `MemorySaver`,
чтобы CI оставалась офлайн‑детерминированной; задайте `CHECKPOINTER=sqlite`
(или `redis`) для cross‑restart resume:

```bash
CHECKPOINTER=sqlite
CHECKPOINT_DB=.checkpoints/nereus.sqlite3   # путь к файлу БД
# или
CHECKPOINTER=redis
REDIS_HOST=localhost
REDIS_PORT=6379
```

CLI поддерживает `--resume <thread_id>` для продолжения прерванной сессии
(восстанавливает состояние из checkpointer + `LearningSession` JSON) и
`--session-path <PATH>` для указания пути к файлу сессии (по умолчанию
`.sessions/{thread_id}.json`):

```bash
python main.py --resume nereus-demo
python main.py --session-path /tmp/my-session.json
```

> Сессия и `thread_id` в Web UI сохраняются в пределах браузерной сессии
> (Chainlit `cl.user_session`); cross‑restart resume требует persistent
> checkpointer (`CHECKPOINTER=sqlite` по умолчанию). Полное восстановление
> после `hard refresh` (без `thread_id`) — в беклоге (#23 follow-up).

Для образования см. issue #16 (persistent checkpointer).

После чего `python main.py` будет генерировать Roadmap, материалы и оценки через модель.

### Оценка цепочки (harness)

CLI‑харнес `scripts/eval_chain.py` прогоняет full‑pipeline end‑to‑end и пишет JSONL‑трассу (roadmap, final assessment, `session_brief`, журнал LLM‑вызовов с latency/retrofit):

```bash
# stub (offline) dry-run на экран
LLM_PROVIDER=stub python -m nereus.scripts.eval_chain --dry-run --skill "Python"

# реальная модель — trace в artifacts/run.jsonl
LLM_PROVIDER=ollama OLLAMA_BASE_URL=http://localhost:11434 \
  OLLAMA_MODEL=gemma4:31b \
  python -m nereus.scripts.eval_chain --skill "Python" --submission "this is good"
```

### Живые тесты против Ollama

Тест `tests/integration/test_live_ollama.py` выполняется **только** при включённом флаге, чтобы CI оставалась офлайн‑детерминированной:

```bash
NEREUS_RUN_LIVE=1 LLM_PROVIDER=ollama OLLAMA_MODEL=gemma4:31b pytest -m "not skip" tests/integration/test_live_ollama.py
```

## Быстрый старт

```bash
# Локальный запуск: CLI‑прототип (human-in-the-loop через input)
python -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"

# старт (первичный запуск — сохраняет сессию в SQLite)
python main.py

# возобновить прерванную/сохранённую сессию по thread_id
python main.py --resume nereus-demo

# Web UI на базе Chainlit (по умолчанию LLM_PROVIDER=stub — офлайн)
chainlit run src/nereus/ui/app.py

# Через Docker (сервис nereus-ui на http://localhost:7457)
docker compose up -d --build ui
```

### Web UI (Chainlit)

`src/nereus/ui/app.py` — диалоговый web‑клиент: собирает профиль, стримит roadmap →
material → assessment, задаёт вопросы экзаменатора и возобновляет граф через
`Command(resume=...)`. Поддерживает офлайн‑режим (`LLM_PROVIDER=stub`) и реальную
модель Ollama. Состояние сессии (material, retrieved_chunks, assessment) рендерится
по‑шагово; `thread_id` хранится в `cl.user_session` для cross‑restart resume
(требует persistent checkpointer — см. issue #16).

```bash
# офлайн‑демо (stub embeddings)
chainlit run src/nereus/ui/app.py

# с Ollama + ChromaDB
LLM_PROVIDER=ollama OLLAMA_BASE_URL=http://localhost:11434 \
  EMBEDDING_PROVIDER=sentence_transformers CHROMADB_HOST=localhost \
  chainlit run src/nereus/ui/app.py
```

### Локальный RAG (demo)

RAG‑хранилище (ChromaDB) наполняется из `materials/` скриптом
`scripts/ingest_materials.py`. По умолчанию используются `EMBEDDING_PROVIDER=stub`
(офлайн, fake‑векторы), но retrieval работает структурно так же, как с
реальными эмбеддингами.

```bash
# 1. поднять ChromaDB (опционально: Ollama для LLM/embedded)
docker compose up -d chromadb            # или ollama chromadb

# 2. загрузить материалы (offline, stub)
python scripts/ingest_materials.py --materials materials --clear

# 3. (опционально) пробный прогон без записи в ChromaDB
python scripts/ingest_materials.py --dry-run --materials materials

# 4. запустить автомат (RAG‑retrieval будет подхватывать материалы)
LLM_PROVIDER=stub python main.py
```

Через Docker (профиль `ragger`, чтобы `ingest` не стартовал в `up` по умолчанию):

```bash
docker compose --profile ragger run --rm ingest     # загрузить материалы
docker compose up -d ui                             # Web UI на http://localhost:7457
```

Формат файла материала: `*.md` в `--materials` (`<topic_id>.md`, где `topic_id` —
ведущие цифры имени файла, совпадают с `RoadmapTopic.id`, напр. `1.md` → тема 1).

## Тесты и линтер

```bash
ruff check .
pytest
```

## Структура

```text
src/nereus/
├── config/
│   └── settings.py        # pydantic-settings (.env): LLM/RAG/Chroma параметры
├── core/
│   ├── state.py           # NereusState TypedDict + домены (UserProfile, Roadmap, Assessment)
│   ├── router.py          # условные переходы автомата (route_after_exam)
│   ├── graph.py           # сборка StateGraph + trim_context; retrieval в tutor_*
│   ├── factory.py         # build_nereus_graph — централизованная сборка + наблюдаемость
│   ├── persistence.py     # build_checkpointer (memory/sqlite/redis), msgpack allowlist
│   ├── session.py         # LearningSession (агрегация прогресса/слабых мест, dump/load)
│   └── context.py         # truncate_messages / summarize_history (RLHF‑ready)
│   └── db/
│       └── chroma.py      # ChromaStore (upsert/search по темам)
├── agents/                # Coach / Tutor / Examiner (структурный inference + stub fallback)
├── llm/
│   ├── base.py            # LLMProvider (абстракция)
│   ├── ollama.py          # /api/chat клиент (base_url/model/timeout observability)
│   ├── stub.py            # in-memory LLM‑провайдер (тесты/без сети)
│   ├── schema.py          # Pydantic‑контракты ответов + parse_structured
│   ├── params.py          # ModelParams + per-role таблица + env overrides
│   ├── prompts.py         # реестр system‑prompts + билдеры с session_brief
│   ├── inference.py       # StructuredInferenceClient (ретраи + LLMOutputError)
│   ├── factory.py         # build_llm_provider (stub | ollama)
│   ├── embed.py           # Embedder (stub | sentence_transformers | ollama)
│   └── retriever.py       # Retriever (stub | ChromaRetriever) + RetrievedChunk
└── ui/app.py              # Chainlit Web UI driver (Step 5)
```
