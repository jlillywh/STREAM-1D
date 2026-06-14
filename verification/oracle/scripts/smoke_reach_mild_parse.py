#!/usr/bin/env python3
"""Smoke test: reach_mild g01/u02/p02 parse for linked verify."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ROOT / "python"))

from lib.hecras_geom_parser import parse_g01  # noqa: E402
from lib.hecras_unsteady_parser import parse_unsteady_flow  # noqa: E402
from lib.reach_mapper import build_reach_unsteady_inputs  # noqa: E402

PROJECT = ORACLE / "projects" / "reach_mild"


def main() -> int:
    g = parse_g01(PROJECT / "reach_mild.g01")
    flow = parse_unsteady_flow(PROJECT / "reach_mild.u02")
    payload, _ = build_reach_unsteady_inputs(PROJECT)
    rms = [xs.rm for xs in g.cross_sections]
    print(f"XS count: {len(g.cross_sections)}  RMs: {rms}")
    print(f"Q steps: {len(flow.upstream_q_cfs)}  DS stage steps: {len(flow.downstream_stage_hydrograph)}")
    print(f"Payload XS: {len(payload['cross_sections'])}  num_steps: {payload['num_steps']}")
    assert len(g.cross_sections) == 8, rms
    assert len(flow.upstream_q_cfs) == len(flow.downstream_stage_hydrograph)
    assert len(flow.upstream_q_cfs) in (49, 48), len(flow.upstream_q_cfs)
    print("OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
