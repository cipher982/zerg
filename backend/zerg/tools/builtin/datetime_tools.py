"""Date and time related tools."""

import datetime as dt
from typing import Optional

from zerg.tools.registry import register_tool


@register_tool(name="get_current_time", description="Get the current date and time in ISO-8601 format")
def get_current_time() -> str:
    """Return the current date/time as ISO-8601 string.

    Returns:
        Current datetime in ISO-8601 format (e.g., "2025-05-25T17:30:00.123456")
    """
    return dt.datetime.now().isoformat()


@register_tool(name="datetime_diff", description="Calculate the difference between two dates/times")
def datetime_diff(start_time: str, end_time: str, unit: Optional[str] = "seconds") -> float:
    """Calculate the difference between two datetime strings.

    Args:
        start_time: Start datetime in ISO-8601 format
        end_time: End datetime in ISO-8601 format
        unit: Unit for the result - "seconds", "minutes", "hours", or "days"

    Returns:
        The difference between the two times in the specified unit

    Raises:
        ValueError: If datetime strings cannot be parsed or unit is invalid
    """
    try:
        start_dt = dt.datetime.fromisoformat(start_time.replace("Z", "+00:00"))
        end_dt = dt.datetime.fromisoformat(end_time.replace("Z", "+00:00"))
    except ValueError as e:
        raise ValueError(f"Invalid datetime format: {e}")

    diff = end_dt - start_dt
    total_seconds = diff.total_seconds()

    if unit == "seconds":
        return total_seconds
    elif unit == "minutes":
        return total_seconds / 60
    elif unit == "hours":
        return total_seconds / 3600
    elif unit == "days":
        return total_seconds / 86400
    else:
        raise ValueError(f"Invalid unit '{unit}'. Must be one of: seconds, minutes, hours, days")
