from __future__ import annotations

from typing import Any, Sequence

from nereus.core.state import RetrievedChunk, RoadmapTopic, UserProfile
from nereus.llm.params import AgentRole

SESSION_PREAMBLE = "SESSION CONTEXT — use this to keep every answer grounded:"

# Mandates Russian *content* output while keeping JSON keys (material, task,
# score, ...) intact — those are structure, not speech (#52). The OpenRouter
# chat model otherwise defaults to English once the system prompt is English.
LANGUAGE_INSTRUCTION = (
    " Отвечайте строго на русском языке. Все текстовые поля выходных данных "
    "(material, task, feedback, weak_areas, а также title и description тем) "
    "должны быть на русском языке. JSON-ключи (material, task, score и т.п.) "
    "остаются английскими — это структура, а не контент. Не используйте "
    "английский в текстовых значениях."
)


def _system(role: AgentRole) -> str:
    if role == AgentRole.COACH:
        base = (
            "You are an expert learning coach. Build a personalized, ordered "
            "learning roadmap (fundamentals -> advanced). Return ONLY valid JSON "
            'with key "topics": [{"id": "<1-based str>", "title": "<str>", '
            '"description": "<str>"}], 3 to 8 topics. Never invent skills '
            "beyond the user's goal. If unsure, ask."
        )
    elif role == AgentRole.TUTOR:
        base = (
            "You are a patient tutor. Generate concise learning material plus a "
            "single practical task for the given topic. Return ONLY valid JSON "
            "with keys: material (<str>), task (<str>)."
        )
    elif role == AgentRole.EXAMINER:
        base = (
            "You are a strict examiner for an AI tutor. Grade the student's "
            "answer to the given task. Return ONLY valid JSON with keys: "
            "score (<int 0-100>), feedback (<str>), weak_areas (list of str). "
            "Passing score is >= 70. Be honest: if the answer is shallow or "
            "wrong, score < 70 and name the weak areas."
        )
    elif role == AgentRole.DIAGNOSTIC:
        base = (
            "You are an expert diagnostician. Create a short diagnostic quiz "
            "(3-5 questions) to assess the user's current knowledge level in "
            'the given skill. Return ONLY valid JSON with key "questions": '
            '[{"id": "<str>", "question": "<str>", "options": [<str>, ...]}], '
            "4-5 options per question."
        )
    elif role == AgentRole.WEAKNESS:
        base = (
            "You are an educational diagnostician. Evaluate the user's answers "
            "to a diagnostic quiz and identify knowledge gaps. Return ONLY valid JSON "
            "with keys: weak_areas (list of str), recommended_topics (list of str topic ids)."
        )
    else:  # SUMMARIZER
        base = (
            "You are a concise summarizer. Condense the conversation so far into "
            "a single short paragraph that preserves the learner's current topic, "
            "recent progress and open weak areas. Do NOT output JSON."
        )
    return base + LANGUAGE_INSTRUCTION


def _brief(session: Any | None) -> str:
    if session is None:
        return ""
    brief = getattr(session, "to_brief", None)
    if callable(brief):
        return str(brief())
    return str(getattr(session, "session_brief", "") or "")


def build_coach_prompt(
    session: Any | None = None,
    weakness_report: Any | None = None,
) -> list[dict[str, str]]:
    system = _system(AgentRole.COACH)
    brief = _brief(session)
    if brief:
        system = f"{SESSION_PREAMBLE}\n{brief}\n\n{system}"

    user_msg = "Build a roadmap now."
    if weakness_report is not None:
        weak_str = ", ".join(getattr(weakness_report, "weak_areas", []))
        rec_str = ", ".join(getattr(weakness_report, "recommended_topics", []))
        if weak_str or rec_str:
            user_msg += (
                f"\n\nDiagnostic identified weak areas: {weak_str or 'none'}.\n"
                f"Recommended topics: {rec_str or 'none'}.\n"
                "Prioritize these areas in your roadmap, ensuring prerequisites "
                "are respected (earlier topics should enable later ones)."
            )
    return [
        {"role": "system", "content": system},
        {"role": "user", "content": user_msg},
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

    user = f"Topic title: {topic.title}\nTopic description: {topic.description}\n{instruction}"
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
            f"- {_content(chunk)} (relevance {score}\n  )"
            for chunk in retrieved
            for score in [_attr(chunk, "score", 0.0)]
        )
        user += f"\n\n[Retrieved context]\n{context_block}"
    user += f"\n\nStudent's answer: {submission}"
    return [
        {"role": "system", "content": system},
        {"role": "user", "content": user},
    ]


def build_diagnostic_prompt(profile: UserProfile) -> list[dict[str, str]]:
    """Build the prompt for the diagnostic quiz generation."""
    system = _system(AgentRole.DIAGNOSTIC)
    user = (
        f"Skill: {profile.skill}\n"
        f"Current level: {profile.current_level.value}\n"
        f"Target level: {profile.target_level.value}\n"
        "Create 3-5 diagnostic questions with 4-5 options each to identify "
        "knowledge gaps the user should address before starting the learning roadmap."
    )
    return [
        {"role": "system", "content": system},
        {"role": "user", "content": user},
    ]


def build_weakness_prompt(
    profile: UserProfile,
    questions: Sequence[Any],
    answers: dict[str, str],
) -> list[dict[str, str]]:
    """Build the prompt for weakness evaluation from diagnostic answers."""
    system = _system(AgentRole.WEAKNESS)

    q_lines: list[str] = []
    for q in questions:
        q_id = _attr(q, "id")
        q_text = _attr(q, "question", "")
        options = _attr(q, "options", [])
        answer_idx = answers.get(q_id, "")
        answer_text = ""
        if answer_idx.isdigit() and options:
            idx = int(answer_idx) - 1
            if 0 <= idx < len(options):
                answer_text = options[idx]
        q_lines.append(f"Q: {q_text}\n  Options: {options}\n  Answer: {answer_text}")

    user = (
        f"Skill: {profile.skill}\n"
        f"Current level: {profile.current_level.value} -> {profile.target_level.value}\n"
        f"\nDiagnostic answers:\n" + "\n".join(q_lines)
    )
    return [
        {"role": "system", "content": system},
        {"role": "user", "content": user},
    ]


def _flatten(messages: Sequence[dict[str, str]]) -> str:
    parts: list[str] = []
    for msg in messages:
        role = msg.get("role", "user")
        parts.append(f"[{role}] {msg.get('content', '')}")
    return "\n".join(parts)


def build_summarizer_prompt(messages: Sequence[dict[str, str]]) -> list[dict[str, str]]:
    return [
        {"role": "system", "content": _system(AgentRole.SUMMARIZER)},
        {"role": "user", "content": "Conversation: " + _flatten(messages)},
    ]


def _attr(obj: Any, key: str, default: Any = None) -> Any:
    """Access attribute or dict key uniformly."""
    if isinstance(obj, dict):
        return obj.get(key, default)
    return getattr(obj, key, default)


def _content(chunk: RetrievedChunk) -> str:
    """Extract content from a RetrievedChunk."""
    return str(_attr(chunk, "content", ""))
