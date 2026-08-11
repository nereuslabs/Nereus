from __future__ import annotations

from nereus.core.router import ADVANCE_TUTOR, RETRY_TUTOR, route_after_exam
from nereus.core.state import Assessment, Roadmap, RoadmapTopic, Verdict


def _make_state(roadmap_topics: int, index: int, verdict: Verdict) -> dict:
    topics = [RoadmapTopic(id=str(i), title=f"t{i}", description="") for i in range(roadmap_topics)]
    return {
        "roadmap": Roadmap(topics=topics),
        "current_topic_index": index,
        "assessment": Assessment(topic_id=str(index), score=90.0, verdict=verdict),
    }


def test_retry_routes_to_continue_tutor() -> None:
    assert route_after_exam(_make_state(3, 0, Verdict.RETRY)) == RETRY_TUTOR


def test_pass_with_more_topics_routes_to_advance() -> None:
    assert route_after_exam(_make_state(3, 0, Verdict.PASS)) == ADVANCE_TUTOR


def test_pass_on_last_topic_ends() -> None:
    assert route_after_exam(_make_state(3, 2, Verdict.PASS)) == "end"


def test_router_requires_assessment() -> None:
    import pytest

    with pytest.raises(ValueError):
        route_after_exam({"assessment": None})
