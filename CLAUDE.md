# CLAUDE.md — Nereus Development Guide

## Architecture
- **LangGraph StateGraph** for agent orchestration with human-in-the-loop interrupts
- **Pydantic models** for state (`NereusState` TypedDict + domain `BaseModel`s)
- **Offline-first**: `StubLLMProvider` + `is_offline_inference()` guard for CI/test safety
- **Structured inference** with retry logic via `StructuredInferenceClient`

## Project Structure
```
src/nereus/
├── agents/         # CoachAgent, TutorAgent, ExaminerAgent, DiagnosticAgent
├── core/           # graph.py, factory.py, state.py, session.py, user_store.py, persistence.py
├── llm/            # providers, params, prompts, schema, inference
├── config/         # settings.py (pydantic-settings)
├── scripts/        # eval_chain.py
└── ui/             # Chainlit web app
tests/
├── unit/           # fast, hermetic tests
└── integration/    # full graph pipeline tests
```

## Multi-user Sessions (P1, Issues #8/#57)
- `UserStore` (`core/user_store.py`): SQLite-backed UserProfile storage, memory fallback
- `UserSession` (`core/session.py`): on-disk session snapshot at `SESSION_ROOT/{user_id}/{session_id}.json`
- `NereusGraph`: accepts `session_id` + `user_id` params; loads/saves `UserSession` via `_load_user_session`/`_dump_session`
- CLI: `--new-session`, `--session-id <id>`, `--user-id <id>`
- Env: `USER_STORAGE`, `USER_DB_PATH`, `SESSION_ROOT`

## GitHub Flow
- Feature branches from `develop`
- PR targeting `develop`
- CI: `ruff check` + `pytest` on push/PR
- Issues tracked on project board (GraphQL automation)

## Testing
```bash
# Offline (default)
python -m pytest tests/

# Live (requires OPENROUTER_API_KEY)
NEREUS_RUN_LIVE=1 python -m pytest tests/integration/test_live_*.py

# Single test
python -m pytest tests/unit/test_user_session.py -xvs
```

## Common Tasks
- Start dev server: `python main.py`
- New session: `python main.py --new-session --user-id <uuid>`
- Resume session: `python main.py --session-id <uuid> --user-id <uuid>`
- Lint+format: `ruff check --fix . && ruff format .`
