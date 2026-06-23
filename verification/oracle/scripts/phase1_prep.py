#!/usr/bin/env python3
"""Phase 1 prep — stage reach_mild for HEC-RAS GUI (Windows)."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))

from lib.stage_paths import hecras_stage_dir  # noqa: E402

ROOT = ORACLE.parents[1]
SOURCE = ORACLE / "projects" / "reach_mild"
STAGE = hecras_stage_dir("reach_mild")
DEFAULT_RAS = Path(r"C:\Program Files (x86)\HEC\HEC-RAS\7.0.1\Ras.exe")

KEEP = (
    "reach_mild.prj",
    "reach_mild.g01",
    "reach_mild.u02",
    "reach_mild.p02",
    "reference_wsel_reach_mild_unsteady.json",
    "README.md",
)
_RAS_TEXT = {".prj", ".g01", ".u02", ".p02", ".f01"}


def _copy_to_stage(src: Path, dest: Path) -> None:
    """HEC-RAS text files must use CRLF on Windows; drop blank lines."""
    if src.suffix.lower() in _RAS_TEXT:
        lines = [ln for ln in src.read_text(encoding="utf-8", errors="replace").splitlines() if ln.strip()]
        dest.write_bytes(("\r\n".join(lines) + "\r\n").encode("utf-8"))
        return
    shutil.copy2(src, dest)


def _kill_ras() -> None:
    if sys.platform != "win32":
        return
    for image in ("Ras.exe", "PipeServer.exe"):
        subprocess.run(
            ["taskkill", "/F", "/IM", image],
            capture_output=True,
            text=True,
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "-OpenRas",
        "--open-ras",
        action="store_true",
        help="Launch HEC-RAS GUI (open the staged .prj via File > Open Project)",
    )
    parser.add_argument("--ras-exe", type=Path, default=DEFAULT_RAS)
    args = parser.parse_args()

    if sys.platform != "win32":
        print("ERROR: Phase 1 prep must run on Windows.", file=sys.stderr)
        return 1
    if not SOURCE.joinpath("reach_mild.prj").is_file():
        print(f"ERROR: source project not found: {SOURCE}", file=sys.stderr)
        return 1

    print("=== Phase 1 prep — reach_mild GUI session ===\n")
    print("1.1 Kill stray HEC-RAS processes...")
    _kill_ras()
    print("    Done.\n")

    print("1.2 Stage fresh project copy (native Windows path)...")
    STAGE.mkdir(parents=True, exist_ok=True)
    for path in STAGE.iterdir():
        if path.is_file() and path.name not in KEEP and path.suffix.lower() != ".hdf":
            path.unlink()
    for name in KEEP:
        src = SOURCE / name
        if src.is_file():
            _copy_to_stage(src, STAGE / name)
    print(f"    Source: {SOURCE}")
    print(f"    Stage:  {STAGE}\n")

    prj = STAGE / "reach_mild.prj"
    print("In HEC-RAS: File > Open Project > Documents > hecras_testing > reach_mild")
    print(f"  {prj}\n")
    print("Phase 1 GUI checklist:")
    print("  1.3 Unsteady Flow Editor -> verify BCs -> Save reach_mild.u02 (if prompted)")
    print("  1.4 Run -> Compute Plan 02 (unsteady)")
    print("  1.5 Record terminal WSEL at RM 20.208, 20.189, 20.095\n")
    print("Expected HDF after compute:")
    print(f"  {STAGE / 'reach_mild.p02.hdf'}\n")
    print("When compute finishes, run:")
    print("  py -3 verification\\oracle\\scripts\\phase1_capture_after_gui.py\n")

    if args.open_ras:
        if not args.ras_exe.is_file():
            print(f"ERROR: Ras.exe not found: {args.ras_exe}", file=sys.stderr)
            return 1
        print("Launching HEC-RAS (open the project manually — .prj is not file-associated)...")
        subprocess.Popen([str(args.ras_exe)])
        print("Then: File > Open Project >")
        print(f"  {prj}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
