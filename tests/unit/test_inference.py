from __future__ import annotations

import json

import pytest

from nereus.llm.inference import LLMOutputError, StructuredInferenceClient
from nereus.llm.params import AgentRole
from nereus.llm.schema import AssessmentOutput, RoadmapOutput
from nereus.llm.stub import StubLLMProvider


def _responder(payload: str):
    def responder(messages, **_):
        return payload

    return responder


def test_inference_success_returns_model() -> None:
    payload = json.dumps({"topics": [{"id": "1", "title": "A", "description": "d"}]})
    client = StructuredInferenceClient(StubLLMProvider(responder=_responder(payload)))
    out = client.generate([], role=AgentRole.COACH, output_model=RoadmapOutput)
    assert isinstance(out, RoadmapOutput)
    assert out.topics[0].title == "A"
    assert len(client.calls) == 1


def test_inference_retries_on_bad_json_then_succeeds() -> None:
    state = {"calls": 0}

    def responder(messages, **_):
        state["calls"] += 1
        if state["calls"] < 3:
            return "not json <<<"
        return json.dumps({"score": 90, "feedback": "ok", "weak_areas": []})

    client = StructuredInferenceClient(
        StubLLMProvider(responder=responder), max_retries=2
    )
    out = client.generate([], role=AgentRole.EXAMINER, output_model=AssessmentOutput)
    assert out.score == 90.0
    assert len(client.calls) == 3  # 1 initial + 2 retries


def test_inference_exhausts_retries_raises_llm_output_error() -> None:
    client = StructuredInferenceClient(
        StubLLMProvider(responder=_responder("garbage <<< not json")), max_retries=2
    )
    with pytest.raises(LLMOutputError):
        client.generate([], role=AgentRole.COACH, output_model=RoadmapOutput)
    assert len(client.calls) == 3


def test_inference_strips_json_fences() -> None:
    payload = '```json\n{"score": 77, "feedback": "ok", "weak_areas": []}\n```'
    client = StructuredInferenceClient(StubLLMProvider(responder=_responder(payload)))
    out = client.generate([], role=AgentRole.EXAMINER, output_model=AssessmentOutput)
    assert out.score == 77.0
