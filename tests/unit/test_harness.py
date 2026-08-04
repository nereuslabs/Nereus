from __future__ import annotations

import argparse

from nereus.scripts.eval_chain import run


def _args(**kw) -> argparse.Namespace:
    base = dict(
        skill="Python",
        current_level="beginner",
        target_level="intermediate",
        hours=1.0,
        deadline=30,
        goal=None,
        submission="this is good, I have mastered this material",
        max_retries=2,
        out="artifacts/run.jsonl",
        log_level="INFO",
        dry_run=True,
    )
    base.update(kw)
    return argparse.Namespace(**base)


def test_harness_run_stub_trace(monkeypatch, tmp_path) -> None:
    monkeypatch.setenv("LLM_PROVIDER", "stub")
    args = _args(out=str(tmp_path / "run.jsonl"), dry_run=False)
    trace = run(args)

    assert trace["model"] == "StubLLMProvider"
    assert len(trace["roadmap"]) == 3
    assert trace["final_status"] == "completed"
    assert trace["final_assessment"]["verdict"] == "pass"
    assert trace["final_assessment"]["score"] >= 70.0
    assert isinstance(trace["llm_calls"], list)
    assert "session_brief" in trace
    assert "Python" in trace["session_brief"]
