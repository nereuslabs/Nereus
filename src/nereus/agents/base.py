from __future__ import annotations

from abc import ABC, abstractmethod

from nereus.core.state import NereusState


class BaseAgent(ABC):
    """Interface for all Nereus agents.

    Agents receive the current graph state and return a partial state update.
    """

    @abstractmethod
    def run(self, state: NereusState) -> dict:
        """Execute the agent against the given state and return updates."""
