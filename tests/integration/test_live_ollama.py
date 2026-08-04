from __future__ import annotations

import json
import os

import pytest

from nereus.core.factory import build_nereus_graph
from nereus.core.state import UserLevel, UserProfile

LIVE = pytest.mark.skipif(
    os.environ.get("NEREUS_RUN_LIVE", "0") != "1",
    reason="set NEREUS_RUN_LIVE=1 (and LLM_PROVIDER=ollama) to run against a real model",
)


def _profile() -> UserProfile:
    return UserProfile(
        skill="Python",
        current_level=UserLevel.BEGINNER,
        target_level=UserLevel.INTERMEDIATE,
        hours_per_day=1.0,
        deadline_days=30,
        goal="Master scripting.",
    )


@LIVE
def test_live_ollama_full_pipeline() -> None:
    """End-to-end run against a real Ollama backend (off by default).

    Asserts structural validity of the generated roadmap, materials and the
    final assessment. Writes a JSONL trace under ``artifacts/live_run.jsonl``
    for offline inspection.
    """
    graph = build_nereus_graph(interactive=False)
    final = graph.invoke(
        {"user_profile": _profile(), "user_submission": "this is good, I've learned it well"}
    )

    assert final["status"] == "completed"
    assert len(final["roadmap"].topics) >= 1
    assert final["assessment"] is not None
    assert 0.0 <= final["assessment"].score <= 100.0

    os.makedirs("artifacts", exist_ok=True)

    trace = {
        "model": graph._tutor_agent._inference.model_name()
        if getattr(graph._tutor_agent, "_inference", None)
        else "n/a",
        "roadmap": [t.model_dump() for t in final["roadmap"].topics],
        "final_status": final["status"],
        "final_topic_index": final["current_topic_index"],
        "final_assessment": final["assessment"].model_dump(),
    }
    inference = getattr(graph._tutor_agent, "_inference", None)
    if inference is not None and getattr(inference, "calls", None):
        trace["llm_calls"] = inference.calls
    with open("artifacts/live_run.jsonl", "a", encoding="utf-8") as fh:
        fh.write(json.dumps(trace, default=str) + "\n")
