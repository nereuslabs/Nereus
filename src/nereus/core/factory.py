from __future__ import annotations

import logging

from langgraph.checkpoint.memory import MemorySaver
from langgraph.checkpoint.serde.jsonplus import JsonPlusSerializer

from nereus.core.graph import NereusGraph
from nereus.llm.base import LLMProvider
from nereus.llm.factory import build_llm_provider

logger = logging.getLogger("nereus")

# Pydantic models / enums stored in NereusState must be allow-listed for
# LangGraph's msgpack checkpointer (avoids warnings and is forward-compatible
# with LANGGRAPH_STRICT_MSGPACK=true).
_ALLOWED_MSGPCK = [
    ("nereus.core.state", "UserProfile"),
    ("nereus.core.state", "Roadmap"),
    ("nereus.core.state", "RoadmapTopic"),
    ("nereus.core.state", "Assessment"),
    ("nereus.core.state", "Verdict"),
    ("nereus.core.state", "UserLevel"),
    ("nereus.core.state", "LearningStatus"),
    ("nereus.core.session", "LearningSession"),
]


def _default_checkpointer() -> MemorySaver:
    serde = JsonPlusSerializer(allowed_msgpack_modules=_ALLOWED_MSGPCK)
    return MemorySaver(serde=serde)


_DEFAULT_CHECKPOINTER = _default_checkpointer()


def _provider_info(provider: LLMProvider) -> str:
    base = provider.__class__.__name__
    extras: list[str] = []
    if hasattr(provider, "model"):
        extras.append(f"model={provider.model!r}")
    if hasattr(provider, "base_url"):
        extras.append(f"base_url={provider.base_url!r}")
    if hasattr(provider, "timeout"):
        extras.append(f"timeout={provider.timeout!r}")
    return base + (f"({', '.join(extras)})" if extras else "")


def build_nereus_graph(
    *,
    interactive: bool = False,
    checkpointer=None,
    provider: LLMProvider | None = None,
    coach=None,
    tutor=None,
    examiner=None,
) -> NereusGraph:
    """Centralized factory for a :class:`NereusGraph`.

    Single entry point used by ``main.py`` and the tests. The LLM backend is
    resolved through :func:`nereus.llm.factory.build_llm_provider` (driven by
    ``LLM_PROVIDER`` in the environment) unless an already-built provider is
    injected, which keeps the graph fully testable with a stub.
    """
    if provider is None:
        provider = build_llm_provider()

    logger.info("Nereus provider resolved: %s", _provider_info(provider))
    if provider.__class__.__name__ == "OllamaProvider":
        try:
            import urllib.error
            import urllib.request

            urllib.request.urlopen(f"{provider.base_url}/api/tags", timeout=3.0)
            logger.info("Ollama endpoint reachable: %s", provider.base_url)
        except Exception as exc:  # noqa: BLE001
            logger.warning(
                "Ollama endpoint unreachable at %s (%s); agents will fall back "
                "to deterministic stubs where possible.",
                getattr(provider, "base_url", None),
                exc,
            )

    resolved_checkpointer = checkpointer if checkpointer is not None else _DEFAULT_CHECKPOINTER
    return NereusGraph(
        provider=provider,
        coach=coach,
        tutor=tutor,
        examiner=examiner,
        checkpointer=resolved_checkpointer,
        interactive=interactive,
    )
