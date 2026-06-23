#!/usr/bin/env python3
"""Capture simple_channel HDF after GUI run into committed reference."""

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

from lib.repo_paths import repo_root_for_wsl  # noqa: E402
from lib.stage_paths import hecras_stage_dir  # noqa: E402

ROOT = ORACLE.parents[1]
PROJECT = "simple_channel"
REPO_PROJECT = ORACLE / "projects" / PROJECT
DEFAULT_STAGE = hecras_stage_dir(PROJECT)
SCENARIO = ORACLE / "scenarios" / "simple_channel_unsteady_linked.json"
DEFAULT_RAS = Path(r"C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe")


def _run(cmd: list[str], *, cwd: Path = ROOT) -> int:
    print(" ".join(cmd))
    return subprocess.call(cmd, cwd=cwd)


def _stream1d_available() -> bool:
    try:
        import stream1d  # noqa: F401

        return True
    except ImportError:
        return False


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
    wsl_root = repo_root_for_wsl(ROOT)
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
    parser.add_argument("--hdf", type=Path, default=None)
    parser.add_argument("--stage-dir", type=Path, default=DEFAULT_STAGE)
    parser.add_argument("--ras-exe", type=Path, default=DEFAULT_RAS)
    args = parser.parse_args()

    if sys.platform != "win32":
        print("ERROR: Chunk 1 capture must run on Windows.", file=sys.stderr)
        return 1

    hdf = args.hdf or (args.stage_dir / f"{PROJECT}.p01.hdf")
    print("=== Chunk 1 capture — simple_channel HDF to reference ===")
    print(f"HDF: {hdf}\n")

    if not hdf.is_file():
        print("ERROR: HDF not found. Complete Plan 01 compute in HEC-RAS GUI first.", file=sys.stderr)
        print(f"  Expected: {hdf}", file=sys.stderr)
        print(
            f"  Or stage default: {hecras_stage_dir(PROJECT) / f'{PROJECT}.p01.hdf'}",
            file=sys.stderr,
        )
        return 1

    os.environ["HECRAS_RAS_EXE"] = str(args.ras_exe)
    script = ORACLE / "scripts" / "run_ras_reference.py"

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
            "--no-verify",
        ]
    )
    if rc != 0:
        return rc

    repo_hdf = REPO_PROJECT / f"{PROJECT}.p01.hdf"
    shutil.copy2(hdf, repo_hdf)
    print(f"Copied HDF -> {repo_hdf}")

    stage_u02 = args.stage_dir / f"{PROJECT}.u02"
    repo_u02 = REPO_PROJECT / f"{PROJECT}.u02"
    if stage_u02.is_file() and stage_u02.resolve() != repo_u02.resolve():
        shutil.copy2(stage_u02, repo_u02)
        print(f"Copied GUI u02 -> {repo_u02}")

    print("\nRe-run verify...")
    return _run_verify(SCENARIO)


if __name__ == "__main__":
    raise SystemExit(main())
