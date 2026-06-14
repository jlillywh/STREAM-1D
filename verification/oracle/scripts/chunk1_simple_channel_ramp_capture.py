#!/usr/bin/env python3
"""
Extract WSEL timeseries reference from HEC-RAS Plan 04/05 HDF for ramp scenarios.

Usage:
  py -3 verification/oracle/scripts/chunk1_simple_channel_ramp_capture.py --plan 04 \\
    --hdf C:\\Users\\jason\\Documents\\hecras_testing\\simple_channel\\simple_channel.p04.hdf

  py -3 verification/oracle/scripts/chunk1_simple_channel_ramp_capture.py --plan 05 \\
    --hdf ...\\simple_channel.p05.hdf --verify
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
ORACLE = ROOT / "verification" / "oracle"

SCENARIOS = {
    "04": ORACLE / "scenarios" / "simple_channel_ramp_unsteady_linked.json",
    "05": ORACLE / "scenarios" / "simple_channel_ramp_rating_unsteady_linked.json",
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", choices=("04", "05"), required=True)
    parser.add_argument("--hdf", type=Path, required=True, help="Path to plan HDF after GUI compute")
    parser.add_argument(
        "--verify",
        action="store_true",
        help="Run WSL/STREAM-1D linked verify after extracting reference",
    )
    parser.add_argument(
        "--no-update-u02",
        action="store_true",
        help="Skip writing Observed HWM into u04/u05",
    )
    args = parser.parse_args()

    scenario = SCENARIOS[args.plan]
    hdf = args.hdf.resolve()
    if not hdf.is_file():
        print(f"ERROR: HDF not found: {hdf}", file=sys.stderr)
        return 1

    cmd = [
        sys.executable,
        str(ORACLE / "scripts" / "run_ras_reference.py"),
        "--scenario",
        str(scenario),
        "--skip-ras-run",
        "--hdf",
        str(hdf),
        "--plan",
        args.plan,
        "--no-verify",
    ]
    if args.no_update_u02:
        cmd.append("--no-update-u02")

    print(" ".join(cmd))
    result = subprocess.run(cmd, cwd=ROOT)
    if result.returncode != 0:
        return result.returncode

    if args.verify:
        verify_cmd = cmd[:]
        # Replace tail with verify-only invocation
        verify_cmd = [
            sys.executable,
            str(ORACLE / "scripts" / "run_ras_reference.py"),
            "--scenario",
            str(scenario),
            "--skip-ras-run",
            "--verify",
        ]
        print("\n=== STREAM-1D verify ===")
        print(" ".join(verify_cmd))
        return subprocess.run(verify_cmd, cwd=ROOT).returncode
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
