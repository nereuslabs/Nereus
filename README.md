# Nereus

AI-тьютор, работающий по принципу автоматизированного учебного процесса — от Roadmap до **уверенных практических навыков**!

## Архитектура

Nereus построен как зацикленный автомат из трёх агентов, оркеструемых через **LangGraph** (human-in-the-loop):

1. **Агент-коуч** — собирает профиль пользователя (скилл, уровень, сроки) и строит **Roadmap**.
2. **Агент-тьютор** — выдаёт учебные материалы и задания по текущей теме Roadmap, углубляет слабые места.
3. **Агент-экзаменатор** — проверяет ответы, ставит оценку и вердикт `PASS` / `RETRY`, после чего автомат решает: двигаться дальше или повторить материал.

## Статус

**Стадия:** прототипирование MVP (Шаг 1 done, Шаг 2 in progress).

На текущем этапе реализовано:
- полный циклический граф LangGraph с условным роутингом (`PASS`/`RETRY`/`END`);
- human-in-the-loop (интерактивный режим через `interrupt`);
- **абстракция LLM-провайдера** (`LLMProvider`: `OllamaProvider` + `StubLLMProvider`); агенты генерируют Roadmap/материалы/оценки через модель, с fallback на детерминированные заглушки без сети;
- автотесты (unit + integration); CI (ruff + pytest);
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

После чего `python main.py` будет генерировать Roadmap, материалы и оценки через модель.

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
├── config/          # Настройки приложения (pydantic-settings)
├── core/
│   ├── state.py     # Модели состояния и доменов
│   ├── router.py    # Условные переходы автомата
│   └── graph.py     # Сборка StateGraph (LangGraph)
├── agents/          # Коуч / Тьютор / Экзаменатор (LLM + stub-fallback)
├── llm/
│   ├── base.py      # LLMProvider (абстракция)
│   ├── ollama.py    # Native /api/chat клиент
│   ├── stub.py      # In-memory провайдер (тесты/без сети)
│   ├── schema.py    # Парсинг JSON из ответов модели
│   └── factory.py   # Выбор провайдера по настройкам
├── db/              # ChromaDB (задел на будущее)
└── ui/              # Chainlit (задел на будущее)
```
