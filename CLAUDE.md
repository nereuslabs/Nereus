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
- `UserStore` (`core/user_store.py`): `UserStore` (SQLite + memory fallback) and
  `UserStoreRedis` (Redis hash per user, degrades to memory). `build_user_store()`
  dispatches on `settings.user_storage` (`sqlite` | `redis` | `memory`).
- `UserSession` (`core/session.py`): on-disk session snapshot at `SESSION_ROOT/{user_id}/{session_id}.json`
- `NereusGraph`: accepts `session_id` + `user_id` params; loads/saves `UserSession` via `_load_user_session`/`_dump_session`
- CLI: `--new-session`, `--session-id <id>`, `--user-id <id>`
- Env: `USER_STORAGE` (`sqlite`/`redis`/`memory`), `USER_DB_PATH`, `REDIS_HOST`, `REDIS_PORT`, `SESSION_ROOT`

## Adaptive Diagnostics (Issue #7)
- `DiagnosticAgent` (`agents/diagnostic.py`): generates an adaptive quiz, evaluates
  answers → `WeaknessReport`.
- `NereusGraph(run_diagnostic=True)` prepends a `diagnostic → interrupt → coach`
  entry so the roadmap is ordered by weak areas.
- Examiner pass threshold is difficulty-scaled (`70 + difficulty*15`, clamped ≤ 85).
- Opt-in via CLI `--diagnostic` / env `RUN_DIAGNOSTIC=true` / `DIAGNOSTIC_QUESTION_COUNT`.
- Offline safety: the autouse fixture in `tests/conftest.py` pins `run_diagnostic=False`
  so a developer `.env` cannot pull the diagnostic/interrupt path into hermetic tests.

## Releases
- `develop` is the integration branch; `main` is the release gate.
- `main` is governed by a **repository ruleset** (`id=20506063`, source
  `nereuslabs/Nereus`) — not legacy branch-protection — enforcing
  `deletion`, `non_fast_forward`, and a `pull_request` rule (review count +
  required status check `lint-and-test (3.11)`). The ruleset **cannot** be
  managed by a token lacking the `manage_rulesets` scope (PATCH → HTTP 404),
  and direct/non-fast-forward pushes to `main` are rejected even for admins
  *unless* an admin‑bypass ruleset actor is configured.
- To promote a release to `main`: open a PR `develop → main`, wait for
  `lint-and-test (3.11)` = success, then merge **as admin** (`gh pr merge -R
  nereuslabs/Nereus --* --admin`).
- Annotate + push the tag (`git tag -s vX.Y.Z <main_tip> && git push origin vX.Y.Z`)
  and publish a GitHub Release targeting `main`.
- **v1.1.0 lesson:** a `--rebase --admin` merge rewrites SHAs, so the tag must
  be (re)created on `main`'s tip and the Release `target_commitish` updated to
  `main`; verify `git rev-list -n1 vX.Y.Z == main`.

## GitHub Flow
- Feature branches from `develop`
- PR targeting `develop`
- CI: `ruff check` + `ruff format --check` + `pytest` on push/PR
- `develop` is the fast lane (CI + linear history); `main` is the release gate
  (CI + 1 required review + linear history, protected).
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
