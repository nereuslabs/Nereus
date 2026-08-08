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
| `openrouter` | `OPENROUTER_API_KEY`, `OPENROUTER_MODEL=openrouter/free` | ключ с openrouter.ai |

`openrouter` также служит провайдером эмбеддингов (`EMBEDDING_PROVIDER=openrouter`),
заменяя тяжёлый локальный Ollama‑embed (~8 ГБ RAM).

## Проверка
```bash
ruff check .
ruff format --check .
pytest                       # 80 passed, 6 skipped (live)
NEREUS_RUN_LIVE=1 pytest     # + OpenRouter/Redis/Live embed
```

## Хранилище
CI (`.github/workflows/ci.yml`) запускает `ruff` + `ruff format --check` +
`pytest` на `main`/`develop` и `pull_request`.

## Ветвление (GitFlow Lite)
- `main` — стабильный релиз
- `develop` — интеграция (только сюда PR)
- `feature/issue-N-<slug>` — ветвь фичи → PR в `develop` → слияние после ревью
- Issues автоматически закрываются через `Fixes #N` в PR description
