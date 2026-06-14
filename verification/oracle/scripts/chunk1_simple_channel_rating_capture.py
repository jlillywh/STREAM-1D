#!/usr/bin/env python3
"""Capture simple_channel Plan 03 HDF (rating-curve DS) into reference."""

from __future__ import annotations

import argparse
import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))

from lib.hdf_paths import format_hdf_search_report, resolve_plan_hdf  # noqa: E402
from lib.stage_paths import hecras_stage_dir  # noqa: E402

ROOT = ORACLE.parents[1]
PROJECT = "simple_channel"
REPO_PROJECT = ORACLE / "projects" / PROJECT
DEFAULT_STAGE = hecras_stage_dir(PROJECT)
SCENARIO = ORACLE / "scenarios" / "simple_channel_rating_unsteady_linked.json"
DEFAULT_RAS = Path(r"C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe")
PLAN_KEY = "03"


def _run(cmd: list[str], *, cwd: Path = ROOT) -> int:
    print(" ".join(cmd))
    return subprocess.call(cmd, cwd=cwd)


def _stream1d_available() -> bool:
    try:
        import stream1d  # noqa: F401

        return True
    except ImportError:
        return False


def _repo_root_for_wsl(root: Path) -> str:
    text = str(root.resolve()).replace("/", "\\")
    marker = "\\wsl.localhost\\"
    lower = text.lower()
    if marker in lower:
        rest = text[lower.index(marker) + len(marker) :]
        _distro, *parts = rest.split("\\")
        return "/" + "/".join(p for p in parts if p)
    result = subprocess.run(
        ["wsl", "wslpath", "-u", text],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode == 0 and result.stdout.strip():
        return result.stdout.strip()
    return "/home/jason/Lillywhite_Consulting/lillywhite_engine/STREAM-1D"


def _run_verify(scenario: Path) -> int:
    script = ORACLE / "scripts" / "run_ras_reference.py"
    if _stream1d_available():
        return _run(
            [
                sys.executable,
                str(script),
                "--scenario",
                str(scenario),
                "--skip-ras-run",
                "--verify",
            ]
        )
    wsl_root = _repo_root_for_wsl(ROOT)
    rel_scenario = scenario.relative_to(ROOT).as_posix()
    wsl_cmd = (
        f"cd {shlex.quote(wsl_root)} && "
        f"PYTHONPATH=python python3 verification/oracle/scripts/run_ras_reference.py "
        f"--scenario {shlex.quote(rel_scenario)} --skip-ras-run --verify"
    )
    print("stream1d not available in Windows Python — running verify in WSL...")
    return _run(["wsl", "-d", "Ubuntu", "bash", "-lc", wsl_cmd])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hdf", type=Path, default=None, help="Explicit path to plan HDF")
    parser.add_argument("--stage-dir", type=Path, default=DEFAULT_STAGE)
    parser.add_argument("--ras-exe", type=Path, default=DEFAULT_RAS)
    parser.add_argument(
        "--run-ras",
        action="store_true",
        help="Run HEC-RAS Plan 03 headless on staged project, then capture",
    )
    args = parser.parse_args()

    if sys.platform != "win32":
        print("ERROR: capture must run on Windows.", file=sys.stderr)
        return 1

    os.environ["HECRAS_RAS_EXE"] = str(args.ras_exe)
    script = ORACLE / "scripts" / "run_ras_reference.py"

    if args.run_ras:
        print("=== Headless HEC-RAS Plan 03 + capture ===\n")
        rc = _run(
            [
                sys.executable,
                str(script),
                "--scenario",
                str(SCENARIO),
                "--plan",
                PLAN_KEY,
                "--no-verify",
            ]
        )
        if rc != 0:
            return rc
        args.hdf = resolve_plan_hdf(
            PROJECT,
            PLAN_KEY,
            stage_dir=args.stage_dir,
            repo_dir=REPO_PROJECT,
        )

    hdf = resolve_plan_hdf(
        PROJECT,
        PLAN_KEY,
        explicit=args.hdf,
        stage_dir=args.stage_dir,
        repo_dir=REPO_PROJECT,
    )
    print("=== Chunk 1 capture — simple_channel rating-curve HDF ===")
    print(f"HDF: {hdf or '(not found)'}\n")

    if hdf is None:
        print(
            format_hdf_search_report(
                PROJECT,
                PLAN_KEY,
                explicit=args.hdf,
                stage_dir=args.stage_dir,
                repo_dir=REPO_PROJECT,
            ),
            file=sys.stderr,
        )
        return 1

    print("Extract terminal WSEL -> reference JSON...")
    rc = _run(
        [
            sys.executable,
            str(script),
            "--scenario",
            str(SCENARIO),
            "--skip-ras-run",
            "--hdf",
            str(hdf),
            "--plan",
            PLAN_KEY,
            "--no-verify",
        ]
    )
    if rc != 0:
        return rc

    repo_hdf = REPO_PROJECT / f"{PROJECT}.p{PLAN_KEY}.hdf"
    shutil.copy2(hdf, repo_hdf)
    print(f"Copied HDF -> {repo_hdf}")

    stage_u03 = args.stage_dir / f"{PROJECT}.u03"
    repo_u03 = REPO_PROJECT / f"{PROJECT}.u03"
    if stage_u03.is_file() and stage_u03.resolve() != repo_u03.resolve():
        shutil.copy2(stage_u03, repo_u03)
        print(f"Copied GUI u03 -> {repo_u03}")

    print("\nRe-run verify...")
    return _run_verify(SCENARIO)


if __name__ == "__main__":
    raise SystemExit(main())
