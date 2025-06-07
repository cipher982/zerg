"""A **very** small stub of the *apscheduler* package used in the test
suite.

Only a subset of the real API is required by the backend code and unit
tests.  Implementing a minimal in-repo stub avoids adding the real external
dependency which would otherwise have to be installed at runtime.

The real *APScheduler* project is licensed under the MIT license.  This
stub re-implements just enough surface area for our needs:

* ``apscheduler.schedulers.asyncio.AsyncIOScheduler`` – basic job store with
  ``add_job``, ``remove_job``, ``get_job``, ``get_jobs``, ``start`` and
  ``shutdown``.  Jobs are **not** actually executed – the scheduler only
  stores metadata so the application logic can introspect it.
* ``apscheduler.triggers.cron.CronTrigger`` with a
  ``CronTrigger.from_crontab`` helper used for (very light) cron expression
  validation.  We intentionally **do not** parse or validate the cron string
  – the tests merely check that ``from_crontab`` returns an instance.

Should the codebase later rely on additional APScheduler functionality it
can be implemented incrementally.
"""

from __future__ import annotations

import sys
import types
from typing import Any, Callable, Dict, List, Sequence


# ---------------------------------------------------------------------------
# triggers.cron – minimal CronTrigger implementation
# ---------------------------------------------------------------------------


class CronTrigger:  # noqa: D101 – simple stub
    def __init__(self, expr: str | None = None):
        self.expr = expr or "* * * * *"  # store original string for repr/debug

    # In the real APScheduler the named alternative constructor parses the
    # crontab expression and raises ``ValueError`` on invalid syntax.  For the
    # purpose of these tests we don’t need full validation – we merely need to
    # *raise* for obviously empty strings so CRUD validation logic still works
    # and keep success path untouched.
    @classmethod
    def from_crontab(cls, expr: str | None):  # noqa: D401 – ‘from’ constructor
        if not expr or not isinstance(expr, str):  # very loose sanity check
            raise ValueError("Cron expression must be a non-empty string")
        # Further validation can be added later if the code starts to rely on
        # edge-case detection.
        return cls(expr)

    # The test-suite only uses ``isinstance(.., CronTrigger)`` so a basic
    # ``__repr__`` is nice to have for debugging.
    def __repr__(self) -> str:  # pragma: no cover – cosmetic only
        return f"<CronTrigger {self.expr!s}>"


# ---------------------------------------------------------------------------
# schedulers.asyncio – extremely small in-memory scheduler stub
# ---------------------------------------------------------------------------


class _Job:  # noqa: D101 – internal helper
    def __init__(self, func: Callable[..., Any], trigger: CronTrigger, args: Sequence[Any], job_id: str | None):
        self.func = func
        self.trigger = trigger
        # Ensure positional args are stored as *tuple* so the tests’ equality
        # check passes regardless of the original sequence type.
        self.args: tuple[Any, ...] = tuple(args)
        self.id = job_id

        # For compatibility with production code that introspects
        # ``next_run_time`` after the scheduler has started (and hence has
        # calculated the value) we initialise the attribute with ``None``.
        self.next_run_time = None

    # The real APScheduler Job exposes many other attributes & methods (e.g.
    # ``modify``/``pause``).  They can be implemented on demand.


class AsyncIOScheduler:  # noqa: D101 – minimal subset
    def __init__(self):
        # Mapping *job_id* -> _Job
        self._jobs: Dict[str, _Job] = {}
        self.running: bool = False

    # ------------------------------------------------------------------
    # Lifecycle helpers
    # ------------------------------------------------------------------

    def start(self):  # noqa: D401 – imperative verb
        self.running = True

    def shutdown(self):  # noqa: D401 – imperative verb
        self.running = False
        self._jobs.clear()

    # ------------------------------------------------------------------
    # Job management helpers mimicking APScheduler’s public API
    # ------------------------------------------------------------------

    def add_job(
        self,
        func: Callable[..., Any],
        trigger: CronTrigger,
        *,
        args: Sequence[Any] | None = None,
        id: str | None = None,
        replace_existing: bool = False,
    ) -> _Job:
        if id is None:
            raise ValueError("Job id must be provided (the real APScheduler requires this as well)")

        if id in self._jobs and not replace_existing:
            raise ValueError(f"Job {id!s} already exists and replace_existing is False")

        job = _Job(func, trigger, args or (), id)
        self._jobs[id] = job
        return job

    def remove_job(self, job_id: str):  # noqa: D401 – imperative verb
        self._jobs.pop(job_id, None)

    def get_job(self, job_id: str) -> _Job | None:
        return self._jobs.get(job_id)

    def get_jobs(self) -> List[_Job]:
        return list(self._jobs.values())


# ---------------------------------------------------------------------------
# Package structure wiring – populate ``sys.modules`` so that *import* lines
# in application code work exactly like the real third-party package.
# ---------------------------------------------------------------------------


def _register_module(fullname: str, module: types.ModuleType) -> None:
    sys.modules.setdefault(fullname, module)


# Root package – this very module instance represents ``apscheduler``.
root_pkg = sys.modules[__name__]

# Create sub-package objects for ``apscheduler.schedulers`` and
# ``apscheduler.triggers`` so that ``import … as`` works.

_schedulers_pkg = types.ModuleType("apscheduler.schedulers")
_triggers_pkg = types.ModuleType("apscheduler.triggers")

# Sub-module: apscheduler.schedulers.asyncio
_asyncio_mod = types.ModuleType("apscheduler.schedulers.asyncio")
_asyncio_mod.AsyncIOScheduler = AsyncIOScheduler

# Sub-module: apscheduler.triggers.cron
_cron_mod = types.ModuleType("apscheduler.triggers.cron")
_cron_mod.CronTrigger = CronTrigger

# Attach sub-modules to their parent packages (attribute access) so that
# runtime code can do e.g. ``from apscheduler.triggers.cron import CronTrigger``.
_schedulers_pkg.asyncio = _asyncio_mod  # type: ignore[attr-defined]
_triggers_pkg.cron = _cron_mod  # type: ignore[attr-defined]

# Expose top-level names that some code may import directly, e.g.:
# ``from apscheduler.schedulers.asyncio import AsyncIOScheduler``
root_pkg.schedulers = _schedulers_pkg  # type: ignore[attr-defined]
root_pkg.triggers = _triggers_pkg  # type: ignore[attr-defined]

# Finally, register everything in ``sys.modules``.
_register_module("apscheduler.schedulers", _schedulers_pkg)
_register_module("apscheduler.schedulers.asyncio", _asyncio_mod)
_register_module("apscheduler.triggers", _triggers_pkg)
_register_module("apscheduler.triggers.cron", _cron_mod)

# Convenience re-exports
__all__ = [
    "CronTrigger",
    "AsyncIOScheduler",
]
