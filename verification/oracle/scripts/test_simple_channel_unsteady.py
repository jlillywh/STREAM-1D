#!/usr/bin/env python3
"""Run simple trapezoidal channel unsteady parity (HEC-RAS ref vs STREAM-1D)."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCENARIO = ROOT / "verification/oracle/scenarios/simple_channel_unsteady_linked.json"


def main() -> int:
    cmd = [
        sys.executable,
        str(ROOT / "verification/oracle/run_linked_verify.py"),
        "--scenario",
        str(SCENARIO),
    ]
    print(" ".join(cmd))
    return subprocess.call(cmd, cwd=ROOT)


if __name__ == "__main__":
    raise SystemExit(main())
