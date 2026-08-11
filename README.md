# Nereus

AI-тьютор, работающий по принципу автоматизированного учебного процесса — от Roadmap до **уверенных практических навыков**!

[![Дорожная карта](https://img.shields.io/badge/Roadmap-Project%20Board-6f42c1?style=flat-square&logo=github&logoColor=white)](https://github.com/orgs/nereuslabs/projects/1)

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
- **абстракция LLM‑провайдера** (`LLMProvider`: `OpenRouterProvider` + `StubLLMProvider`) с фабрикой `build_nereus_graph()`; агенты генерируют roadmap/материалы/оценки через модель, офлайн‑stub — детерминированно без сети; при недоступности LLM сервис сообщает «Сервис временно недоступен» (#44/#45);
- **сло́й промптов и схем**: реестр ролей/промптов (`llm/prompts.py`), Pydantic‑контракты ответов (`llm/schema.py`), параметры модели (`llm/params.py`), inference‑клиент с ретраями (`llm/inference.py`);
- **RAG‑сло́й**: протоколы `Embedder` и `Retriever` (`llm/embed.py`, `llm/retriever.py`), `ChromaStore` (`db/chroma.py`), retrieval, проинтегрированный в узлы `tutor_*` (`core/graph.py`);
- **память сессии** (`core/session.py` `LearningSession`) с агрегацией слабых мест, `session_brief`‑преамбулой, `dump`/`load`, `messages` и `trim_context`;
- **чекпоинтер** (`core/persistence.py:build_checkpointer` — sqlite/redis/memory) для cross‑restart resume в CLI и Chainlit;
- авто- и опциональные live‑тесты; CI (ruff + pytest); Docker + Docker Compose.

## Стек

- **Python 3.11+**, **LangGraph** — оркестрация агентной цепочки
- **OpenRouter** (`https://openrouter.ai/api/v1`, chat + embeddings) + **ChromaDB** —
  агрегатор LLM/embeddings (локальная модель через `openrouter/free`/cloud); ChromaDB —
  векторное хранилище и RAG (интегрировано в Шаг 4);
- **Chainlit** — Web UI (`src/nereus/ui/app.py`, Шаг 5 ✅);
- **Docker + Docker Compose** — развёртывание

## Конфигурация LLM

По умолчанию используется `LLM_PROVIDER=stub` (без сети). Для реальной модели в `.env`:

```bash
# OpenRouter (рекомендуется): агрегатор со свободными и платными моделями.
LLM_PROVIDER=openrouter
OPENROUTER_API_KEY=<your-key-from-openrouter.ai>
OPENROUTER_MODEL=openrouter/free   # или конкретная платная модель, e.g. "anthropic/claude-3.5-sonnet"
```

> **Примечание про `openrouter/free`.** Свободные модели выбираются роутером в момент
> запроса; реальная модель возвращается в `response.model` и кэшируется в `last_model`.
> Лимиты бесплатных моделей: ~20 RPM / ~50 запросов в сутни без кредитов
> (или ~1000 RPD при наличии кредитов). При 401/402/403 API‑сообщает об ошибке, и
> агенты сообщают «Сервис временно недоступен», не падая (#44/#45).

> **Без ключа?** Если `LLM_PROVIDER=openrouter`, но `OPENROUTER_API_KEY` пустой
> (например, в Docker без env‑переменной), сервис стартует в офлайн‑режиме `stub`
> с предупреждением в логах — UI и CLI не падают. Чтобы включить OpenRouter, задайте
> ключ: `OPENROUTER_API_KEY=<key> docker compose up -d --build ui`.

Дополнительные параметры (опционально, fallback на defaults в `llm/params.py`):
```bash
CONTEXT_MAX_TOKENS=8000     # бюджет окна истории сообщений
```

Параметры RAG (по умолчанию `EMBEDDING_PROVIDER=stub` — без сети, используются fake‑векторы для офлайн‑демо):
```bash
EMBEDDING_PROVIDER=stub                       # "stub" | "sentence_transformers" | "openrouter"
SENTENCE_TRANSFORMERS_MODEL=sentence-transformers/all-MiniLM-L6-v2
OPENROUTER_EMBED_MODEL=openai/text-embedding-3-small  # только для EMBEDDING_PROVIDER=openrouter
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

# OpenRouter (рекомендуется) — trace в artifacts/run.jsonl
LLM_PROVIDER=openrouter OPENROUTER_API_KEY=<key> OPENROUTER_MODEL=openrouter/free \
  python -m nereus.scripts.eval_chain --skill "Python" --submission "this is good"
```

### Живые тесты против реального LLM

Тест `tests/integration/test_live_openrouter.py` выполняется
**только** при включённом флаге, чтобы CI оставалась офлайн‑детерминированной:

```bash
# OpenRouter
NEREUS_RUN_LIVE=1 LLM_PROVIDER=openrouter OPENROUTER_API_KEY=<key> \
  OPENROUTER_MODEL=openrouter/free pytest -m "not skip" tests/integration/test_live_openrouter.py
```

## Быстрый старт

```bash
# Локальный запуск: CLI‑прототип (human-in-the-loop через input; офлайн по умолчанию)
python -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"

# старт (первичный запуск — сохраняет сессию в SQLite)
python main.py

# возобновить прерванную/сохранённую сессию по thread_id
python main.py --resume nereus-demo

# Web UI на базе Chainlit (по умолчанию LLM_PROVIDER=stub — офлайн)
chainlit run src/nereus/ui/app.py

# OpenRouter (рекомендуется): агрегатор, ничего локально не тащить
LLM_PROVIDER=openrouter OPENROUTER_API_KEY=<key> python main.py

# Через Docker (сервис nereus-ui на http://localhost:7457)
docker compose up -d --build ui
docker compose --profile ragger run --rm ingest   # загрузить материалы в ChromaDB
```

### Web UI (Chainlit)

`src/nereus/ui/app.py` — диалоговый web‑клиент: собирает профиль, стримит roadmap →
material → assessment, задаёт вопросы экзаменатора и возобновляет граф через
`Command(resume=...)`. Поддерживает офлайн‑режим (`LLM_PROVIDER=stub`) и реальную
модель через OpenRouter. Состояние сессии (material, retrieved_chunks, assessment) рендерится
по‑шагово; `thread_id` хранится в `cl.user_session` для cross‑restart resume
(требует persistent checkpointer — см. issue #16).

```bash
# офлайн‑демо (stub embeddings)
chainlit run src/nereus/ui/app.py

# OpenRouter (рекомендуется): чат + эмбеддинги через агрегатор
LLM_PROVIDER=openrouter OPENROUTER_API_KEY=<key> \
  EMBEDDING_PROVIDER=openrouter \
  chainlit run src/nereus/ui/app.py
```

### Multi-user sessions (Issue #8/#57)

Nereus поддерживает несколько независимых пользовательских сессий в одном
deployment. Каждая сессия сохраняет профиль, roadmap, прогресс и диагностическое
состояние на диск и может быть восстановлена между перезапусками.

```bash
# Создать новую сессию (генерируется session_id)
python main.py --new-session --user-id <uuid>

# Возобновить существующую сессию по session_id
python main.py --session-id <uuid> --user-id <uuid>
```

- **`--new-session`** — генерирует новый `session_id` (UUID4) и запускает чат с чистого листа.
- **`--session-id <id>`** — загружает ранее сохранённую сессию из `SESSION_ROOT/{user_id}/{session_id}.json`.
- **`--user-id <id>`** — идентификатор пользователя для sharding файлов сессий.
- **`--resume <thread_id>`** — восстанавливает LangGraph checkpoints (SQLite checkpointer).

Файлы сессий хранятся в `SESSION_ROOT` (по умолчанию `.sessions/`), что обеспечивает
изоляцию между пользователями даже при совпадении `session_id`.

Профили пользователей (UserProfile) сохраняются через `UserStore` в SQLite
(`.users/users.sqlite3` по умолчанию), в Redis (при `USER_STORAGE=redis`) либо
в памяти при недоступности БД. Redis‑бекенд деградирует в память, если Redis
недоступен — сервис не падает.

```env
# .env — persistence & multi-user (Issue #8/#57)
USER_STORAGE=sqlite        # sqlite | redis | memory
USER_DB_PATH=.users/users.sqlite3
# redis (used when USER_STORAGE=redis)
REDIS_HOST=redis
REDIS_PORT=6379

# session snapshots (Issue #57)
SESSION_ROOT=.sessions      # {root}/{user_id}/{session_id}.json
```

### Диагностика навыков (Issue #7)

Перед генерацией roadmap бот может пройти пользователя кратким диагностическим
квизом (3–5 вопросов), оценить ответы и построить **адаптивную** дорожную карту,
ориентированную на слабые зоны.

```bash
# CLI: включить диагностический этап
python main.py --new-session --user-id <uuid> --diagnostic

# Harness
LLM_PROVIDER=stub python -m nereus.scripts.eval_chain --dry-run --diagnostic --skill "Python"
```

```env
# .env — diagnostic (Issue #7)
RUN_DIAGNOSTIC=true          # запускать квиз до roadmap (default: false)
DIAGNOSTIC_QUESTION_COUNT=5  # число вопросов (default: 5)
```

В офлайн‑тестах диагностика выключена принудительно (autouse‑фикстура
`_force_stub_offline` в `tests/conftest.py`), чтобы `RUN_DIAGNOSTIC=true` из
`.env` не ломал регрессионные тесты экзаменатора; специализированные тесты
включают её через `build_nereus_graph(run_diagnostic=True)`.

### Локальный RAG (demo)

RAG‑хранилище (ChromaDB) наполняется из `materials/` скриптом
`scripts/ingest_materials.py`. По умолчанию используются `EMBEDDING_PROVIDER=stub`
(офлайн, fake‑векторы), но retrieval работает структурно так же, как с
реальными эмбеддингами.

```bash
# 1. поднять ChromaDB (и опционально UI через OpenRouter)
docker compose up -d chromadb

# 2. загрузить материалы (offline, stub embeddings)
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

#### Cloud: чат + эмбеддинги через OpenRouter

Чат и эмбеддинги оба идут через один `OPENROUTER_API_KEY`; `openrouter/free`
покрывает лимиты для демо‑трафика (платные модели — по‑требности). Ранее чат
шёл через Ollama Cloud, а эмбеддинги — через локальный `ollama serve`
(«гибрид», чтобы не платить за эмбеддинги в Cloud); после миграции на OpenRouter
этот разворот стал избыточным и удалён (#46).

```bash
# OpenRouter — один токен для чата и эмбеддингов
export LLM_PROVIDER=openrouter
export OPENROUTER_API_KEY=<key>
export OPENROUTER_MODEL=openrouter/free
export EMBEDDING_PROVIDER=openrouter
export OPENROUTER_EMBED_MODEL=openai/text-embedding-3-small
```

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
├── agents/                # Coach / Tutor / Examiner (structured inference; offline‑stub fallback)
├── llm/
│   ├── base.py            # LLMProvider (абстракция)
│   ├── stub.py            # in-memory LLM‑провайдер (тесты/без сети)
│   ├── schema.py          # Pydantic‑контракты ответов + parse_structured
│   ├── params.py          # ModelParams + per-role таблица
│   ├── prompts.py         # реестр system‑prompts + билдеры с session_brief
│   ├── inference.py       # StructuredInferenceClient (ретраи + LLMUnavailableError)
│   ├── factory.py         # build_llm_provider (stub | openrouter)
│   ├── openrouter.py      # OpenRouter chat LLM provider + OpenRouterError
│   ├── embed.py           # Embedder (stub | sentence_transformers | openrouter)
│   └── retriever.py       # Retriever (stub | ChromaRetriever) + RetrievedChunk
└── ui/app.py              # Chainlit Web UI driver (Step 5)
```
