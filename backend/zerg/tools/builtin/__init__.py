"""Built-in tools for Zerg agents.

This module contains the standard tools that come with the platform.
All tools in this module are automatically registered when imported.
"""

# Import all built-in tools to trigger registration
from zerg.tools.builtin.datetime_tools import datetime_diff
from zerg.tools.builtin.datetime_tools import get_current_time
from zerg.tools.builtin.http_tools import http_get
from zerg.tools.builtin.math_tools import math_eval
from zerg.tools.builtin.uuid_tools import generate_uuid

__all__ = [
    "get_current_time",
    "datetime_diff",
    "http_get",
    "math_eval",
    "generate_uuid",
]
