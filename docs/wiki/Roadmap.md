# Roadmap

| Шаг | Содержание | Статус | Issue |
|-----|------------|--------|-------|
| 1 | MVP learning automaton (LangGraph) | ✅ Done | PR #1 |
| 2 | Абстракция LLM‑провайдера + Ollama | ✅ Done | PR #2 |
| 3 | Рантайм LLM, промпты, схемы, память сессии | ✅ Done | PR #3 |
| 4 | RAG: ChromaDB + retrieval‑augmented экзаменатор | ✅ Done | #4 |
| 5 | Chainlit‑UI | ✅ Done | #5 |
| 6 | Персистентная сессия + чекпоинты | ✅ Done | #6 |
| — | Runtime LearningSession dump/load wiring | ✅ Done | #22 |
| 7 | Автоматическая диагностика навыков | 🟩 Todo | #7 |
| 8 | Мульти‑пользовательские профили + синхронизация | 🟩 Todo | #8 |

Фазы Backlog (#7 diagnostics, #8 multi-user) планируются после MVP.

## Выполняется (MVP 1.0 GA)
- #23 — UIApp: inject persistent checkpointer
- #24 — Ollama embedding integration test
- #25 — Local RAG demo (ingest script + README)
- #26 — docs/wiki mirror (этот файл)

## Мильстоуны
- **MVP 1.0** — Steps 1–3 + RAG (#4) + UI (#5) + persistent session (#6):
  https://github.com/nereuslabs/Nereus/milestones/1
