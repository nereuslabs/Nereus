# Nereus

AI-тьютор, работающий по принципу автоматизированного учебного процесса — от Roadmap до **уверенных практических навыков**!

## Архитектура

Nereus построен как зацикленный автомат из трёх агентов, оркеструемых через **LangGraph** (human-in-the-loop):

1. **Агент-коуч** — собирает профиль пользователя (скилл, уровень, сроки) и строит **Roadmap**.
2. **Агент-тьютор** — выдаёт учебные материалы и задания по текущей теме Roadmap, углубляет слабые места.
3. **Агент-экзаменатор** — проверяет ответы, ставит оценку и вердикт `PASS` / `RETRY`, после чего автомат решает: двигаться дальше или повторить материал.

## Статус

**Стадия:** прототипирование MVP (Шаг 1 ✅, Шаг 2 🟢 — слит в `develop`, Шаг 3 🚧 in progress).

Ветвление следует GitHub Flow с веткой `develop` как integration line: feature‑ветки → Pull Request → merge в `develop`. Текущая работа ведётся в `feature/step-3-llm-runtime`.

На текущем этапе реализовано:
- полный циклический граф LangGraph с условным роутингом (`PASS`/`RETRY`/`END`);
- human-in-the-loop (интерактивный режим через `interrupt`);
- **абстракция LLM‑провайдера** (`LLMProvider`: `OllamaProvider` + `StubLLMProvider`) с централизованной фабрикой `build_nereus_graph()`; агенты генерируют Roadmap/материалы/оценки через модель, с fallback на детерминированные заглушки без сети;
- **сло́й промптов и схем**: реестр ролей/промптов (`llm/prompts.py`), Pydantic‑контракты ответов (`llm/schema.py`), централизованные параметры модели (`llm/params.py`), inference‑клиент с ретраями и валидацией (`llm/inference.py`);
- **память сессии** (`core/session.py` `LearningSession`) с агрегацией слабых мест, `session_brief`‑преамбулой в каждом промпте, наполнением `messages` и управлением окном контекста (`core/context.py`);
- автотесты (unit + integration + опциональные live‑тесты); CI (ruff + pytest);
- контейнеризация (Docker + Docker Compose).

## Стек

- **Python 3.11+**, **LangGraph** — оркестрация агентной цепочки
- **Ollama** (`/api/chat`, local / Cloud Free Tier) + **ChromaDB** (векторная БД и RAG, в перспективе)
- **Chainlit** — UI (в перспективе)
- **Docker + Docker Compose** — развёртывание

## Конфигурация LLM

По умолчанию используется `LLM_PROVIDER=stub` (без сети). Для реальной модели в `.env`:

```bash
LLM_PROVIDER=ollama
OLLAMA_BASE_URL=https://<your-ollama-host>
OLLAMA_MODEL=gemma4:31b-cloud
OLLAMA_API_KEY=...
```

Дополнительные параметры (опционально, fallback на defaults в `llm/params.py`):
```bash
OLLAMA_TEMPERATURE=0.2      # переопределение temperature
OLLAMA_MAX_TOKENS=4096      # переопределение max_tokens
CONTEXT_MAX_TOKENS=8000     # бюджет окна истории сообщений
```

После чего `python main.py` будет генерировать Roadmap, материалы и оценки через модель.

### Оценка цепочки (harness)

CLI‑харнес `scripts/eval_chain.py` прогоняет full‑pipeline end‑to‑end и пишет JSONL‑трассу (roadmap, final assessment, `session_brief`, журнал LLM‑вызовов с latency/retrofit):

```bash
# stub (offline) dry-run на экран
LLM_PROVIDER=stub python -m nereus.scripts.eval_chain --dry-run --skill "Python"

# реальная модель — trace в artifacts/run.jsonl
LLM_PROVIDER=ollama OLLAMA_BASE_URL=http://localhost:11434 \
  OLLAMA_MODEL=gemma4:31b-cloud \
  python -m nereus.scripts.eval_chain --skill "Python" --submission "this is good"
```

### Живые тесты против Ollama

Тест `tests/integration/test_live_ollama.py` выполняется **только** при включённом флаге, чтобы CI оставалась офлайн‑детерминированной:

```bash
NEREUS_RUN_LIVE=1 LLM_PROVIDER=ollama OLLAMA_MODEL=gemma4:31b-cloud pytest -m "not skip" tests/integration/test_live_ollama.py
```

## Быстрый старт

```bash
# Локальный запуск прототипа
python -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"
python main.py

# Через Docker
docker compose up -d --build
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
│   └── settings.py        # pydantic-settings (.env), CONTEXT_MAX_TOKENS
├── core/
│   ├── state.py           # NereusState TypedDict + домены (UserProfile, Roadmap, Assessment)
│   ├── router.py          # условные переходы автомата (route_after_exam)
│   ├── graph.py           # сборка StateGraph + trim_context (bound message window)
│   ├── factory.py         # build_nereus_graph — централизованная сборка + наблюдаемость
│   ├── session.py         # LearningSession (агрегация прогресса/слабых мест)
│   └── context.py         # truncate_messages / summarize_history (RLHF‑ready)
├── agents/                # Coach / Tutor / Examiner (structured inference + stub fallback)
├── llm/
│   ├── base.py            # LLMProvider (абстракция)
│   ├── ollama.py          # /api/chat клиент (base_url/model/timeout observability)
│   ├── stub.py            # in-memory провайдер (тесты/без сети)
│   ├── schema.py          # Pydantic‑контракты ответов + parse_structured
│   ├── params.py          # ModelParams + per-role таблица + env overrides
│   ├── prompts.py         # реестр system‑prompts + билдеры с session_brief
│   ├── inference.py       # StructuredInferenceClient (ретраи + LLMOutputError)
│   └── factory.py         # build_llm_provider (stub | ollama)
└── ui/app.py              # Chainlit (placeholder)
```
