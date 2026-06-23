#!/usr/bin/env python3
"""
Stage simple_channel ramp scenarios (u04/p04 friction, u05/p05 rating) for HEC-RAS GUI.

Usage:
  py -3 verification/oracle/scripts/chunk1_simple_channel_ramp_prep.py
  py -3 verification/oracle/scripts/chunk1_simple_channel_ramp_prep.py --plan 05 --open-ras
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))

from lib.ras_headless import stage_project_for_hecras  # noqa: E402
from lib.stage_paths import hecras_stage_dir  # noqa: E402

PROJECT = ORACLE / "projects" / "simple_channel"
WRITE_SCRIPT = ORACLE / "scripts" / "write_simple_channel_ramp.py"


def _run_write(plan: str) -> None:
    cmd = [sys.executable, str(WRITE_SCRIPT), "--set-prj-plan", plan]
    print(" ".join(cmd))
    subprocess.run(cmd, check=True, cwd=ROOT)


def _open_ras(stage_dir: Path) -> None:
    prj = stage_dir / "simple_channel.prj"
    if not prj.is_file():
        print(f"WARNING: project file not found: {prj}", file=sys.stderr)
        return
    if sys.platform == "win32":
        os_startfile = getattr(__import__("os"), "startfile", None)
        if os_startfile:
            os_startfile(str(prj))
            print(f"Opened {prj}")
            return
    print(f"Open in HEC-RAS: {prj}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--plan",
        choices=("04", "05", "both"),
        default="04",
        help="Which plan to activate in simple_channel.prj (default: 04 friction ramp)",
    )
    parser.add_argument(
        "--open-ras",
        action="store_true",
        help="Open staged simple_channel.prj in HEC-RAS (Windows only)",
    )
    args = parser.parse_args()

    if args.plan == "both":
        _run_write("04")
    else:
        _run_write(args.plan)

    stage_dir, _ = stage_project_for_hecras(PROJECT)
    print(f"\nStaged project: {stage_dir}")
    print("Next steps:")
    if args.plan in ("04", "both"):
        print("  1. HEC-RAS → Plan 04 (ramp Q, friction-slope DS) → Compute")
        print("     HDF: simple_channel.p04.hdf")
    if args.plan in ("05", "both"):
        print("  2. HEC-RAS → Plan 05 (ramp Q, rating-curve DS) → Compute")
        print("     HDF: simple_channel.p05.hdf")
    print(
        "  3. Capture reference:\n"
        "     py -3 verification/oracle/scripts/chunk1_simple_channel_ramp_capture.py "
        "--plan 04 --hdf <path-to-p04.hdf>"
    )

    if args.open_ras:
        _open_ras(hecras_stage_dir("simple_channel"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
