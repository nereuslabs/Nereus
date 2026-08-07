from __future__ import annotations

from nereus.agents.coach import CoachAgent
from nereus.agents.examiner import ExaminerAgent
from nereus.agents.tutor import TutorAgent
from nereus.core.state import Roadmap, Verdict


def test_coach_builds_roadmap_from_profile(base_state) -> None:
    result = CoachAgent().run(base_state)
    roadmap: Roadmap = result["roadmap"]
    assert len(roadmap.topics) == 3
    assert "Python" in roadmap.topics[0].title


def test_coach_requires_profile() -> None:
    import pytest

    with pytest.raises(ValueError):
        CoachAgent().run({"user_profile": None})


def test_tutor_provides_material_and_task(base_state, roadmap) -> None:
    base_state["roadmap"] = roadmap
    result = TutorAgent().run(base_state)
    assert result["material"]
    assert result["task"]
    assert "fundamentals" in result["material"]


def test_tutor_retry_produces_revision_material(base_state, roadmap) -> None:
    from nereus.core.state import Assessment

    base_state["roadmap"] = roadmap
    base_state["assessment"] = Assessment(
        topic_id="1",
        score=40.0,
        verdict=Verdict.RETRY,
        feedback="Needs work.",
        weak_areas=["syntax"],
    )
    result = TutorAgent().run(base_state)
    assert "повтор" in result["material"].lower()
    assert "syntax" in result["material"]


def test_examiner_passes_good_submission(base_state, roadmap) -> None:
    base_state["roadmap"] = roadmap
    result = ExaminerAgent().run({**base_state, "user_submission": "this is good work"})
    assert result["assessment"].verdict == Verdict.PASS


def test_examiner_retries_bad_submission(base_state, roadmap) -> None:
    base_state["roadmap"] = roadmap
    result = ExaminerAgent().run({**base_state, "user_submission": "nonsense"})
    assert result["assessment"].verdict == Verdict.RETRY


def test_examiner_requires_submission(base_state) -> None:
    import pytest

    with pytest.raises(ValueError):
        ExaminerAgent().run(base_state)
