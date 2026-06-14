#!/usr/bin/env python3
"""Chunk 4 exit gate — ConSpan mild unsteady mode 0 baseline."""

from __future__ import annotations

import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
LOG_DIR = ORACLE / "logs"
LOG_DIR.mkdir(parents=True, exist_ok=True)
LOG_PATH = LOG_DIR / "chunk4_verify_latest.log"


def _run(label: str, cmd: list[str]) -> None:
    print(f"--- {label} ---")
    subprocess.run(cmd, cwd=ROOT, check=True)
    print(f"{label}: OK\n")


def main() -> int:
    started = datetime.now(timezone.utc).isoformat()
    print("=== Chunk 4 — ConSpan unsteady mode 0 gate ===")
    print(f"Repo: {ROOT}")
    print(f"Date: {started}\n")

    _run(
        "4.1 smoke_conspan_unsteady_parse",
        [sys.executable, str(ORACLE / "scripts" / "smoke_conspan_unsteady_parse.py")],
    )
    _run(
        "4.2 linked verify conspan_unsteady_mild_linked",
        [
            sys.executable,
            str(ORACLE / "run_linked_verify.py"),
            "--scenario",
            str(ORACLE / "scenarios" / "conspan_unsteady_mild_linked.json"),
        ],
    )

    print("=== Chunk 4 gate complete ===")
    print(f"Log: {LOG_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
