# Разработка

## Окружение
```bash
python3 -m venv .venv && . .venv/bin/activate
pip install -e ".[dev]"      # dev = ruff + pytest
```

## LLM‑провайдер
`LLM_PROVIDER` выбирает бэкенд (по умолчанию `stub` — полностью офлайн,
безетапный JSON‑вывод, идеален для CI/демо):

| Провайдер | Переменные | Требования |
|-----------|-----------|------------|
| `stub`    | `LLM_PROVIDER=stub` | выбран по умолчанию, ничего не нужно |
| `ollama`  | `OLLAMA_BASE_URL=http://localhost:11434`, `OLLAMA_MODEL=qwen3:1.7b` | `ollama serve` |
| `openai`  | `OPENAI_API_KEY`, `OPENAI_MODEL=gpt-4o-mini` | ключ OpenAI |

## Проверка
```bash
ruff check .
ruff format --check .
pytest                       # 95 passed, 2 skipped (live)
NEREUS_RUN_LIVE=1 pytest     # дополнительно с Ollama/Redis, если включено
```

## Хранилище
CI (`.github/workflows/ci.yml`) запускает `ruff` + `ruff format --check` +
`pytest` на `main`/`develop` и `pull_request`.

## Ветвление (GitFlow Lite)
- `main` — стабильный релиз
- `develop` — интеграция (только сюда PR)
- `feature/issue-N-<slug>` — ветвь фичи → PR в `develop` → слияние после ревью
- Issues автоматически закрываются через `Fixes #N` в PR description
