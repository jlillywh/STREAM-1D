#!/usr/bin/env python3
"""Smoke test: parity case emit → parse → STREAM payload build."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ROOT / "python"))

from lib.generic_unsteady_mapper import build_generic_unsteady_inputs  # noqa: E402
from lib.hecras_emitter import emit_hecras_project  # noqa: E402
from lib.hecras_geom_parser import parse_g01  # noqa: E402
from lib.parity_case import load_parity_case, resolve_culverts, resolve_cross_sections  # noqa: E402


def _smoke(case_name: str) -> None:
    case_path = ORACLE / "cases" / f"{case_name}.json"
    case = load_parity_case(case_path)
    project_dir = ORACLE / "projects" / "generated" / case.id
    emit_hecras_project(case, project_dir)

    geom = parse_g01(project_dir / f"{case.id}.g01")
    assert len(geom.cross_sections) == len(resolve_cross_sections(case))
    assert len(geom.culverts) == len(resolve_culverts(case))

    payload, flow = build_generic_unsteady_inputs(
        project_dir,
        geometry_name=f"{case.id}.g01",
        flow_name=f"{case.id}.u02",
        plan_name=f"{case.id}.p02",
        coupling_mode=int(case.stream1d_cfg().get("coupling_mode", 0)),
    )
    assert len(payload["cross_sections"]) == len(geom.cross_sections)
    assert len(flow.upstream_q_cfs) >= 2
    if geom.culverts:
        barrels = payload.get("culvert_barrels", [])
        expected_barrels = [c.num_barrels for c in resolve_culverts(case)]
        assert barrels == expected_barrels, f"culvert_barrels {barrels} != expected {expected_barrels}"
    print(f"OK {case_name}: {len(geom.cross_sections)} XS, {len(geom.culverts)} culvert(s), {len(flow.upstream_q_cfs)} steps")


def main() -> None:
    _smoke("reach_mild_stage")
    _smoke("conspan_arch_culvert")
    print("All parity case smokes passed.")


if __name__ == "__main__":
    main()
