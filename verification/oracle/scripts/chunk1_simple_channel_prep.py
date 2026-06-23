#!/usr/bin/env python3
"""Chunk 1 prep — stage simple_channel for HEC-RAS GUI (friction-slope DS)."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))

from lib.hecras_text_io import assert_compact_ras_text, copy_ras_text_file, is_ras_text_file  # noqa: E402
from lib.patch_simple_channel_u02 import patch_friction_slope_ds  # noqa: E402
from lib.stage_paths import hecras_stage_dir  # noqa: E402

ROOT = ORACLE.parents[1]
PROJECT = "simple_channel"
SOURCE = ORACLE / "projects" / PROJECT
STAGE = hecras_stage_dir(PROJECT)
WRITE_U02 = ORACLE / "scripts" / "write_simple_channel_u02.py"
DEFAULT_RAS = Path(r"C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe")

KEEP = (
    f"{PROJECT}.prj",
    f"{PROJECT}.g01",
    f"{PROJECT}.u02",
    f"{PROJECT}.p01",
    "reference_wsel_simple_channel_unsteady.json",
    "README.md",
)


def _copy_to_stage(src: Path, dest: Path) -> None:
    if is_ras_text_file(src):
        copy_ras_text_file(src, dest)
        assert_compact_ras_text(dest)
        return
    shutil.copy2(src, dest)


def _regenerate_u02() -> None:
    subprocess.run([sys.executable, str(WRITE_U02)], check=True, cwd=ROOT)


def _kill_ras() -> None:
    if sys.platform != "win32":
        return
    for image in ("Ras.exe", "PipeServer.exe"):
        subprocess.run(["taskkill", "/F", "/IM", image], capture_output=True, text=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--open-ras", action="store_true", help="Launch HEC-RAS GUI")
    parser.add_argument("--ras-exe", type=Path, default=DEFAULT_RAS)
    args = parser.parse_args()

    if sys.platform != "win32":
        print("ERROR: Chunk 1 prep must run on Windows.", file=sys.stderr)
        return 1
    if not SOURCE.joinpath(f"{PROJECT}.prj").is_file():
        print(f"ERROR: source project not found: {SOURCE}", file=sys.stderr)
        return 1

    print("=== Chunk 1 prep — simple_channel (friction-slope DS) ===\n")
    print("1.1 Kill stray HEC-RAS processes...")
    _kill_ras()
    print("    Done.\n")

    print("1.2 Regenerate + patch simple_channel.u02 (RAS 7 Normal Depth DS)...")
    _regenerate_u02()
    repo_u02 = SOURCE / f"{PROJECT}.u02"
    patch_meta = patch_friction_slope_ds(repo_u02)
    assert_compact_ras_text(repo_u02)
    friction_line = str(patch_meta.get("friction_line", ""))
    if "Friction Slope=" not in friction_line:
        print("ERROR: repo u02 missing Friction Slope after patch", file=sys.stderr)
        return 1
    if "Stage Hydrograph=" in repo_u02.read_text(encoding="utf-8"):
        print("ERROR: repo u02 still contains Stage Hydrograph", file=sys.stderr)
        return 1
    print(f"    {friction_line}")
    if patch_meta.get("ras_commander"):
        print("    ras-commander: converted downstream block to Normal Depth")
    print("    OK.\n")

    print("1.3 Stage fresh project copy (native Windows path)...")
    if STAGE.exists():
        shutil.rmtree(STAGE)
    STAGE.mkdir(parents=True, exist_ok=True)
    for name in KEEP:
        src = SOURCE / name
        if src.is_file():
            _copy_to_stage(src, STAGE / name)
    print(f"    Source: {SOURCE}")
    print(f"    Stage:  {STAGE}\n")

    prj = STAGE / f"{PROJECT}.prj"
    print("In HEC-RAS: File > Open Project > Documents > hecras_testing > simple_channel")
    print(f"  {prj}\n")
    staged_u02 = STAGE / f"{PROJECT}.u02"
    patch_friction_slope_ds(staged_u02)
    assert_compact_ras_text(staged_u02)
    friction_line = next(
        ln
        for ln in staged_u02.read_text(encoding="utf-8").splitlines()
        if ln.startswith("Friction Slope=")
    )
    print(f"    Staged u02: {staged_u02.stat().st_size} bytes")
    print(f"    DS BC line: {friction_line}\n")

    print("Chunk 1 GUI checklist:")
    print("  1.4 Unsteady Flow Editor -> verify:")
    print("      Upstream RM 3.0: Flow Hydrograph 150 cfs")
    print("      Downstream RM 0.0: Friction Slope = 0.001")
    print("      Save simple_channel.u02 if prompted")
    print("  1.5 Run -> Compute Plan 01 (unsteady)")
    print("  1.6 Record terminal WSEL at RM 3.0, 2.0, 1.0, 0.0\n")
    print("Expected HDF after compute:")
    print(f"  {STAGE / f'{PROJECT}.p01.hdf'}\n")
    print("When compute finishes, run:")
    print("  py -3 verification\\oracle\\scripts\\chunk1_simple_channel_capture.py\n")

    if args.open_ras:
        if not args.ras_exe.is_file():
            print(f"ERROR: Ras.exe not found: {args.ras_exe}", file=sys.stderr)
            return 1
        subprocess.Popen([str(args.ras_exe)])
        print("Launched HEC-RAS — open project manually:")
        print(f"  {prj}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
