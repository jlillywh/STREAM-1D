#!/usr/bin/env python3
"""Smoke test: run HEC-RAS plan 02 on full ConSpan project (validates u02/plan pairing)."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))

from lib.ras_headless import RasHeadlessError, hecras_available, run_plan_headless  # noqa: E402

PROJECT = ORACLE / "projects" / "conspan"


def main() -> int:
    ok, msg = hecras_available()
    if not ok:
        print(f"SKIP: {msg}", file=sys.stderr)
        return 2
    print(f"Running ConSpan plan 02 from {PROJECT}", flush=True)
    try:
        hdf = run_plan_headless(PROJECT, "02")
    except RasHeadlessError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    print(f"OK: {hdf}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
