from __future__ import annotations

import json

import httpx

from nereus.core.factory import build_nereus_graph
from nereus.core.state import Verdict
from nereus.llm.openrouter import OpenRouterProvider


def _superset_reply() -> dict:
    """A single JSON payload that validates against every LLM output schema.

    Pydantic models use the default ``extra='ignore'``, so a superset document
    is accepted by RoadmapOutput (topics), MaterialOutput (material/task) and
    AssessmentOutput (score/feedback/weak_areas) alike. This lets one mocked
    endpoint drive the *whole* coach -> tutor -> examiner cycle.
    """
    return {
        "model": "openrouter/free",
        "choices": [
            {
                "message": {
                    "content": json.dumps(
                        {
                            "topics": [
                                {
                                    "id": "1",
                                    "title": "Python: fundamentals",
                                    "description": "Core concepts and syntax.",
                                }
                            ],
                            "material": "Here is the lesson text.",
                            "task": "Write a hello-world script.",
                            "score": 90.0,
                            "feedback": "Well done, topic mastered.",
                            "weak_areas": [],
                        }
                    )
                }
            }
        ],
    }


def test_openrouter_provider_drives_full_pipeline(base_state) -> None:
    """End-to-end (hermetic): the real OpenRouterProvider feeds every agent.

    Injects an ``OpenRouterProvider`` backed by ``httpx.MockTransport`` (no
    network, no API key required) and runs the complete coach -> tutor ->
    examiner automaton, asserting the pipeline reaches ``completed`` with a
    PASS assessment. This exercises the production code path that free-router
    prose-vs-JSON ambiguity could break.
    """
    def handler(_request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, json=_superset_reply())

    provider = OpenRouterProvider(
        api_key="test-key",
        client=httpx.Client(transport=httpx.MockTransport(handler)),
    )
    graph = build_nereus_graph(interactive=False, provider=provider)

    final = graph.invoke(
        {**base_state, "user_submission": "this is good, I've learned it well"},
        config={"configurable": {"thread_id": "openrouter-e2e"}},
    )

    # Provider saw one call per agent (coach, tutor, examiner).
    assert len(provider.calls) == 3
    assert provider.last_model == "openrouter/free"
    for entry in provider.calls:
        assert entry["status"] == 200

    assert final["status"] == "completed"
    assert final["assessment"] is not None
    assert final["assessment"].verdict == Verdict.PASS
    assert final["assessment"].score == 90.0
    provider.close()
