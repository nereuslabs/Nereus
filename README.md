# Nereus

AI‑tutor по принципу автоматизированного учебного процесса — от **Roadmap** до
**уверенных практических навыков**: собирает профиль, строит дорожную карту,
выдаёт материалы и задания, проверяет ответы и адаптирует план под слабые зоны.
Всё через диалог в терминале или в Web UI (Chainlit).

[![CI](https://github.com/nereuslabs/Nereus/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/nereuslabs/Nereus/actions/workflows/ci.yml?query=branch%3Adevelop)
[![Release](https://img.shields.io/github/v/release/nereuslabs/Nereus?sort=semver&color=blue)](https://github.com/nereuslabs/Nereus/releases)
[![PRs](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/nereuslabs/Nereus/blob/main/CONTRIBUTING.md)
[![Roadmap](https://img.shields.io/badge/Roadmap-Project%20Board-6f42c1?style=flat-square&logo=github&logoColor=white)](https://github.com/orgs/nereuslabs/projects/1)
[![License](https://img.shields.io/github/license/nereuslabs/Nereus)](LICENSE)

---

## Возможности

- **Агентный автомат на LangGraph** — Coach → Tutor → Examiner с условным роутингом
  (`PASS`/`RETRY`/`END`) и human‑in‑the‑loop через `interrupt`.
- **Адаптивная диагностика** (Issue #7) — короткий квиз перед roadmap‑ом,
  `WeaknessReport` → дорожная карта, упорядоченная по слабым зонам.
- **Мульти‑пользовательские сессии** (Issue #8/#57) — профили + snapshot‑ы
  сессий на диск (`SESSION_ROOT`), хранение профилилей в SQLite / Redis / memory.
- **RAG** — ChromaDB‑хранилище материалов, retrieval‑augmented экзаменатор.
- **LLM‑абстракция** — `stub` (офлайн, CI‑безопасно) и `openrouter` (chat + embeddings).
- **Web UI** на Chainlit + **CLI** (`main.py`) с `--resume`/`--session-id`.
- **CI**: `ruff check` + `ruff format --check` + `pytest` (174 passed, 2 skipped).
- **Docker Compose** — `app`, `ui`, `chromadb`, `redis`.

---

## Быстрый старт

```bash
git clone https://github.com/nereuslabs/Nereus.git
cd Nereus
python -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"

# Офлайн‑демо (LLM_PROVIDER=stub) — ничего дополнительно не нужно
python main.py

# Web UI на http://localhost:7457
chainlit run src/nereus/ui/app.py
```

Новый запуск → `--new-session`, возобновление → `--resume <thread_id>` или
`--session-id <id> --user-id <uuid>` (см. [Multi-user sessions](#multi-user-sessions)).

---

## Конфигурация (`.env`)

Все параметры читаются через Pydantic‑settings (`src/nereus/config/settings.py`).
Скопируйте шаблон:

```bash
cp .env.example .env
```

| Группа | Переменная | По умолчанию | Описание |
|--------|-----------|--------------|----------|
| LLM | `LLM_PROVIDER` | `stub` | `stub` \| `openrouter` |
| LLM | `OPENROUTER_API_KEY` | — | ключ OpenRouter (опционален) |
| LLM | `OPENROUTER_MODEL` | `openrouter/free` | модель чата |
| LLM | `OPENROUTER_TIMEOUT` | `60` | таймаут запроса, с |
| Embeddings | `EMBEDDING_PROVIDER` | `stub` | `stub` \| `sentence_transformers` \| `openrouter` |
| RAG | `CHROMADB_HOST` | `localhost` | хост ChromaDB |
| RAG | `CHROMADB_PORT` | `8000` | порт ChromaDB |
| Session | `CHECKPOINT_BACKEND` | `memory` | `memory` \| `sqlite` \| `redis` |
| Session | `CHECKPOINT_DB` | `.checkpoints/nereus.sqlite3` | путь к SQLite‑чекпоинтеру |
| Session | `SESSION_ROOT` | `.sessions` | `{root}/{user_id}/{session_id}.json` |
| Multi-user | `USER_STORAGE` | `sqlite` | `sqlite` \| `redis` \| `memory` |
| Multi-user | `USER_DB_PATH` | `.users/users.sqlite3` | SQLite БД профилей |
| Multi-user | `REDIS_HOST` | `localhost` | Redis (для `USER_STORAGE=redis`/`CHECKPOINT_BACKEND=redis`) |
| Multi-user | `REDIS_PORT` | `6379` | порт Redis |
| Diagnostic | `RUN_DIAGNOSTIC` | `false` | запускать квиз до roadmap |
| Diagnostic | `DIAGNOSTIC_QUESTION_COUNT` | `5` | число вопросов |
| Tests | `NEREUS_RUN_LIVE` | `0` | `=1` включает live‑интеграционные тесты |

---

## Архитектура

```
main.py ──► src/nereus/core/graph.py  (LangGraph StateGraph)
                 │
            ┌────┴────┐
            │ NereusState │  (pydantic TypedDict)
            └────┬────┘
        ┌────────┼────────┐
        │ Coach  │ Tutor  │ Examiner │   agents/* + llm/*
        │ (roadmap)│ (materials)│ (assessment)│
        └────────┼────────┘
                 ▼
        llm/inference.py  (StructuredInferenceClient, retry x2)
                 │
   LLM_PROVIDER = stub | openrouter
```

Слои: **Automaton** (`core/graph.py`, `core/factory.py`), **LLM runtime**
(`llm/`), **память** (`core/session.py`, `core/context.py`), **RAG**
(`db/chroma.py`, `llm/retriever.py`), **хранилище пользователей**
(`core/user_store.py`), **UI** (`ui/app.py`).

---

## Использование

### CLI

```bash
# Новый автотест (LLM_PROVIDER=stub)
python main.py --new-session --user-id <uuid>

# С диагностикой навыка
python main.py --new-session --user-id <uuid> --diagnostic

# Возобновить сессию
python main.py --session-id <uuid> --user-id <uuid>
python main.py --resume <thread_id>            # checkpoints (CHECKPOINT_BACKEND=sqlite)

# Harness: end-to-end trace в artifacts/run.jsonl
LLM_PROVIDER=stub python -m nereus.scripts.eval_chain --dry-run --diagnostic --skill "Python"
```

### Web UI (Chainlit)

```bash
chainlit run src/nereus/ui/app.py
# OpenRouter + embeddings
LLM_PROVIDER=openrouter OPENROUTER_API_KEY=<key> EMBEDDING_PROVIDER=openrouter \
  chainlit run src/nereus/ui/app.py
```

### Multi-user sessions (Issue #8/#57)

Каждая сессия сохраняет профиль, roadmap и прогресс в `SESSION_ROOT` и может
восстанавливаться между перезапусками:

```bash
python main.py --new-session --user-id <uuid>                 # создать
python main.py --session-id <uuid> --user-id <uuid>           # возобновить
```

`UserStore` сохраняет профили в SQLite (по умолчанию), Redis (`USER_STORAGE=redis`)
либо в памяти — Redis‑бекенд деградирует в память при недоступности, сервис не падает.

### Диагностика навыков (Issue #7)

Перед генерацией roadmap бот проходит краткий квиз (3–5 вопросов), оценивает
ответы и строит адаптивную дорожную карту по слабым зонам. Включается через
`--diagnostic` / `RUN_DIAGNOSTIC=true`. В офлайн‑тестах диагностика выключена
autouse‑фикстурой `_force_stub_offline` в `tests/conftest.py`.

### Локальный RAG (demo)

```bash
docker compose up -d chromadb
python scripts/ingest_materials.py --materials materials --clear
LLM_PROVIDER=stub python main.py
```

---

## Тесты и качество

```bash
ruff check .
ruff format --check .
pytest                      # 174 passed, 2 skipped (live gated by NEREUS_RUN_LIVE=1)
NEREUS_RUN_LIVE=1 pytest     # + live‑интеграционные с OpenRouter
```

CI (`.github/workflows/ci.yml`) запускает всё выше на push/PR в `develop` и `main`.

---

## Docker

```bash
docker compose up -d --build ui                 # Web UI http://localhost:7457
docker compose --profile ragger run --rm ingest  # загрузить материалы в ChromaDB
docker compose --profile cli run --rm app --new-session --user-id <uuid>  # CLI (interactive)
```

`docker-compose.yml` поднимает `app`, `ui`, `ingest`, `chromadb`, `redis`.

---

## Структура

```text
src/nereus/
├── config/settings.py     # pydantic-settings (.env): LLM/RAG/session параметры
├── core/graph.py          # StateGraph + trim_context, диагностический entry
├── core/factory.py        # build_nereus_graph — централизованная сборка
├── core/persistence.py    # checkpointer (memory/sqlite/redis)
├── core/session.py        # LearningSession (агрегация прогресса/слабых мест)
├── core/user_store.py     # UserStore (sqlite) + UserStoreRedis + factory
├── core/router.py         # условные переходы автомата
├── core/context.py        # truncate/summarize
├── db/chroma.py           # ChromaStore (upsert/search по темам)
├── agents/                # Coach / Tutor / Examiner / Diagnostic
├── llm/                   # провайдеры, схемы, промпты, inference, embeddings
└── ui/app.py              # Chainlit Web UI
tests/                     # unit/ + integration/ (live gated by NEREUS_RUN_LIVE=1)
scripts/                   # eval_chain.py, ingest_materials.py
```

---

## Ветвление и релизы

GitHub Flow с интеграционной веткой `develop`:

- feature‑ветки → Pull Request **на `develop`** → squash/merge (CI зелёный).
- `develop` — защищена (strict status checks, linear history, enforce admins).
- `main` — релизная ворота, управляется репо‑rulesетом (review + статус‑чек);
  релисы back‑portятся из `develop` через PR с `--admin`‑слиянием (см. `CLAUDE.md`).
- Теги `vX.Y.Z` и GitHub Release наносятся на `main`.

Текущие версии: **v1.1.0** (MVP 1.1) — адаптивная диагностика + мульти‑пользовательские сессии; **MVP 1.0** (Steps 1–6) завершён. Смотрите [доску проекта](https://github.com/orgs/nereuslabs/projects/1) и [milestones](https://github.com/nereuslabs/Nereus/milestones).

---

## Участие

Разворачивайтесь, находите задачу со [меткой `good first issue`](https://github.com/nereuslabs/Nereus/issues)
или `help wanted` — см. [CONTRIBUTING.md](CONTRIBUTING.md). Комментарии в коде — на
английском, issues/PR — на русском. Спасибо, что помогаете **Nereus**! 🚀
