from __future__ import annotations

import json
import re
from typing import Any


def extract_json(text: str) -> dict[str, Any]:
    """Best-effort extraction of a JSON object from an LLM reply.

    Ollama models frequently wrap JSON in ```json ... ``` fences or prefix it
    with prose. This helper tries the whole payload first, then a fenced block,
    then the first balanced `{...}` slice.
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
    raise ValueError(f"Could not extract JSON from LLM response: {text[:200]!r}")
