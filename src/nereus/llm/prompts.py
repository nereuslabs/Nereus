from __future__ import annotations

from typing import Any, Sequence

from nereus.core.state import RetrievedChunk, RoadmapTopic
from nereus.llm.params import AgentRole

SESSION_PREAMBLE = "SESSION CONTEXT — use this to keep every answer grounded:"


def _system(role: AgentRole) -> str:
    if role == AgentRole.COACH:
        return (
            "You are an expert learning coach. Build a personalized, ordered "
            "learning roadmap (fundamentals -> advanced). Return ONLY valid JSON "
            'with key "topics": [{"id": "<1-based str>", "title": "<str>", '
            '"description": "<str>"}], 3 to 8 topics. Never invent skills '
            "beyond the user's goal. If unsure, ask."
        )
    if role == AgentRole.TUTOR:
        return (
            "You are a patient tutor. Generate concise learning material plus a "
            "single practical task for the given topic. Return ONLY valid JSON "
            "with keys: material (<str>), task (<str>)."
        )
    if role == AgentRole.EXAMINER:
        return (
            "You are a strict examiner for an AI tutor. Grade the student's "
            "answer to the given task. Return ONLY valid JSON with keys: "
            "score (<int 0-100>), feedback (<str>), weak_areas (list of str). "
            "Passing score is >= 70. Be honest: if the answer is shallow or "
            "wrong, score < 70 and name the weak areas."
        )
    return (  # SUMMARIZER
        "You are a concise summarizer. Condense the conversation so far into "
        "a single short paragraph that preserves the learner's current topic, "
        "recent progress and open weak areas. Do NOT output JSON."
    )


def _brief(session: Any | None) -> str:
    if session is None:
        return ""
    brief = getattr(session, "to_brief", None)
    if callable(brief):
        return str(brief())
    return str(getattr(session, "session_brief", "") or "")


def build_coach_prompt(session: Any | None = None) -> list[dict[str, str]]:
    system = _system(AgentRole.COACH)
    brief = _brief(session)
    if brief:
        system = f"{SESSION_PREAMBLE}\n{brief}\n\n{system}"
    return [
        {"role": "system", "content": system},
        {"role": "user", "content": "Build a roadmap now."},
    ]


def build_tutor_prompt(
    topic: RoadmapTopic,
    *,
    revision: bool = False,
    weak_areas: Sequence[str] = (),
    session: Any | None = None,
) -> list[dict[str, str]]:
    system = _system(AgentRole.TUTOR)
    brief = _brief(session)
    if brief:
        system = f"{SESSION_PREAMBLE}\n{brief}\n\n{system}"

    if revision:
        instruction = (
            "The student previously struggled with: "
            f"{', '.join(weak_areas)}. Focus the material on closing these gaps."
        )
    else:
        instruction = "Introduce the topic clearly for the student."

    user = (
        f"Topic title: {topic.title}\nTopic description: {topic.description}\n"
        f"{instruction}"
    )
    return [
        {"role": "system", "content": system},
        {"role": "user", "content": user},
    ]


def build_examiner_prompt(
    topic_title: str,
    task: str,
    submission: str,
    *,
    session: Any | None = None,
    retrieved: Sequence[RetrievedChunk] | None = None,
) -> list[dict[str, str]]:
    system = _system(AgentRole.EXAMINER)
    brief = _brief(session)
    if brief:
        system = f"{SESSION_PREAMBLE}\n{brief}\n\n{system}"
    user = f"Topic: {topic_title}\nTask: {task}"
    if retrieved:
        context_block = "\n".join(
            f"- {chunk.content[:300]} (relevance {chunk.score:.2f})"
            for chunk in retrieved
        )
        user += f"\n\n[Retrieved context]\n{context_block}"
    user += f"\n\nStudent's answer: {submission}"
    return [
        {"role": "system", "content": system},
        {"role": "user", "content": user},
    ]


def build_summarizer_prompt(messages: Sequence[dict[str, str]]) -> list[dict[str, str]]:
    return [
        {"role": "system", "content": _system(AgentRole.SUMMARIZER)},
        {"role": "user", "content": "Conversation: " + _flatten(messages)},
    ]


def _flatten(messages: Sequence[dict[str, str]]) -> str:
    parts: list[str] = []
    for msg in messages:
        role = msg.get("role", "user")
        parts.append(f"[{role}] {msg.get('content', '')}")
    return "\n".join(parts)
