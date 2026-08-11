# Nereus — Стратегические векторы развития (пост-MVP 1.0 GA)

> Основано на аудите кодовой базы, GitHub Issues/Milestones, project board и анализе архитектурных возможностей.

---

## 0. Текущий статус (baseline)

- ✅ **MVP 1.0** — завершён (Steps 1–6 + RAG + UI + persistent session + OpenRouter migration)
- ✅ 80 unit/integration тестов, 2 live (gated)
- ✅ CI: ruff + pytest на push/PR к `main`/`develop`
- ✅ Project board automation через GraphQL
- ⚠️ 19 файлов не отформатированы (`ruff format --check`)

---

## Вектор 1: Автоматическая диагностика навыков (Issue #7)

**Цель:** Заменить статичный stub roadmap (3 темы: fundamentals → practice → advanced) на адаптивный план, основанный на диагностическом тесте.

### Что есть сейчас
- `StubLLMProvider` + `CoachAgent.build_roadmap()` → жёстко закодированные 3 темы
- `LearningSession.aggregated_weak_areas` — уже агрегирует слабые зоны
- `Roadmap.topics` — список `RoadmapTopic(id, title, description)`

### Что нужно построить
1. **Diagnostic Agent** (новый агент или расширение CoachAgent):
   - Генерирует 3–5 вопросов по базовым концепциям выбранного скилла
   - Оценивает ответы → выявляет пробелы → формирует персонализированный roadmap
2. **Adaptive Roadmapper**:
   - `Roadmap` расширяется: `prerequisites: dict[str, list[str]]`, `estimated_hours: float`
   - Темы с weight/difficulty, чтобы роутер мог пропускать освоенные темы
3. **Интеграция в граф**: `START → diagnostic → coach (adaptive roadmap) → tutor_new → examiner → router`

### Архитектурная перегрузка
```
# core/state.py
class RoadmapTopic(BaseModel):
    id: str
    title: str
    description: str
    difficulty: float = 1.0          # 0.0–1.0 относительной сложности
    prerequisites: list[str] = []    # topic_ids, which must be mastered
    estimated_hours: float = 1.0    # часы на освоение
```

### Тестируемость
- Stub-режим: детерминированный diagnostic (fixture-based)
- LLM-режим: `FakeLLMProvider` с discriminator на "diagnostic" в prompt
- E2E: `test_diagnostic_pipeline.py` — проверка адаптивного роутинга

---

## Вектор 2: Мультипользовательские профили и синхронизация (Issue #8, #57)

**Цоль:** Поддержка нескольких пользователей с синхронизацией прогресса между устройствами.

### Архитектурные требования
1. **UserStore** — отдельный модуль (`core/user_store.py` или `db/users.py`):
   - `UserProfile` + `LearningSession` сериализуются в SQLite/Redis
   - thread_id → user_id mapping через `cl.user_session`
2. **Multi-session UI**:
   - Chainlit `cl.User` — выбор/создание профиля перед стартом
   - "Продолжить последний сеанс" или "Начать новый"
3. **Sync layer**:
   - `session_path` → `user_sessions/{user_id}/{thread_id}.json`
   - Checkpointer shard key: `user_id/thread_id`

### Изменения в настройках
```env
# .env.example
USER_STORAGE=sqlite          # sqlite | redis | memory
USER_DB_PATH=.users/users.sqlite3
SESSION_ROOT=.sessions       # → {root}/{user_id}/{thread_id}.json
```

### Тестируемость
- Unit: `test_user_store.py` — CRUD операции, session lookup
- Integration: `test_multi_user_resume.py` — 2 пользователя, 2 thread_id, изоляция checkpointer

---

## Вектор 3: Probing assessment + skills inference (в рамках #7)

**Цель:** Заменить `default_evaluator` (который ищет "good" в тексте) на настоящую LLM-оценку с RAG-контекстом.

### Текущее ограничение
- `ExaminerAgent.default_evaluator` — regex на "good"/"partial", 35–90 баллов
- `LLMEvaluator` — уже есть, но в stub-режиме не вызывается

### Реализация
1. Убрать `is_offline` guard в `examiner.assess()` — всегда вызывать `LLMEvaluator`, если inference client есть
2. Stub-режим: `StubLLMProvider` с `responder`, возвращающим JSON assessment
3. RAG-контекст в examiner prompt уже есть (`build_examiner_prompt(..., retrieved=...)`)

### Тестируемость
- `FakeLLMProvider` с discriminator на AssessmentOutput schema
- `test_examiner_llm_evaluation_quality.py` — проверка, что low-quality ответы получают score < 70

---

## Вектор 4: Observability + telemetry (production-ready)

**Цель:** Добавить структурированный логинг, метрики и трейсинг для продакшн-деплоя.

### Архитектурные изменения
1. **OpenTelemetry instrumentation**:
   - `traceloop` уже в `.venv` (но не используется)
   - Span-ы на каждом узле графа (coach/tutor/examiner), LLM вызовы, RAG retrieval
2. **Structured logging**:
   - `logging.JSONFormatter` для production
   - Correlation ID через `thread_id`
3. **Metrics**:
   - `prometheus_client` — request count, latency, LLM token usage
   - Экспортер в `/metrics` endpoint

### Изменения в CI
```yaml
# .github/workflows/ci.yml
- name: Type check with mypy
  run: mypy src/nereus --strict  # или pyright

- name: Test coverage
  run: pytest --cov=src/nereus --cov-fail-under=85
```

### Тестируемость
- Unit: `test_observability.py` — проверка span creation, metric counters
- Integration: `test_tracing_headers.py` — проверка propagation через graph

---

## Вектор 5: Multi-provider LLM abstraction (гибридный режим)

**Цель:** Расширить абстракцию `LLMProvider` для поддержки Ollama (локально), Anthropic напрямую, Gemini, и provider fallback.

### Архитектурные изменения
1. **Provider registry** в `llm/factory.py`:
   ```python
   PROVIDERS: dict[str, type[LLMProvider]] = {
       "stub": StubLLMProvider,
       "openrouter": OpenRouterProvider,
       "anthropic": AnthropicProvider,  # новый
       "gemini": GeminiProvider,  # новый
   }


   def build_llm_provider(name: str | None = None) -> LLMProvider:
       name = name or settings.llm_provider
       cls = PROVIDERS[name]
       ...
   ```
2. **Fallback chain**:
   - `LLM_PROVIDER=openrouter,anthropic-fallback` → try OpenRouter, fallback к Anthropic
3. **Ollama локальный** как optional backend:
   - `LLM_PROVIDER=ollama` → `OllamaProvider(host=localhost:11434)`
   - Требует `ollama` в зависимостях (опциональная)

### Тестируемость
- `test_provider_registry.py` — все провайдеры создаются через factory
- `test_fallback_chain.py` — при 503 на OpenRouter автоматически переключается на Anthropic

---

## Вектор 6: Enhanced materials + content ingestion pipeline

**Цель:** Расширить RAG materials pipeline: поддержка PDF, HTML, YouTube транскриптов.

### Архитектурные изменения
1. **Content loader abstraction** в `scripts/ingest_materials.py`:
   ```python
   class ContentLoader(Protocol):
       def load(self, path: Path) -> str: ...
       def supports(self, path: Path) -> bool: ...
   
   class MarkdownLoader, PdfLoader, HtmlLoader, YoutubeTranscriptLoader
   ```
2. **Metadata schema evolution**:
   ```python
   # db/chroma.py — metadatas расширяются
   {"topic_id": "1", "source": "file.md", "format": "markdown", "page": 0}
   ```
3. **Content indexing pipeline** в Docker Compose:
   - `ingest` профиль: `--format` флаг
   - Поддержка `--watch` для hot-reload

### Тестируемость
- `test_loaders.py` — unit-тесты каждого loader
- `test_ingest_formats.py` — E2E: ingest PDF + поиск по ChromaDB

---

## Вектор 7: CLI enhancement + scripting API

**Цель:** Превратить `main.py` и `eval_chain.py` в более мощный CLI с подкомандами.

### Предлагаемая структура
```bash
nereus --help
├── nereus run [--profile PROFILE] [--resume THREAD_ID]
├── nereus diagnose [--skill PYTHON] [--questions 5]
├── nereus ingest [--materials DIR] [--clear] [--format markdown|pdf]
├── nereus eval  [--skill PYTHON] [--out artifacts/run.jsonl]
├── nereus status [THREAD_ID]
└── nereus admin reset [--all-sessions] [--chroma-clear]
```

### Архитектурные изменения
1. **`typer`** вместо `argparse` (уже в `.venv`):
   - Автогенерация help, автодокументация
2. **CLI как отдельный слой** (`src/nereus/cli/`):
   - `cli/__init__.py`, `cli/main.py`, `cli/run.py`, `cli/admin.py`

### Тестируемость
- `test_cli.py` — `typer.testing.CliRunner` для каждой подкоманды
- Интеграция с `__main__.py` (`python -m nereus run`)

---

## Приоритизация рекомендаций

| Приоритет | Вектор | Причина |
|---|---|---|
| **P0 (следующий спринт)** | #3 Probing assessment + #1 Adaptive roadmapper | Наиболее влияют на core UX; зависят от друг друга; Issue #7 активно обсуждается |
| **P1** | #2 Multi-user profiles + #6 Enhanced materials | User-facing фичи, unlock новых use cases |
| **P2** | #4 Observability + #5 Multi-provider | Production readiness и flexibility |
| **P3** | #7 CLI enhancement | Developer experience, можно делать параллельно |

---

## Требования к CI для всех векторов

```yaml
# ci.yml — добавить:
- name: Type check
  run: mypy src/nereus --strict || pyright src/nereus
- name: Coverage check
  run: pytest --cov=src/nereus --cov-report=xml --cov-fail-under=85
- name: Format check
  run: ruff format --check .
- name: Security scan
  run: pip-audit  # или bandit
```

---

## Связь с дорожной картой (Project Board)

| Текущий статус доски | Соответствие векторам |
|---|---|
| #7 (adaptive diagnostics) | → Вектор 1 + 3 |
| #8 (multi-user) | → Вектор 2 |
| #57 (chat history UX) | → Вектор 2 |
| Backlog (#44, #45) | → Вектор 3 (production LLM by default) |

Векторы 1 и 3 напрямую реализуют Issue #7.
Вектор 2 реализует Issue #8 и #57.
Векторы 4–7 — продвижение к продакшн-готовому продукту.