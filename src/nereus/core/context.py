from __future__ import annotations

from typing import TYPE_CHECKING, Mapping, Sequence

if TYPE_CHECKING:
    from nereus.llm.inference import StructuredInferenceClient

_APPROX_TOKENS_PER_MSG = 100
_APPROX_CHARS_PER_TOKEN = 4


def _as_dict(msg: Mapping[str, object] | object) -> dict:
    if isinstance(msg, dict):
        return {"role": str(msg.get("role", "user")), "content": str(msg.get("content", ""))}
    role = getattr(msg, "type", None) or getattr(msg, "role", "user")
    content = getattr(msg, "content", "")
    if isinstance(content, (list, tuple)):
        parts = [str(c) if isinstance(c, str) else str(getattr(c, "text", c)) for c in content]
        content = "".join(parts)
    return {"role": str(role), "content": str(content)}


def approx_token_count(messages: Sequence[Mapping[str, object] | object]) -> int:
    total = 0
    for msg in messages:
        d = _as_dict(msg)
        total += max(_APPROX_TOKENS_PER_MSG, len(d["content"]) // _APPROX_CHARS_PER_TOKEN)
    return total


def _is_system(msg: Mapping[str, object] | object) -> bool:
    return _as_dict(msg)["role"] == "system"


def truncate_messages(
    messages: Sequence[Mapping[str, object] | object], max_tokens: int
) -> list[dict]:
    norms = [_as_dict(m) for m in messages]
    if not norms or approx_token_count(norms) <= max_tokens:
        return norms
    systems = [m for m in norms if m["role"] == "system"]
    rest = [m for m in norms if m["role"] != "system"]
    kept: list[dict] = []
    tokens = sum(approx_token_count([m]) for m in systems)
    for msg in reversed(rest):
        cost = approx_token_count([msg])
        if tokens + cost > max_tokens and kept:
            break
        kept.append(msg)
        tokens += cost
    kept.reverse()
    return systems + kept


def summarize_history(
    messages: Sequence[Mapping[str, object] | object],
    inference: "StructuredInferenceClient | None",
    max_tokens: int,
) -> list[dict]:
    if approx_token_count(messages) <= max_tokens:
        return [_as_dict(m) for m in messages]
    if inference is not None:
        from nereus.llm.params import AgentRole
        from nereus.llm.prompts import build_summarizer_prompt
        from nereus.llm.schema import SummaryOutput

        recent = truncate_messages(messages, max_tokens // 2)
        try:
            result = inference.generate(
                build_summarizer_prompt(recent),
                role=AgentRole.SUMMARIZER,
                output_model=SummaryOutput,
            )
            summary = str(result.summary) or ""
        except Exception:
            summary = ""
        if summary:
            return [
                {"role": "system", "content": f"[History summary]\n{summary}"},
                *recent,
            ]
    return truncate_messages(messages, max_tokens)
