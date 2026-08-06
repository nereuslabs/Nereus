from __future__ import annotations

from pathlib import Path
from typing import Any, Mapping

from pydantic import BaseModel, Field

from nereus.core.state import Assessment, Roadmap, UserProfile


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
