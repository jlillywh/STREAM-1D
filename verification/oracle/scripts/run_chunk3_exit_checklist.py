#!/usr/bin/env python3
"""Chunk 3 exit criteria checklist — runs after sub-gates, prints formal sign-off."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]

CRITERIA = [
    (
        "Steady culvert crates + linked ConSpan steady oracle",
        [sys.executable, str(ORACLE / "scripts" / "run_chunk3_culvert_steady_gate.py")],
    ),
    (
        "Steady bridge HEC-RAS verification crates",
        [sys.executable, str(ORACLE / "scripts" / "run_chunk3_bridge_steady_gate.py")],
    ),
    (
        "g01 mapping (deck, piers, BU/BD, ineffective, no duplicate obstruction)",
        [sys.executable, str(ORACLE / "scripts" / "run_chunk3_g01_mapping_gate.py")],
    ),
]


def main() -> int:
    print("=== Chunk 3 exit criteria ===\n")
    for idx, (label, cmd) in enumerate(CRITERIA, start=1):
        print(f"[{idx}/{len(CRITERIA)}] {label}")
        try:
            subprocess.run(cmd, cwd=ROOT, check=True)
            print(f"  PASS\n")
        except subprocess.CalledProcessError as exc:
            print(f"  FAIL (exit {exc.returncode})\n")
            return exc.returncode or 1

    print("Chunk 3 exit criteria: ALL PASS")
    print("- Required steady verification crates green")
    print("- Linked steady ConSpan PASS (via 3.1 culvert gate)")
    print("- g01 parser coverage for linked scenarios (Beaver + ConSpan)")
    print("- No duplicate pier/obstruction on reach parent XS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
