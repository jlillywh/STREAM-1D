#!/usr/bin/env python3
"""Chunk 1 prep — stage simple_channel Plan 03 (rating-curve DS)."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))

from lib.hecras_text_io import assert_compact_ras_text, copy_ras_text_file, is_ras_text_file  # noqa: E402
from lib.patch_simple_channel_rating_u02 import patch_rating_curve_ds  # noqa: E402
from lib.stage_paths import hecras_stage_dir  # noqa: E402
from lib.write_simple_channel_rating_prj import write_rating_prj  # noqa: E402

ROOT = ORACLE.parents[1]
PROJECT = "simple_channel"
SOURCE = ORACLE / "projects" / PROJECT
STAGE = hecras_stage_dir(PROJECT)
WRITE_U03 = ORACLE / "scripts" / "write_simple_channel_rating_u02.py"
DEFAULT_RAS = Path(r"C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe")

KEEP = (
    f"{PROJECT}.prj",
    f"{PROJECT}.g01",
    f"{PROJECT}.u03",
    f"{PROJECT}.p03",
    "rating_curve_ds.json",
    "reference_wsel_simple_channel_rating_unsteady.json",
)


def _copy_to_stage(src: Path, dest: Path) -> None:
    if is_ras_text_file(src):
        copy_ras_text_file(src, dest)
        assert_compact_ras_text(dest)
        return
    shutil.copy2(src, dest)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--open-ras", action="store_true")
    parser.add_argument("--ras-exe", type=Path, default=DEFAULT_RAS)
    args = parser.parse_args()

    if sys.platform != "win32":
        print("ERROR: prep must run on Windows.", file=sys.stderr)
        return 1

    print("=== Chunk 1 prep — simple_channel Plan 03 (rating-curve DS) ===\n")
    subprocess.run([sys.executable, str(WRITE_U03)], check=True, cwd=ROOT)

    import json

    rating_doc = json.loads((SOURCE / "rating_curve_ds.json").read_text(encoding="utf-8"))
    rating_q = rating_doc["rating_q_cfs"]
    rating_wsel = rating_doc["rating_wsel_ft"]
    repo_u03 = SOURCE / f"{PROJECT}.u03"
    patch_rating_curve_ds(repo_u03, rating_q=rating_q, rating_wsel=rating_wsel)
    assert_compact_ras_text(repo_u03)

    if STAGE.exists():
        shutil.rmtree(STAGE)
    STAGE.mkdir(parents=True, exist_ok=True)
    for name in KEEP:
        src = SOURCE / name
        if src.is_file():
            _copy_to_stage(src, STAGE / name)
    write_rating_prj(STAGE / f"{PROJECT}.prj")

    prj = STAGE / f"{PROJECT}.prj"
    print(f"Staged: {STAGE}")
    print(f"Open project: {prj}")
    print("(prj defaults to Plan 03 + u03 rating-curve flow)")
    print("In HEC-RAS: verify RM 0.0 = Rating Curve, then Run > Compute Plan 03")
    print("After compute, confirm:")
    print(f"  {STAGE / f'{PROJECT}.p03.hdf'}")
    print("Then capture:")
    print("  py -3 verification\\oracle\\scripts\\chunk1_simple_channel_rating_capture.py")

    if args.open_ras and args.ras_exe.is_file():
        subprocess.Popen([str(args.ras_exe)])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
