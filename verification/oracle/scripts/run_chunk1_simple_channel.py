#!/usr/bin/env python3
"""Chunk 1 verify gate — simple_channel friction-slope DS (WSL/Linux)."""

from __future__ import annotations

import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
LOG = ORACLE / "logs" / "chunk1_simple_channel_latest.log"
SCENARIO = ORACLE / "scenarios" / "simple_channel_unsteady_linked.json"


def run_step(label: str, cmd: list[str]) -> None:
    print(f"\n--- {label} ---")
    print(" ".join(cmd))
    subprocess.run(cmd, cwd=ROOT, check=True)


def main() -> int:
    LOG.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = []

    def log(msg: str = "") -> None:
        print(msg)
        lines.append(msg)

    log("=== Chunk 1 verify gate — simple_channel friction-slope DS ===")
    log(f"Repo: {ROOT}")
    log(f"Date: {datetime.now(timezone.utc).astimezone().isoformat(timespec='seconds')}")
    log()

    py = sys.executable

    log("--- stream1d import ---")
    try:
        import stream1d  # noqa: F401

        log(f"stream1d: {stream1d.__file__}")
    except ImportError:
        log("stream1d: MISSING — run: maturin develop --features python")
        LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return 1

    try:
        run_step("smoke_simple_channel_parse", [py, str(ORACLE / "scripts" / "smoke_simple_channel_parse.py")])
        log("smoke: OK")

        run_step(
            "run_ras_reference --skip-ras-run --verify",
            [
                py,
                str(ORACLE / "scripts" / "run_ras_reference.py"),
                "--scenario",
                str(SCENARIO),
                "--skip-ras-run",
                "--verify",
            ],
        )
        log("verify: OK")
    except subprocess.CalledProcessError as exc:
        log(f"\nFAILED: exit code {exc.returncode}")
        LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return exc.returncode or 1

    log()
    log("=== Chunk 1 complete ===")
    log(f"Log: {LOG}")
    LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
