#!/usr/bin/env python3
"""Chunk 3.2 — steady bridge parity gate (HEC-RAS golden crates)."""

from __future__ import annotations

import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
LOG = ORACLE / "logs" / "chunk3_bridge_steady_latest.log"

BRIDGE_CRATES = [
    "bridge_abutment_hecras_verification",
    "bridge_bu_bd_hecras_verification",
    "bridge_high_flow_hecras_verification",
    "bridge_roadway_embankment_verification",
    "bridge_guide_bank_contraction_verification",
    "bridge_friction_weighting_hecras_verification",
    "bridge_reverse_flow_rating_verification",
    "bridge_opening_alignment_verification",
]


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

    log("=== Chunk 3.2 steady bridge gate ===")
    log(f"Repo: {ROOT}")
    log(f"Date: {datetime.now(timezone.utc).astimezone().isoformat(timespec='seconds')}")
    log()

    try:
        for crate in BRIDGE_CRATES:
            run_step(f"3.2 cargo {crate}", ["cargo", "test", "--test", crate])
            log(f"  {crate}: OK")

        run_step("3.2 bridge lib smoke", ["cargo", "test", "bridge", "--lib"])
        log("bridge --lib: OK")
    except subprocess.CalledProcessError as exc:
        log(f"\nFAILED: exit code {exc.returncode}")
        LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return exc.returncode or 1

    log()
    log("=== Chunk 3.2 bridge steady gate complete ===")
    log(f"Log: {LOG}")
    LOG.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
