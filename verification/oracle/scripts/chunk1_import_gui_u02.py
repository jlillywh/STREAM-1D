#!/usr/bin/env python3
"""Import GUI-validated simple_channel.u02 from the Windows stage folder into the repo."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))

from lib.hecras_text_io import assert_compact_ras_text, copy_ras_text_file, read_ras_lines  # noqa: E402
from lib.stage_paths import hecras_stage_dir  # noqa: E402

PROJECT = "simple_channel"
REPO_U02 = ORACLE / "projects" / PROJECT / f"{PROJECT}.u02"
DEFAULT_STAGE = hecras_stage_dir(PROJECT)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage-dir", type=Path, default=DEFAULT_STAGE)
    args = parser.parse_args()

    staged = args.stage_dir / f"{PROJECT}.u02"
    if not staged.is_file():
        print(f"ERROR: staged u02 not found: {staged}", file=sys.stderr)
        return 1

    assert_compact_ras_text(staged)
    lines = read_ras_lines(staged)
    text = "\n".join(lines)
    if "Friction Slope=" not in text:
        print(
            "ERROR: staged u02 has no Friction Slope= line — set Normal Depth / friction slope in GUI first.",
            file=sys.stderr,
        )
        return 1
    if "Stage Hydrograph=" in text:
        print(
            "WARNING: staged u02 still contains Stage Hydrograph; importing anyway.",
            file=sys.stderr,
        )

    copy_ras_text_file(staged, REPO_U02)
    print(f"Imported {staged}")
    print(f"  -> {REPO_U02}")
    for line in lines:
        if line.startswith("Friction Slope=") or line.startswith("Boundary Location="):
            print(f"  {line}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
