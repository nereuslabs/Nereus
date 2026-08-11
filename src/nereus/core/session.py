from __future__ import annotations

import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping

from pydantic import BaseModel, Field

from nereus.core.state import (
    Assessment,
    DiagnosticQuestion,
    Roadmap,
    UserProfile,
    WeaknessReport,
)


class LearningSession(BaseModel):
    """Aggregated, request-scoped record of the learning process.

    Stored in :class:`NereusState.session` and serialized by the LangGraph
    checkpointer alongside the rest of state. Feeds the ``SESSION CONTEXT``
    preamble into every LLM prompt.
    """

    model_config = {"arbitrary_types_allowed": True}

    user_profile: UserProfile | None = None
    roadmap: Roadmap = Field(default_factory=Roadmap)
    current_topic_index: int = 0
    completed: list[Assessment] = Field(default_factory=list)
    retry_counts: dict[str, int] = Field(default_factory=dict)
    aggregated_weak_areas: dict[str, int] = Field(default_factory=dict)
    last_material: str = ""
    last_task: str = ""
    last_submission: str | None = None

    # ------------------------------------------------------------------ #
    def note_weak_areas(self, areas: list[str]) -> None:
        for area in areas:
            self.aggregated_weak_areas[area] = self.aggregated_weak_areas.get(area, 0) + 1

    def record_assessment(self, assessment: Assessment) -> None:
        self.completed = [a for a in self.completed if a.topic_id != assessment.topic_id]
        self.completed.append(assessment)
        if assessment.verdict.value == "retry":
            self.retry_counts[assessment.topic_id] = (
                self.retry_counts.get(assessment.topic_id, 0) + 1
            )
        else:
            self.retry_counts[assessment.topic_id] = 0
        self.note_weak_areas(assessment.weak_areas)

    def _current_topic_title(self) -> str:
        topics = self.roadmap.topics
        if 0 <= self.current_topic_index < len(topics):
            return topics[self.current_topic_index].title
        return ""

    def to_brief(self) -> str:
        profile = self.user_profile
        if profile is None:
            profile_line = "Profile: not set"
        else:
            profile_line = (
                f"Skill: {profile.skill} | Level: {profile.current_level.value} "
                f"-> {profile.target_level.value} | {profile.hours_per_day}h/day, "
                f"{profile.deadline_days} days"
            )

        n = len(self.roadmap.topics)
        idx = self.current_topic_index
        if n:
            current_line = (
                f'Roadmap: {n} topics; current: {idx + 1}/{n} "{self._current_topic_title()}"'
            )
        else:
            current_line = "Roadmap: empty"

        if self.completed:
            completed_line = "Completed: " + ", ".join(
                f"{a.topic_id} {a.verdict.value} ({a.score:g})" for a in self.completed
            )
        else:
            completed_line = "Completed: none"

        if self.aggregated_weak_areas:
            weak_sorted = sorted(
                self.aggregated_weak_areas.items(), key=lambda kv: kv[1], reverse=True
            )
            weak_line = "Weak areas: " + ", ".join(
                f"{area} (x{count})" for area, count in weak_sorted
            )
        else:
            weak_line = "Weak areas: none"

        active_retries = {k: v for k, v in self.retry_counts.items() if v > 0}
        if active_retries:
            retry_line = "Retries: " + ", ".join(f"{t} x{n}" for t, n in active_retries.items())
        else:
            retry_line = "Retries: none"

        return f"{profile_line}\n{current_line}\n{completed_line}\n{weak_line}\n{retry_line}"

    # ------------------------------------------------------------------ #
    def update_from_state(
        self, state: Mapping[str, Any], *, own_output: Mapping[str, Any] | None = None
    ) -> "LearningSession":
        """Return a new session reflecting ``state`` (+ optional agent output).

        Used by agents after computing their own output so the session stays in
        sync with the post-step state without a dedicated graph node.
        """
        merged = dict(state)
        if own_output:
            merged.update(own_output)

        profile = merged.get("user_profile")
        roadmap = merged.get("roadmap") or self.roadmap
        current_index = merged.get("current_topic_index", self.current_topic_index)
        last_material = merged.get("material") or self.last_material
        last_task = merged.get("task") or self.last_task
        last_submission = merged.get("user_submission") or self.last_submission

        completed = list(self.completed)
        retry_counts = dict(self.retry_counts)
        weak_areas = dict(self.aggregated_weak_areas)

        assessment = merged.get("assessment")
        if isinstance(assessment, Assessment):
            completed = [a for a in completed if a.topic_id != assessment.topic_id]
            completed.append(assessment)
            if assessment.verdict.value == "retry":
                retry_counts[assessment.topic_id] = retry_counts.get(assessment.topic_id, 0) + 1
            else:
                retry_counts[assessment.topic_id] = 0
            for w in assessment.weak_areas:
                weak_areas[w] = weak_areas.get(w, 0) + 1

        return LearningSession(
            user_profile=profile if isinstance(profile, UserProfile) else None,
            roadmap=roadmap if isinstance(roadmap, Roadmap) else Roadmap(),
            current_topic_index=int(current_index),
            completed=completed,
            retry_counts=retry_counts,
            aggregated_weak_areas=weak_areas,
            last_material=last_material or "",
            last_task=last_task or "",
            last_submission=last_submission,
        )

    # ------------------------------------------------------------------ #
    # Persistence (#6, Step 4)                                          #
    def dump(self, path: str | Path, *, indent: int = 2) -> None:
        """Serialize the session to ``path`` as JSON (UTF-8).

        Used to checkpoint/restore a learning session between CLI runs so RAG
        progress is not lost on exit.
        """
        Path(path).write_text(self.model_dump_json(indent=indent), encoding="utf-8")

    @classmethod
    def load(cls, path: str | Path) -> "LearningSession":
        """Load a session previously written by :meth:`dump`."""
        return cls.model_validate_json(Path(path).read_text(encoding="utf-8"))


@dataclass
class UserSession:
    """On-disk session for a specific user (P1 — multi-user profiles).

    Unlike :class:`LearningSession` (an in-graph aggregated context), a
    ``UserSession`` is the *filesystem* record that survives a process restart:
    profile, roadmap, progress, and any diagnostic state.

    Files live at ``{SESSION_ROOT}/{user_id}/{session_id}.json``.
    """

    session_id: str
    user_id: str | None = None
    user_profile: UserProfile | None = None
    roadmap: Roadmap | None = None
    current_topic_index: int = 0
    retry_count: int = 0
    user_submission: str | None = None
    session_brief: str = ""
    diagnostic_questions: list[DiagnosticQuestion] = field(default_factory=list)
    user_diagnostic_answers: dict[str, str] | None = None
    weakness_report: WeaknessReport | None = None

    def __post_init__(self) -> None:
        if not self.session_id:
            self.session_id = uuid.uuid4().hex

    def to_state_dict(self) -> dict[str, Any]:
        """Convert to a dict suitable for seeding :class:`NereusState`."""
        from nereus.core.state import Assessment, Verdict  # noqa: F401

        state: dict[str, Any] = {
            "user_profile": self.user_profile,
            "roadmap": self.roadmap or Roadmap(),
            "current_topic_index": self.current_topic_index,
            "retry_count": self.retry_count,
            "session_brief": self.session_brief,
            "diagnostic_questions": self.diagnostic_questions,
            "user_diagnostic_answers": self.user_diagnostic_answers,
            "weakness_report": self.weakness_report,
            "messages": [],
        }
        if self.user_submission:
            state["user_submission"] = self.user_submission
        return state

    @classmethod
    def from_state(cls, state: Mapping[str, Any], session_id: str | None = None) -> "UserSession":
        """Capture the relevant NereusState fields into a UserSession."""
        profile = state.get("user_profile")
        roadmap = state.get("roadmap")
        return cls(
            session_id=session_id or uuid.uuid4().hex,
            user_profile=profile if isinstance(profile, UserProfile) else None,
            roadmap=roadmap if isinstance(roadmap, Roadmap) else None,
            current_topic_index=int(state.get("current_topic_index", 0)),
            retry_count=int(state.get("retry_count", 0)),
            user_submission=state.get("user_submission"),
            session_brief=state.get("session_brief", ""),
            diagnostic_questions=list(state.get("diagnostic_questions") or []),
            user_diagnostic_answers=state.get("user_diagnostic_answers"),
            weakness_report=state.get("weakness_report")
            if isinstance(state.get("weakness_report"), WeaknessReport)
            else None,
        )

    def to_json(self) -> str:
        from dataclasses import asdict

        data = asdict(self)
        # Pydantic models → dict
        for k, v in list(data.items()):
            if isinstance(v, BaseModel):
                data[k] = v.model_dump()
        import json

        return json.dumps(data, default=str)

    @classmethod
    def from_json(cls, raw: str, *, session_id: str | None = None) -> "UserSession":
        import json

        data = json.loads(raw)
        data["session_id"] = session_id or data.get("session_id") or uuid.uuid4().hex
        # Restore pydantic models
        if isinstance(data.get("user_profile"), dict):
            data["user_profile"] = UserProfile(**data["user_profile"])
        if isinstance(data.get("roadmap"), dict):
            data["roadmap"] = Roadmap(**data["roadmap"])
        if isinstance(data.get("weakness_report"), dict):
            data["weakness_report"] = WeaknessReport(**data["weakness_report"])
        if isinstance(data.get("diagnostic_questions"), list):
            data["diagnostic_questions"] = [
                DiagnosticQuestion(**q) if isinstance(q, dict) else q
                for q in data["diagnostic_questions"]
            ]
        return cls(**data)

    # ------------------------------------------------------------------ #
    def dump(self, path: str | Path) -> None:
        """Persist this session to *path* as JSON."""
        p = Path(path)
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(self.to_json(), encoding="utf-8")

    @classmethod
    def load(cls, path: str | Path) -> "UserSession | None":
        """Load a session from *path*, or ``None`` if the file doesn't exist.

        The ``session_id`` is preserved from the stored JSON, not derived from
        the filename stem.
        """
        p = Path(path)
        if not p.exists():
            return None
        return cls.from_json(p.read_text(encoding="utf-8"))


def session_path_for(user_id: str | None, session_id: str) -> Path:
    """Resolve the on-disk path for a session: ``SESSION_ROOT/user_id/session_id.json``.

    Falls back to ``SESSION_ROOT/session_id.json`` when no user_id is set.
    """
    from nereus.config.settings import settings

    root = Path(settings.session_root)
    if user_id:
        return root / user_id / f"{session_id}.json"
    return root / f"{session_id}.json"
