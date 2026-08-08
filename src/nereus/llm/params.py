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


def role_params(role: AgentRole) -> ModelParams:
    """Per-role generation defaults.

    The legacy ``OLLAMA_TEMPERATURE`` / ``OLLAMA_MAX_TOKENS`` env overrides were
    dropped together with the legacy Ollama provider (#46 / Y1); use the real
    provider's own configuration to tune sampling.
    """
    defaults: dict[AgentRole, ModelParams] = {
        AgentRole.COACH: ModelParams(temperature=0.5, max_tokens=2048, json_mode=True),
        AgentRole.TUTOR: ModelParams(temperature=0.6, max_tokens=4096, json_mode=True),
        AgentRole.EXAMINER: ModelParams(temperature=0.0, max_tokens=1024, json_mode=True),
        AgentRole.SUMMARIZER: ModelParams(temperature=0.1, max_tokens=512, json_mode=False),
    }
    return defaults[role]


def params_for(role: AgentRole) -> ModelParams:
    return role_params(role)
