#!/usr/bin/env python3
"""
Refresh HEC-RAS plan HDF reference for a linked unsteady scenario, then optionally verify.

**Recommended:** run on native Windows (PowerShell) with HEC-RAS installed.
Daily STREAM-1D compare can run from WSL/Linux without HEC-RAS using the committed
reference JSON (``--skip-ras-run --verify``).

Requirements:
  pip install -r verification/requirements-oracle-hecras.txt
  maturin develop --features python

Environment (optional):
  HECRAS_RAS_EXE   Full path to Ras.exe
  HECRAS_VERSION   RAS version for ras-commander

Examples:
  # Windows — headless RAS + refresh reference + verify
  python verification/oracle/scripts/run_ras_reference.py \\
    --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json

  # WSL/Linux — compare only (no HEC-RAS)
  python verification/oracle/scripts/run_ras_reference.py \\
    --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json \\
    --skip-ras-run --verify

  # Refresh reference from an existing plan HDF
  python verification/oracle/scripts/run_ras_reference.py \\
    --scenario verification/oracle/scenarios/reach_mild_unsteady_linked.json \\
    --skip-ras-run --hdf verification/oracle/projects/reach_mild/reach_mild.p02.hdf
"""

from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ROOT / "python"))

from lib.hecras_geom_parser import parse_g01  # noqa: E402
from lib.ras_preflight import validate_linked_unsteady_project  # noqa: E402
from lib.ras_headless import (  # noqa: E402
    RasHeadlessError,
    TerminalWselCheckpoint,
    checkpoints_to_reference_doc,
    extract_terminal_wsel_at_rms,
    extract_wsel_timeseries_at_rms,
    hecras_available,
    is_wsl,
    run_plan_headless,
    timeseries_checkpoints_to_reference_doc,
    update_u02_observed_hwm,
    write_reference_json,
)
from lib.scenario import load_scenario  # noqa: E402


def _reference_json_path(scenario) -> Path:
    ref = scenario.raw.get("reference", {})
    rel = ref.get("hdf_extract") or ref.get("file")
    if rel and (ref.get("hdf_extract") or not str(rel).endswith(".u02")):
        return (scenario.oracle_root / rel).resolve()
    return scenario.linked_project_dir() / f"reference_wsel_{scenario.id}.json"


def _plan_hdf_default(scenario, plan_key: str) -> Path:
    stem = scenario.linked_project_dir().name
    return scenario.linked_project_dir() / f"{stem}.p{plan_key}.hdf"


def _river_reach(scenario) -> tuple[str, str]:
    files = scenario.linked_files()
    geom = parse_g01(files["geometry"])
    if not geom.cross_sections:
        raise RasHeadlessError(f"No cross sections in {files['geometry']}")
    xs = geom.cross_sections[0]
    return xs.river.strip(), xs.reach.strip()


def _repo_root_for_wsl_verify() -> str:
    """Linux path to repo root when verify must run under WSL from Windows."""
    root = ROOT.resolve()
    if sys.platform != "win32":
        return str(root)
    parts = [p for p in str(root).replace("/", "\\").split("\\") if p]
    for idx, part in enumerate(parts):
        if part.lower() == "home":
            return "/" + "/".join(parts[idx:])
    return str(root)


def _run_linked_verify(scenario_path: Path) -> int:
    scenario_path = scenario_path.resolve()
    print("\n=== STREAM-1D linked verify ===")
    if sys.platform == "win32":
        wsl_root = _repo_root_for_wsl_verify()
        try:
            rel = scenario_path.relative_to(ROOT.resolve())
            wsl_scenario = f"{wsl_root}/{rel.as_posix()}"
        except ValueError:
            wsl_scenario = scenario_path.as_posix()
        cmd = [
            "wsl",
            "-e",
            "bash",
            "-lc",
            (
                f"cd {shlex.quote(wsl_root)} && "
                f"PYTHONPATH=python python3 verification/oracle/run_linked_verify.py "
                f"--scenario {shlex.quote(wsl_scenario)}"
            ),
        ]
        print(" ".join(cmd))
        print("(Windows: verify runs in WSL — native extension is Linux-built)")
        return subprocess.call(cmd, cwd=ROOT)

    cmd = [
        sys.executable,
        str(ORACLE / "run_linked_verify.py"),
        "--scenario",
        str(scenario_path),
    ]
    env = os.environ.copy()
    python_pkg = str(ROOT / "python")
    prev = env.get("PYTHONPATH", "")
    env["PYTHONPATH"] = f"{python_pkg}{os.pathsep}{prev}" if prev else python_pkg
    print(" ".join(cmd))
    return subprocess.call(cmd, cwd=ROOT, env=env)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--scenario",
        type=Path,
        required=True,
        help="Linked scenario manifest JSON",
    )
    parser.add_argument(
        "--skip-ras-run",
        action="store_true",
        help="Do not invoke HEC-RAS; extract from existing plan HDF only",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help="Run run_linked_verify.py after reference refresh (default when RAS runs)",
    )
    parser.add_argument(
        "--no-verify",
        action="store_true",
        help="Skip linked verify after reference refresh",
    )
    parser.add_argument(
        "--hdf",
        type=Path,
        default=None,
        help="Override plan HDF path (implies --skip-ras-run if not set with RAS run)",
    )
    parser.add_argument(
        "--plan",
        default=None,
        help="Plan number (default: from scenario linked_project.plan_number)",
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
        "--clear-geompre",
        action="store_true",
        default=True,
        help="Force geometry preprocessor before unsteady run (default: on)",
    )
    parser.add_argument(
        "--no-clear-geompre",
        action="store_false",
        dest="clear_geompre",
        help="Skip forced geometry preprocessor",
    )
    parser.add_argument(
        "--no-update-u02",
        action="store_true",
        help="Do not write Observed HWM lines into bundled u02",
    )
    parser.add_argument(
        "--preflight-only",
        action="store_true",
        help="Check ras-commander + Ras.exe availability and exit",
    )
    args = parser.parse_args()

    scenario_path = args.scenario.resolve()
    if not scenario_path.is_file():
        print(f"ERROR: scenario not found: {scenario_path}", file=sys.stderr)
        return 2

    scenario = load_scenario(scenario_path)
    if scenario.mode != "unsteady":
        print(f"ERROR: scenario mode must be unsteady, got {scenario.mode!r}", file=sys.stderr)
        return 2

    ok, msg = hecras_available()
    print(f"HEC-RAS preflight: {msg}")

    linked = scenario.raw["linked_project"]
    plan_key = (args.plan or scenario.plan_number()).zfill(2)
    project_dir = scenario.linked_project_dir()
    files = scenario.linked_files()
    plan_path = files.get("plan") or (project_dir / f"{project_dir.name}.p{plan_key}")
    u02_path = files.get("unsteady_flow")
    if u02_path is None:
        print("ERROR: scenario linked_project.unsteady_flow is required for preflight", file=sys.stderr)
        return 2
    project_errors = validate_linked_unsteady_project(
        project_dir,
        plan_path=plan_path,
        u02_path=u02_path,
    )
    if project_errors:
        print("\nERROR: linked project preflight failed:", file=sys.stderr)
        for err in project_errors:
            print(f"  - {err}", file=sys.stderr)
        return 2
    print("Linked project preflight: ok (hydrograph ordinates match plan simulation span)")

    if args.preflight_only:
        if not ok:
            print(
                "Warning: HEC-RAS Ras.exe not available — project checks passed but "
                "headless runs will fail until HECRAS_RAS_EXE is set.",
                file=sys.stderr,
            )
        return 0

    ref_path = _reference_json_path(scenario)
    compare = scenario.raw.get("compare", {})
    checkpoints_rm = [float(rm) for rm in compare.get("checkpoints_rm", [])]
    if not checkpoints_rm:
        print("ERROR: scenario compare.checkpoints_rm is empty", file=sys.stderr)
        return 2

    run_verify = args.verify or (not args.no_verify and not args.skip_ras_run and args.hdf is None)
    if args.no_verify:
        run_verify = False

    extract_ref = not args.skip_ras_run or args.hdf is not None
    hdf_path = args.hdf or _plan_hdf_default(scenario, plan_key)

    if not args.skip_ras_run and args.hdf is None:
        if not ok:
            print(
                "\nERROR: Headless HEC-RAS is not available.\n"
                "Install HEC-RAS on Windows, pip install ras-commander, set HECRAS_RAS_EXE.\n"
                "Or use --skip-ras-run --verify to compare against the committed reference only.",
                file=sys.stderr,
            )
            return 2
        if is_wsl():
            print(
                "\nNote: WSL detected — project will be staged to Windows LOCALAPPDATA.\n"
                "For fewer path issues, run this script from Windows PowerShell instead.",
                flush=True,
            )
        print("\n=== HEC-RAS headless run ===", flush=True)
        try:
            hdf_path = run_plan_headless(
                project_dir,
                plan_key,
                ras_version=args.ras_version,
                ras_exe=args.ras_exe,
                clear_geompre=args.clear_geompre,
            )
        except RasHeadlessError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 1
        print(f"Plan HDF: {hdf_path}")

    if extract_ref:
        if not hdf_path.is_file():
            print(f"ERROR: plan HDF not found: {hdf_path}", file=sys.stderr)
            return 1

        river, reach = _river_reach(scenario)
        time_checkpoints_hr = [
            float(h) for h in compare.get("time_checkpoints_hr", [])
        ]
        if time_checkpoints_hr:
            print("\n=== Extract WSEL timeseries from HDF ===")
            try:
                ts_checkpoints = extract_wsel_timeseries_at_rms(
                    hdf_path,
                    checkpoints_rm,
                    time_checkpoints_hr,
                    river=river,
                    reach=reach,
                )
            except RasHeadlessError as exc:
                print(f"ERROR: {exc}", file=sys.stderr)
                return 1
            for c in ts_checkpoints:
                print(
                    f"  RM {c.rm:.3f}  hour {c.hour:g}: {c.wsel_ft:.4f} ft  "
                    f"(RS={c.station}, {c.river}/{c.reach})"
                )
            hecras_version = os.environ.get("HECRAS_VERSION") or args.ras_version
            hours_label = ", ".join(f"{h:g}" for h in time_checkpoints_hr)
            doc = timeseries_checkpoints_to_reference_doc(
                ts_checkpoints,
                source=(
                    f"HEC-RAS plan {plan_key} WSEL timeseries "
                    f"(hours {hours_label}) from {hdf_path.name}"
                ),
                time_checkpoints_hr=time_checkpoints_hr,
                hdf_path=hdf_path,
            )
            if hecras_version:
                doc["hecras_version"] = hecras_version
            write_reference_json(ref_path, doc)
            print(f"Wrote reference: {ref_path}")
            max_hour = max(time_checkpoints_hr)
            terminal = [
                TerminalWselCheckpoint(
                    rm=c.rm,
                    wsel_ft=c.wsel_ft,
                    river=c.river,
                    reach=c.reach,
                    station=c.station,
                )
                for c in ts_checkpoints
                if float(c.hour) == float(max_hour)
            ]
        else:
            print("\n=== Extract terminal WSEL from HDF ===")
            try:
                checkpoints = extract_terminal_wsel_at_rms(
                    hdf_path,
                    checkpoints_rm,
                    river=river,
                    reach=reach,
                )
            except RasHeadlessError as exc:
                print(f"ERROR: {exc}", file=sys.stderr)
                return 1

            for c in checkpoints:
                print(f"  RM {c.rm:.3f}: {c.wsel_ft:.4f} ft  (RS={c.station}, {c.river}/{c.reach})")

            hecras_version = os.environ.get("HECRAS_VERSION") or args.ras_version
            doc = checkpoints_to_reference_doc(
                checkpoints,
                source=f"HEC-RAS plan {plan_key} terminal WSEL from {hdf_path.name}",
                hdf_path=hdf_path,
            )
            if hecras_version:
                doc["hecras_version"] = hecras_version
            write_reference_json(ref_path, doc)
            print(f"Wrote reference: {ref_path}")
            terminal = checkpoints

        u02 = scenario.linked_files().get("unsteady_flow")
        if u02 and u02.is_file() and not args.no_update_u02 and terminal:
            update_u02_observed_hwm(u02, terminal, river=river, reach=reach)
            print(f"Updated Observed HWM in {u02}")
    elif args.skip_ras_run:
        print(f"Skipping HDF extract (using committed reference: {ref_path})")

    if run_verify:
        return _run_linked_verify(scenario_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
