# Nereus Wiki

> Интерактивный «машина учёбы»: LLM‑агент, который разбирает ваш roadmap,
> задаёт вопросы, проверяет знания и адаптирует план — всё через диалог в
> терминале или в Web UI (Chainlit).

## Содержание
- [Architecture](Architecture.md)
- [Development](Development.md)
- [Roadmap](Roadmap.md)
- [Releases](https://github.com/nereuslabs/Nereus/releases)

## Статус

**Репо:** `nereuslabs/Nereus` (ранее `Yan123-tech/Nereus`).

| Версия | Статус | Что входит |
|--------|--------|------------|
| **v1.1.0** | ✅ Released (на `main`) | Адаптивная диагностика (#7) + мульти‑пользовательские сессии + Redis `UserStore` (#8, #57) |
| **v1.0.0** | ✅ Released | Шаги 1–6: автомат‑агент, LLM‑абстракция, RAG, Chainlit‑UI, персистентные сессии |

Этапы (см. Milestones и доску проекта):
- Шаг 1 ✅ — базовый автомат‑агент (LangGraph, LLM runtime, промпты, память). В `develop`.
- Шаг 2 ✅ — абстракция LLM‑провайдера (`stub` + `openrouter`).
- Шаг 3 ✅ — inference‑клиент с ретраями, схемы/промпты, CLI‑харнес (`scripts/eval_chain.py`).
- Шаг 4 ✅ — RAG: эмбеддинги + ChromaDB (`db/chroma.py`) + retrieval в узлах графа.
- Шаг 5 ✅ — Chainlit Web‑UI (`src/nereus/ui/app.py`).
- Шаг 6 ✅ — Персистентная сессия + чекпоинтеры (`sqlite`/`redis`/`memory`).
- Шаг 7 ✅ — Адаптивная диагностика навыков (#7).
- Шаг 8 ✅ — Мульти‑пользовательские профили + сессии (#8, #57).

Ветвление — **GitHub Flow** с integration‑веткой `develop`: feature‑ветки → PR (base `develop`) → squash/merge. Релизы — обратно из `develop` в `main` через `--admin`‑слияние (см. `CLAUDE.md`).

## Быстрый старт

```bash
git clone https://github.com/nereuslabs/Nereus.git
cd Nereus
python -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"
python main.py      # LLM_PROVIDER=stub → полностью офлайн
```

Подробнее — в [README](https://github.com/nereuslabs/Nereus#readme) и [Development](Development.md).
