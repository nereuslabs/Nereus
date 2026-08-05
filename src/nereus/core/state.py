from __future__ import annotations

from enum import Enum
from typing import Annotated, Any, Optional, TypedDict

from langgraph.graph import add_messages
from pydantic import BaseModel, Field


class UserLevel(str, Enum):
    BEGINNER = "beginner"
    INTERMEDIATE = "intermediate"
    ADVANCED = "advanced"


class LearningStatus(str, Enum):
    COACHING = "coaching"
    LEARNING = "learning"
    EXAMINING = "examining"
    COMPLETED = "completed"


class Verdict(str, Enum):
    PASS = "pass"
    RETRY = "retry"


class UserProfile(BaseModel):
    skill: str
    current_level: UserLevel
    target_level: UserLevel
    hours_per_day: float = Field(ge=0.0)
    deadline_days: int = Field(ge=1)
    goal: str


class RoadmapTopic(BaseModel):
    id: str
    title: str
    description: str


class Roadmap(BaseModel):
    topics: list[RoadmapTopic] = Field(default_factory=list)


class Assessment(BaseModel):
    topic_id: str
    score: float = Field(ge=0.0, le=100.0)
    verdict: Verdict
    feedback: str = ""
    weak_areas: list[str] = Field(default_factory=list)


class RetrievedChunk(BaseModel):
    """A single passage retrieved from the RAG store for the current topic."""

    topic_id: str
    content: str
    score: float = 0.0


class NereusState(TypedDict, total=False):
    user_profile: Optional[UserProfile]
    roadmap: Roadmap
    current_topic_index: int

    material: str
    task: str
    user_submission: Optional[str]

    assessment: Optional[Assessment]

    retry_count: int
    max_retries: int

    status: str

    # Aggregated session context (filled in by agents, see core/session.py).
    session: Optional[Any]
    session_brief: str

    # RAG context (filled in by the retriever node, see core/graph.py).
    retrieved_chunks: Optional[list[RetrievedChunk]]

    messages: Annotated[list, add_messages]