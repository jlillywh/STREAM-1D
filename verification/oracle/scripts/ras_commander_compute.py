#!/usr/bin/env python3
"""
Run one HEC-RAS plan via ras-commander (Windows Python only).

Invoked from WSL by ras_headless.py so compute uses RasCmdr.compute_plan
(dialog watchdog) instead of Ras.exe -c, which often opens the GUI and hangs.
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path


def _compute_succeeded(result: object) -> bool:
    if result is False or result is None:
        return False
    success = getattr(result, "success", None)
    if success is not None:
        if isinstance(success, str):
            return success.upper() in {"SUCCESS", "OK", "TRUE", "1"}
        return bool(success)
    return bool(result)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("project_dir", type=Path, help="HEC-RAS project folder")
    parser.add_argument("plan_key", help="Plan number, e.g. 03")
    parser.add_argument("--ras-version", default=os.environ.get("HECRAS_VERSION"))
    parser.add_argument(
        "--clear-geompre",
        action="store_true",
        help="Clear geometry preprocessor before run",
    )
    args = parser.parse_args()

    try:
        from ras_commander import RasCmdr, init_ras_project  # type: ignore[import-not-found]
    except ImportError:
        print(
            "ERROR: ras-commander not installed for this Python.\n"
            "Install with: py -3 -m pip install ras-commander",
            file=sys.stderr,
        )
        return 2

    project_dir = args.project_dir.resolve()
    stem = project_dir.name
    prj = project_dir / f"{stem}.prj"
    if not prj.is_file():
        prj_files = sorted(project_dir.glob("*.prj"))
        if len(prj_files) == 1:
            prj = prj_files[0]
        else:
            print(f"ERROR: no unique .prj in {project_dir}", file=sys.stderr)
            return 2

    version = args.ras_version or "7.0.1"
    plan_key = str(args.plan_key).zfill(2)
    os.chdir(project_dir)
    print(f"ras-commander: init {prj.name} (RAS {version})", flush=True)
    init_ras_project(str(prj.resolve()), version, hide_intro=True)
    print(f"ras-commander: compute plan {plan_key}", flush=True)
    result = RasCmdr.compute_plan(
        plan_key,
        clear_geompre=args.clear_geompre,
        force_rerun=True,
        dialog_watchdog=True,
    )
    if not _compute_succeeded(result):
        print(f"ERROR: RasCmdr.compute_plan failed: {result!r}", file=sys.stderr)
        return 1
    print(f"OK: plan {plan_key} complete", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
