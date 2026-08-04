from __future__ import annotations

import json

import pytest
from pydantic import ValidationError

from nereus.llm.schema import (
    AssessmentOutput,
    MaterialOutput,
    RoadmapOutput,
    SummaryOutput,
    parse_structured,
)


def test_parse_structured_plain_json() -> None:
    raw = json.dumps({"score": 88, "feedback": "nice", "weak_areas": []})
    out = parse_structured(raw, AssessmentOutput)
    assert isinstance(out, AssessmentOutput)
    assert out.score == 88.0
    assert out.weak_areas == []


def test_parse_structured_accepts_fenced_and_prose() -> None:
    raw = (
        'Here is the roadmap:\n```json\n'
        '{"topics": [{"id":"1","title":"A","description":"d"}]}\n```\nDone.'
    )
    out = parse_structured(raw, RoadmapOutput)
    assert len(out.topics) == 1
    assert out.topics[0].title == "A"


def test_parse_structured_raises_on_garbage() -> None:
    with pytest.raises((ValueError, ValidationError)):
        parse_structured("no json here at all", AssessmentOutput)


def test_assessment_output_rejects_out_of_range_score() -> None:
    with pytest.raises(ValidationError):
        AssessmentOutput(score=150, feedback="x", weak_areas=[])
    with pytest.raises(ValidationError):
        AssessmentOutput(score=-1, feedback="x", weak_areas=[])


def test_material_output_fields() -> None:
    out = MaterialOutput.model_validate({"material": "m", "task": "t"})
    assert out.material == "m" and out.task == "t"


def test_summary_output_optional() -> None:
    out = parse_structured('{"summary": "recap"}', SummaryOutput)
    assert out.summary == "recap"
