# Разработка

## Окружение

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"      # dev = ruff + pytest (и др.)
```

Требуется **Python 3.11+**.

## LLM‑провайдер

`LLM_PROVIDER` выбирает бэкенд (по умолчанию `stub` — полностью офлайн, безетапный
JSON‑вывод, идеален для CI/демо):

| Провайдер | Переменные | Требования |
|-----------|-----------|------------|
| `stub`      | `LLM_PROVIDER=stub` | по умолчанию, ничего не нужно |
| `openrouter` | `OPENROUTER_API_KEY`, `OPENROUTER_MODEL=openrouter/free` | ключ OpenRouter |

(Ранее поддерживались `ollama`/`openai` — мигрировано на OpenRouter, см. #46.)

## Проверка

```bash
ruff check .
ruff format --check .
pytest                       # 174 passed, 2 skipped
NEREUS_RUN_LIVE=1 pytest     # + live‑интеграционные с OpenRouter (и ключом)
```

Live‑тесты включаются только при `NEREUS_RUN_LIVE=1`, чтобы CI оставался
офлайн‑детерминированным.

## Хранилище и ветвление

CI (`.github/workflows/ci.yml`) запускает `ruff` + `pytest` на push/PR в
`develop` и `main`. `develop` — fast lane; `main` — release gate (правила
контроля доступа репо‑rulesетом, см. `CLAUDE.md`).
