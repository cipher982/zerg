"""
Legacy AgentManager API entrypoint (delegates to legacy_agent_manager).

This module is a shim for backward compatibility.  All functionality is
delegated to `legacy_agent_manager.py` and will be removed in a future release.
"""

# Ensure patches to ChatOpenAI in this shim propagate to the legacy module
import importlib as _importlib

from .legacy_agent_manager import *  # noqa: F403,F401

_legacy = _importlib.import_module(__package__ + ".legacy_agent_manager")
# Sync ChatOpenAI binding
_legacy.ChatOpenAI = ChatOpenAI  # type: ignore[name-defined]

__all__ = [
    "AgentManager",
    "get_current_time",
    "AgentManagerState",
    "END",
    "START",
    "StateGraph",
    "add_messages",
]
