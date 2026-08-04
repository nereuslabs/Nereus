from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class AgentRole(str, Enum):
    COACH = "coach"
    TUTOR = "tutor"
    EXAMINER = "examiner"
    SUMMARIZER = "summarizer"


@dataclass(frozen=True)
class ModelParams:
    temperature: float = 0.2
    max_tokens: int = 4096
    top_p: float = 0.95
    repeat_penalty: float = 1.1
    json_mode: bool = False


def _env_float(name: str, default: float) -> float:
    import os

    raw = os.environ.get(name)
    if not raw:
        return default
    try:
        return float(raw)
    except ValueError:
        return default


def _env_int(name: str, default: int) -> int:
    import os

    raw = os.environ.get(name)
    if not raw:
        return default
    try:
        return int(raw)
    except ValueError:
        return default


def role_params(role: AgentRole) -> ModelParams:
    """Per-role generation defaults, overridable via environment."""
    temp_override = _env_float("OLLAMA_TEMPERATURE", -1.0)
    max_override = _env_int("OLLAMA_MAX_TOKENS", -1)

    defaults: dict[AgentRole, ModelParams] = {
        AgentRole.COACH: ModelParams(
            temperature=0.5, max_tokens=2048, json_mode=True
        ),
        AgentRole.TUTOR: ModelParams(
            temperature=0.6, max_tokens=4096, json_mode=True
        ),
        AgentRole.EXAMINER: ModelParams(
            temperature=0.0, max_tokens=1024, json_mode=True
        ),
        AgentRole.SUMMARIZER: ModelParams(
            temperature=0.1, max_tokens=512, json_mode=False
        ),
    }
    params = defaults[role]
    kwargs: dict = {}
    if 0.0 <= temp_override <= 1.0:
        kwargs["temperature"] = temp_override
    if max_override > 0:
        kwargs["max_tokens"] = max_override
    if kwargs:
        params = ModelParams(
            temperature=kwargs.get("temperature", params.temperature),
            max_tokens=kwargs.get("max_tokens", params.max_tokens),
            top_p=params.top_p,
            repeat_penalty=params.repeat_penalty,
            json_mode=params.json_mode,
        )
    return params


def params_for(role: AgentRole) -> ModelParams:
    return role_params(role)
