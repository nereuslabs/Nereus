# Nereus Wiki

> Интерактивный «машина учёбы»: LLM‑агент, который разбирает ваш roadmap,
> задаёт вопросы, проверяет знания и адаптирует план — всё через обычный
> диалог в терминале.

## Содержание
- [Architecture](Architecture.md)
- [Roadmap](Roadmap.md)
- [Development](Development.md)

## Статус
- ✅ **Step 1** — MVP learning automaton (LangGraph)
- ✅ **Step 2** — LLM provider abstraction + Ollama
- ✅ **Step 3** — LLM runtime, prompt registry, schema validation, session memory
- 🛠️ **Step 4** — RAG store + Chainlit UI (скаффолд `ChromaStore` в
  `feature/step-4-rag-ui`, ждёт вашего одобрения)

Первая полностью рабочая версия отслеживается в
[MVP 1.0](https://github.com/Yan123-tech/Nereus/milestones/1).

## Быстрый старт
```bash
python -m venv .venv && . .venv/bin/activate
pip install -e .
python main.py      # LLM_PROVIDER=stub  → полностью офлайн
```
