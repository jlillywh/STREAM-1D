#!/usr/bin/env python3
"""Phase 0 — no HEC-RAS verify gate (WSL/Linux)."""

from __future__ import annotations

import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
LOG = ORACLE / "logs" / "phase0_latest.log"


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

    log("=== Phase 0 verify gate ===")
    log(f"Repo: {ROOT}")
    log(f"Date: {datetime.now(timezone.utc).astimezone().isoformat(timespec='seconds')}")
    log()

    py = sys.executable

    log("--- 0.0 stream1d import ---")
    try:
        import stream1d  # noqa: F401

        log(f"stream1d: {stream1d.__file__}")
        log("0.0.1 stream1d: OK")
    except ImportError:
        log("0.0.1 stream1d: MISSING — run: maturin develop --features python")
        LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return 1

    try:
        run_step(
            "0.1 smoke_reach_mild_parse",
            [py, str(ORACLE / "scripts" / "smoke_reach_mild_parse.py")],
        )
        log("0.1: OK")

        run_step(
            "0.2 run_ras_reference --skip-ras-run --verify",
            [
                py,
                str(ORACLE / "scripts" / "run_ras_reference.py"),
                "--scenario",
                str(ORACLE / "scenarios" / "reach_mild_unsteady_linked.json"),
                "--skip-ras-run",
                "--verify",
            ],
        )
        log("0.2: OK")

        run_step(
            "0.3 run_oracle.sh (linked verify)",
            [
                "bash",
                str(ORACLE / "run_oracle.sh"),
                "--scenario",
                str(ORACLE / "scenarios" / "reach_mild_unsteady_linked.json"),
            ],
        )
        log("0.3: OK")
    except subprocess.CalledProcessError as exc:
        log(f"\nFAILED: exit code {exc.returncode}")
        LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return exc.returncode or 1

    log()
    log("=== Phase 0 complete ===")
    log(f"Log: {LOG}")
    LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
