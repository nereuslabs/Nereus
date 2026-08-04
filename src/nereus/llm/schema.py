from __future__ import annotations

import json
import re
from typing import Any, TypeVar

from pydantic import BaseModel, Field, ValidationError

T = TypeVar("T", bound=BaseModel)


def extract_json(text: str, *, verbose: bool = False) -> dict[str, Any]:
    """Best-effort extraction of a JSON object from an LLM reply.

    Ollama models frequently wrap JSON in ```json ... ``` fences or prefix it
    with prose. This helper tries the whole payload first, then a fenced block,
    then the first balanced ``{...}`` slice.
    """
    candidates: list[str] = [text]

    fenced = re.search(r"```(?:json)?\s*(.*?)```", text, flags=re.DOTALL)
    if fenced:
        candidates.append(fenced.group(1))

    for candidate in candidates:
        stripped = candidate.strip()
        try:
            parsed = json.loads(stripped)
            if isinstance(parsed, dict):
                return parsed
        except json.JSONDecodeError:
            continue

    for i, char in enumerate(text):
        if char == "{":
            depth = 0
            for j in range(i, len(text)):
                if text[j] == "{":
                    depth += 1
                elif text[j] == "}":
                    depth -= 1
                    if depth == 0:
                        try:
                            parsed = json.loads(text[i : j + 1])
                            if isinstance(parsed, dict):
                                return parsed
                        except json.JSONDecodeError:
                            break
    if verbose:
        return {"_raw": text}
    raise ValueError(f"Could not extract JSON from LLM response: {text[:200]!r}")


def parse_structured(raw: str, model: type[T]) -> T:
    """Parse an LLM reply into a validated Pydantic model.

    Tries strict ``model_validate`` first; on failure attempts JSON extraction
    from fenced/prose output. Raises ``ValidationError``/``ValueError`` if the
    output cannot be coerced into ``model``.
    """
    try:
        return model.model_validate(json.loads(raw))
    except (json.JSONDecodeError, ValidationError, TypeError):
        data = extract_json(raw, verbose=False)
        return model.model_validate(data)


# --------------------------------------------------------------------------- #
# Pydantic contracts for LLM responses                                      #
# --------------------------------------------------------------------------- #
class RoadmapTopicOutput(BaseModel):
    id: str
    title: str
    description: str


class RoadmapOutput(BaseModel):
    topics: list[RoadmapTopicOutput]


class MaterialOutput(BaseModel):
    material: str
    task: str


class AssessmentOutput(BaseModel):
    score: float = Field(ge=0.0, le=100.0)
    feedback: str
    weak_areas: list[str] = Field(default_factory=list)


class SummaryOutput(BaseModel):
    summary: str

