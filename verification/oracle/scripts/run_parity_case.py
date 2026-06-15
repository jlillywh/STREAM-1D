#!/usr/bin/env python3
"""
Emit HEC-RAS + STREAM parity projects from a canonical case JSON, optionally run RAS and verify.

Workflow:
  1. Load ``cases/<id>.json`` (single source of truth).
  2. Emit ``projects/generated/<id>/`` (.g01, .u02, .p02, .prj).
  3. Write ``scenarios/generated/<id>_parity.json`` for linked verify.
  4. Optional ``--ras-run``: headless HEC-RAS → reference_wsel.json (Windows).
  5. Optional ``--verify``: STREAM-1D vs committed/generated reference.

Examples:
  # Emit only (no HEC-RAS required)
  PYTHONPATH=python python3 verification/oracle/scripts/run_parity_case.py \\
    --case verification/oracle/cases/reach_mild_stage.json --emit-only

  # Full loop on Windows (RAS + verify)
  python verification/oracle/scripts/run_parity_case.py \\
    --case verification/oracle/cases/conspan_arch_culvert.json --ras-run --verify

  # Daily dev: verify against committed reference (WSL/Linux)
  PYTHONPATH=python python3 verification/oracle/scripts/run_parity_case.py \\
    --case verification/oracle/cases/conspan_arch_culvert.json --verify
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ROOT / "python"))

from lib.hecras_emitter import emit_hecras_project, emit_linked_scenario  # noqa: E402
from lib.hecras_geom_parser import parse_g01  # noqa: E402
from lib.parity_case import load_parity_case, resolve_cross_sections, resolve_culverts  # noqa: E402


def _emit_case(case_path: Path) -> tuple[Path, Path]:
    case = load_parity_case(case_path)
    project_dir = ORACLE / "projects" / "generated" / case.id
    emit_hecras_project(case, project_dir)

    scenario_doc = emit_linked_scenario(case, ORACLE, project_dir)
    scenario_dir = ORACLE / "scenarios" / "generated"
    scenario_dir.mkdir(parents=True, exist_ok=True)
    scenario_path = scenario_dir / f"{case.id}_parity.json"
    scenario_path.write_text(json.dumps(scenario_doc, indent=2) + "\n", encoding="utf-8")
    return project_dir, scenario_path


def _roundtrip_smoke(case_path: Path, project_dir: Path) -> None:
    case = load_parity_case(case_path)
    g01 = project_dir / f"{case.id}.g01"
    geom = parse_g01(g01)
    expected_xs = len(resolve_cross_sections(case))
    expected_culverts = len(resolve_culverts(case))
    if len(geom.cross_sections) != expected_xs:
        raise RuntimeError(
            f"g01 round-trip XS count {len(geom.cross_sections)} != expected {expected_xs}"
        )
    if len(geom.culverts) != expected_culverts:
        raise RuntimeError(
            f"g01 round-trip culvert count {len(geom.culverts)} != expected {expected_culverts}"
        )
    print(
        f"Round-trip OK: {len(geom.cross_sections)} XS, "
        f"{len(geom.culverts)} culvert(s) parsed from {g01.name}"
    )


def _run_ras_reference(scenario_path: Path) -> int:
    script = ORACLE / "scripts" / "run_ras_reference.py"
    if not script.is_file():
        print(f"WARN: {script} not found — skip RAS reference refresh")
        return 0
    cmd = [sys.executable, str(script), "--scenario", str(scenario_path)]
    print(" ".join(cmd))
    return subprocess.call(cmd)


def _run_verify(scenario_path: Path, *, fmt: str) -> int:
    cmd = [
        sys.executable,
        str(ORACLE / "run_linked_verify.py"),
        "--scenario",
        str(scenario_path),
        "--format",
        fmt,
    ]
    env = dict(**{k: v for k, v in dict(**{"PYTHONPATH": str(ROOT / "python")}).items()})
    import os

    merged = os.environ.copy()
    merged.update(env)
    print(" ".join(cmd))
    return subprocess.call(cmd, env=merged)


def main() -> int:
    parser = argparse.ArgumentParser(description="Parity case → HEC-RAS emit → optional RAS + verify")
    parser.add_argument("--case", required=True, help="Path to cases/<id>.json")
    parser.add_argument("--emit-only", action="store_true", help="Emit project + scenario only")
    parser.add_argument("--ras-run", action="store_true", help="Run headless HEC-RAS and refresh reference")
    parser.add_argument("--verify", action="store_true", help="Run STREAM-1D linked verify")
    parser.add_argument("--format", choices=("table", "matrix", "both"), default="matrix")
    args = parser.parse_args()

    case_path = Path(args.case).resolve()
    if not case_path.is_file():
        print(f"Case file not found: {case_path}", file=sys.stderr)
        return 2

    project_dir, scenario_path = _emit_case(case_path)
    print(f"Emitted HEC-RAS project: {project_dir}")
    print(f"Emitted scenario: {scenario_path}")

    try:
        _roundtrip_smoke(case_path, project_dir)
    except Exception as exc:
        print(f"Round-trip smoke FAILED: {exc}", file=sys.stderr)
        return 1

    if args.emit_only and not args.ras_run and not args.verify:
        return 0

    rc = 0
    if args.ras_run:
        rc = _run_ras_reference(scenario_path)
        if rc != 0:
            return rc

    if args.verify:
        rc = _run_verify(scenario_path, fmt=args.format)
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
