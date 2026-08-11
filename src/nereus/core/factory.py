from __future__ import annotations

import logging

from nereus.core.graph import NereusGraph
from nereus.core.persistence import build_checkpointer
from nereus.llm.base import LLMProvider
from nereus.llm.factory import build_llm_provider
from nereus.llm.retriever import Retriever

logger = logging.getLogger("nereus")


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
    diagnostic=None,
    retriever: Retriever | None = None,
    session_path: str | None = None,
    session_id: str | None = None,
    user_id: str | None = None,
    run_diagnostic: bool | None = None,
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

    resolved_checkpointer = checkpointer if checkpointer is not None else build_checkpointer()
    from nereus.config.settings import settings

    if run_diagnostic is None:
        run_diagnostic = settings.run_diagnostic

    graph = NereusGraph(
        provider=provider,
        coach=coach,
        tutor=tutor,
        examiner=examiner,
        diagnostic=diagnostic,
        retriever=retriever,
        checkpointer=resolved_checkpointer,
        interactive=interactive,
        session_path=session_path,
        session_id=session_id,
        user_id=user_id,
        run_diagnostic=run_diagnostic,
    )

    logger.info(
        "Nereus graph ready; retriever=%s checkpointer=%s session=%s diagnostic=%s",
        type(graph._retriever).__name__
        if getattr(graph, "_retriever", None) is not None
        else "n/a",
        settings.checkpoint_backend,
        graph._session_path or "disabled",
        run_diagnostic,
    )
    return graph
