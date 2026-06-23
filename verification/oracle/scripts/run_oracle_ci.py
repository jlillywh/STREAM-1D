#!/usr/bin/env python3
"""Linked oracle CI gate — no HEC-RAS install required."""

from __future__ import annotations

import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
LOG = ORACLE / "logs" / "oracle_ci_latest.log"


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

    log("=== Linked oracle CI gate ===")
    log(f"Repo: {ROOT}")
    log(f"Date: {datetime.now(timezone.utc).astimezone().isoformat(timespec='seconds')}")
    log()

    py = sys.executable

    log("--- stream1d import ---")
    try:
        import stream1d  # noqa: F401

        log(f"stream1d: {stream1d.__file__}")
        log("stream1d: OK")
    except ImportError:
        log("stream1d: MISSING — run: maturin develop --features python")
        LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return 1

    try:
        run_step(
            "smoke_reach_mild_parse",
            [py, str(ORACLE / "scripts" / "smoke_reach_mild_parse.py")],
        )
        log("smoke_reach_mild_parse: OK")

        run_step(
            "run_ras_reference --skip-ras-run --verify",
            [
                py,
                str(ORACLE / "scripts" / "run_ras_reference.py"),
                "--scenario",
                str(ORACLE / "scenarios" / "reach_mild_unsteady_linked.json"),
                "--skip-ras-run",
                "--verify",
            ],
        )
        log("reference verify: OK")

        run_step(
            "run_oracle.sh (linked verify)",
            [
                "bash",
                str(ORACLE / "run_oracle.sh"),
                "--scenario",
                str(ORACLE / "scenarios" / "reach_mild_unsteady_linked.json"),
            ],
        )
        log("linked verify: OK")

        run_step(
            "conspan mode4 ramp matrix (CI gate)",
            [
                py,
                str(ORACLE / "run_linked_verify.py"),
                "--scenario",
                str(ORACLE / "scenarios" / "conspan_unsteady_ramp_matrix_mode4.json"),
                "--format",
                "matrix",
            ],
        )
        log("conspan mode4 ramp matrix: OK")

        run_step(
            "smoke_simple_channel_parse",
            [py, str(ORACLE / "scripts" / "smoke_simple_channel_parse.py")],
        )
        log("smoke_simple_channel_parse: OK")

        run_step(
            "simple_channel constant Q (linked verify)",
            [
                py,
                str(ORACLE / "run_linked_verify.py"),
                "--scenario",
                str(ORACLE / "scenarios" / "simple_channel_unsteady_linked.json"),
            ],
        )
        log("simple_channel constant Q: OK")

        run_step(
            "smoke_simple_channel_ramp_parse",
            [py, str(ORACLE / "scripts" / "smoke_simple_channel_ramp_parse.py")],
        )
        log("smoke_simple_channel_ramp_parse: OK")

        run_step(
            "simple_channel Q ramp (linked verify)",
            [
                py,
                str(ORACLE / "run_linked_verify.py"),
                "--scenario",
                str(ORACLE / "scenarios" / "simple_channel_ramp_unsteady_linked.json"),
            ],
        )
        log("simple_channel Q ramp: OK")
    except subprocess.CalledProcessError as exc:
        log(f"\nFAILED: exit code {exc.returncode}")
        LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return exc.returncode or 1

    log()
    log("=== Oracle CI gate complete ===")
    log(f"Log: {LOG}")
    LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
