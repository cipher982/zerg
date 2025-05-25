"""HTTP-related tools for making web requests."""

import json
import logging
from typing import Any
from typing import Dict
from typing import Optional
from urllib.parse import urlencode

import httpx

from zerg.tools.registry import register_tool

logger = logging.getLogger(__name__)


@register_tool(name="http_get", description="Make an HTTP GET request to a URL and return the response")
def http_get(
    url: str,
    params: Optional[Dict[str, str]] = None,
    headers: Optional[Dict[str, str]] = None,
    timeout: Optional[float] = 30.0,
) -> Dict[str, Any]:
    """Make an HTTP GET request.

    Args:
        url: The URL to request
        params: Optional query parameters as a dictionary
        headers: Optional HTTP headers as a dictionary
        timeout: Request timeout in seconds (default: 30)

    Returns:
        Dictionary containing:
        - status_code: HTTP status code
        - headers: Response headers as a dict
        - body: Response body (as text or parsed JSON if applicable)
        - error: Error message if request failed

    Example:
        >>> http_get("https://api.example.com/data", params={"key": "value"})
        {"status_code": 200, "headers": {...}, "body": {...}}
    """
    try:
        # Build URL with params if provided
        if params:
            url = f"{url}?{urlencode(params)}"

        # Default headers
        default_headers = {"User-Agent": "Zerg-Agent/1.0"}
        if headers:
            default_headers.update(headers)

        # Make the request
        with httpx.Client() as client:
            response = client.get(url, headers=default_headers, timeout=timeout, follow_redirects=True)

        # Prepare response data
        result = {
            "status_code": response.status_code,
            "headers": dict(response.headers),
            "url": str(response.url),  # Final URL after redirects
        }

        # Try to parse JSON response
        content_type = response.headers.get("content-type", "").lower()
        if "application/json" in content_type:
            try:
                result["body"] = response.json()
            except json.JSONDecodeError:
                result["body"] = response.text
        else:
            result["body"] = response.text

        # Truncate very long responses
        if isinstance(result["body"], str) and len(result["body"]) > 10000:
            result["body"] = result["body"][:10000] + "... (truncated)"
            result["truncated"] = True

        return result

    except httpx.TimeoutException:
        logger.error(f"HTTP GET timeout for URL: {url}")
        return {"status_code": 0, "error": f"Request timed out after {timeout} seconds", "url": url}
    except httpx.RequestError as e:
        logger.error(f"HTTP GET error for URL {url}: {e}")
        return {"status_code": 0, "error": f"Request failed: {str(e)}", "url": url}
    except Exception as e:
        logger.exception(f"Unexpected error in http_get for URL: {url}")
        return {"status_code": 0, "error": f"Unexpected error: {str(e)}", "url": url}
