from __future__ import annotations

from nereus.core.state import NereusState, Verdict

RETRY_TUTOR = "retry_tutor"
ADVANCE_TUTOR = "advance_tutor"
END = "end"


def route_after_exam(state: NereusState) -> str:
    """Decide the next node after the examiner has produced an assessment.

    * ``RETRY``             -> deep-dive the same topic (revision tutor)
    * ``PASS`` + more topics -> advance to the next roadmap topic
    * ``PASS`` + last topic  -> the learning process is complete
    """
    assessment = state.get("assessment")
    if assessment is None:
        raise ValueError("route_after_exam requires an assessment in state")

    if assessment.verdict == Verdict.RETRY:
        return RETRY_TUTOR

    next_index = state["current_topic_index"] + 1
    if next_index < len(state["roadmap"].topics):
        return ADVANCE_TUTOR

    return END