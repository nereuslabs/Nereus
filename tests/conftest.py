from __future__ import annotations

import pytest

from nereus.config import settings as settings_module
from nereus.core.state import Roadmap, RoadmapTopic, UserLevel, UserProfile


@pytest.fixture(autouse=True)
def _force_stub_offline(monkeypatch, request) -> None:
    """Keep offline tests hermetic regardless of a developer's ``.env``.

    pydantic-settings caches values at import time, so ``monkeypatch.setenv`` is
    ineffective once ``Settings()`` is built. This autouse fixture pins the
    LLM/embedding/checkpointer backends to stub/sqlite for every *non-live* test
    so the 90+ offline tests stay deterministic even when a developer's ``.env``
    (e.g. ``LLM_PROVIDER=ollama``, ``NEREUS_RUN_LIVE=1``) is present.

    Live tests under ``tests/integration/test_live_*.py`` are exempt — they
    gate themselves via ``pytest.mark.skipif`` on ``NEREUS_RUN_LIVE`` and manage
    their own provider wiring.

    Note: we pin on the ``settings`` singleton directly (not env) because a
    mid-session ``importlib``-time ``load_dotenv`` (e.g. triggered by
    ``import chainlit``) can rewrite ``os.environ`` after collection — reading
    the env var per-test would therefore spuriously flip the mode.
    """
    if (
        request.node.fspath
        and request.node.fspath.basename
        and request.node.fspath.basename.startswith("test_live_")
    ):
        return  # live integration suites carry their own gating + provider wiring
    s = settings_module.settings
    monkeypatch.setattr(s, "llm_provider", "stub")
    monkeypatch.setattr(s, "embedding_provider", "stub")
    monkeypatch.setattr(s, "checkpoint_backend", "sqlite")


@pytest.fixture
def user_profile() -> UserProfile:
    return UserProfile(
        skill="Python",
        current_level=UserLevel.BEGINNER,
        target_level=UserLevel.INTERMEDIATE,
        hours_per_day=1.0,
        deadline_days=30,
        goal="Get comfortable writing scripts.",
    )


@pytest.fixture
def roadmap() -> Roadmap:
    return Roadmap(
        topics=[
            RoadmapTopic(
                id="1",
                title="Python: fundamentals",
                description="Syntax and core concepts.",
            ),
            RoadmapTopic(
                id="2",
                title="Python: practice",
                description="Hands-on exercises.",
            ),
            RoadmapTopic(
                id="3",
                title="Python: advanced",
                description="Advanced topics.",
            ),
        ]
    )


@pytest.fixture
def base_state(user_profile: UserProfile) -> dict:
    return {
        "user_profile": user_profile,
        "roadmap": Roadmap(),
        "current_topic_index": 0,
        "material": "",
        "task": "",
        "user_submission": None,
        "assessment": None,
        "retry_count": 0,
        "max_retries": 2,
        "status": "coaching",
        "messages": [],
    }
