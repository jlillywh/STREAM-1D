#!/usr/bin/env python3
"""
Automated HEC-RAS vs STREAM-1D parity for the simple trapezoidal channel.

Runs HEC-RAS plan 01 headlessly (ras-commander), extracts terminal WSEL from
the plan HDF, updates the oracle reference, and runs linked verify.

Requirements (Windows host with HEC-RAS 6.x installed):
  pip install ras-commander
  maturin develop --features python

Environment (optional):
  HECRAS_RAS_EXE  Full path to Ras.exe (e.g. C:/Program Files/HEC/HEC-RAS/6.6/Ras.exe)
  HECRAS_VERSION  RAS version string passed to init_ras_project (e.g. 6.6)

Usage:
  python3 verification/oracle/scripts/run_simple_channel_hecras_parity.py
  python3 verification/oracle/scripts/run_simple_channel_hecras_parity.py --skip-ras-run
  python3 verification/oracle/scripts/run_simple_channel_hecras_parity.py --preflight-only
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ROOT / "python"))

from lib.ras_headless import (  # noqa: E402
    RasHeadlessError,
    checkpoints_to_reference_doc,
    extract_terminal_wsel_at_rms,
    hecras_available,
    is_wsl,
    run_plan_headless,
    write_reference_json,
    update_u02_observed_hwm,
)

PROJECT = ORACLE / "projects" / "simple_channel"
SCENARIO = ORACLE / "scenarios" / "simple_channel_unsteady_linked.json"
REF_PATH = PROJECT / "reference_wsel_simple_channel_unsteady.json"
U02_PATH = PROJECT / "simple_channel.u02"
CHECKPOINTS_RM = [3.0, 2.0, 1.0, 0.0]
RIVER = "Simple Creek"
REACH = "Trapezoid Reach"


def _default_hdf_path() -> Path:
    return PROJECT / "simple_channel.p01.hdf"


def _run_linked_verify() -> int:
    cmd = [
        sys.executable,
        str(ORACLE / "run_linked_verify.py"),
        "--scenario",
        str(SCENARIO),
    ]
    print("\n=== STREAM-1D linked verify ===")
    print(" ".join(cmd))
    return subprocess.call(cmd, cwd=ROOT)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--plan",
        default="01",
        help="HEC-RAS plan number (default: 01)",
    )
    parser.add_argument(
        "--skip-ras-run",
        action="store_true",
        help="Use existing plan HDF; only extract reference + compare",
    )
    parser.add_argument(
        "--hdf",
        type=Path,
        default=None,
        help="Override plan HDF path (with --skip-ras-run)",
    )
    parser.add_argument(
        "--ras-version",
        default=None,
        help="HEC-RAS version for ras-commander (or set HECRAS_VERSION)",
    )
    parser.add_argument(
        "--ras-exe",
        default=None,
        help="Full path to Ras.exe (or set HECRAS_RAS_EXE)",
    )
    parser.add_argument(
        "--num-cores",
        type=int,
        default=1,
        help="HEC-RAS compute cores (default: 1)",
    )
    parser.add_argument(
        "--clear-geompre",
        action="store_true",
        help="Force geometry preprocessor before unsteady run",
    )
    parser.add_argument(
        "--no-update-u02",
        action="store_true",
        help="Do not write Observed HWM lines into u02",
    )
    parser.add_argument(
        "--preflight-only",
        action="store_true",
        help="Check ras-commander + Ras.exe availability and exit",
    )
    args = parser.parse_args()

    ok, msg = hecras_available()
    print(f"HEC-RAS preflight: {msg}")
    if args.preflight_only:
        return 0 if ok else 2
    if not args.skip_ras_run and not ok:
        print(
            "\nERROR: Headless HEC-RAS is not available on this machine.\n"
            "Install HEC-RAS 6.x on Windows, pip install ras-commander, and set HECRAS_RAS_EXE if needed.\n"
            "From WSL, run this script with Windows Python against the same repo path.",
            file=sys.stderr,
        )
        return 2

    hdf_path = args.hdf or _default_hdf_path()

    if not args.skip_ras_run:
        if is_wsl():
            print(
                "\nNote: WSL detected — project will be staged to Windows LOCALAPPDATA "
                "before HEC-RAS runs (avoids UNC/Linux path issues).",
                flush=True,
            )
        print("\n=== HEC-RAS headless run ===", flush=True)
        try:
            hdf_path = run_plan_headless(
                PROJECT,
                args.plan,
                ras_version=args.ras_version,
                ras_exe=args.ras_exe,
                num_cores=args.num_cores,
                clear_geompre=args.clear_geompre,
            )
        except RasHeadlessError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 1
        print(f"Plan HDF: {hdf_path}")

    if not hdf_path.is_file():
        print(f"ERROR: plan HDF not found: {hdf_path}", file=sys.stderr)
        return 1

    print("\n=== Extract terminal WSEL from HDF ===")
    try:
        checkpoints = extract_terminal_wsel_at_rms(
            hdf_path,
            CHECKPOINTS_RM,
            river=RIVER,
            reach=REACH,
        )
    except RasHeadlessError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    for c in checkpoints:
        print(f"  RM {c.rm:.1f}: {c.wsel_ft:.4f} ft  (RS={c.station}, {c.river}/{c.reach})")

    doc = checkpoints_to_reference_doc(
        checkpoints,
        source=f"HEC-RAS plan {args.plan} terminal WSEL from {hdf_path.name}",
        hdf_path=hdf_path,
    )
    write_reference_json(REF_PATH, doc)
    print(f"Wrote reference: {REF_PATH}")

    if not args.no_update_u02:
        update_u02_observed_hwm(U02_PATH, checkpoints, river=RIVER, reach=REACH)
        print(f"Updated Observed HWM in {U02_PATH}")

    return _run_linked_verify()


if __name__ == "__main__":
    raise SystemExit(main())
