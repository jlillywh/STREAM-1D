#!/usr/bin/env python3
"""Smoke test for Beaver linked parsers + g01 → STREAM-1D bridge mapping."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ROOT / "python"))

from lib.beaver_mapper import build_beaver_unsteady_inputs  # noqa: E402
from lib.hecras_geom_parser import cross_section_by_description, parse_g01  # noqa: E402
from lib.hecras_plan_parser import find_plan_file, parse_plan  # noqa: E402
from lib.hecras_unsteady_parser import parse_unsteady_flow  # noqa: E402

PROJECT = ORACLE / "projects" / "beaver"


def main() -> int:
    g = parse_g01(PROJECT / "beaver.g01")
    flow = parse_unsteady_flow(PROJECT / "beaver.u02")
    plan_path = find_plan_file(PROJECT)
    if plan_path:
        plan = parse_plan(plan_path)
        print(
            "plan",
            plan.plan_number,
            "theta",
            plan.unsteady_theta,
            "dt",
            plan.computation_interval_seconds,
        )
    print("XS", len(g.cross_sections))
    if g.bridge:
        print(
            "bridge_rm",
            g.bridge.rm,
            "deck_pts",
            len(g.bridge.deck_low_stations),
            "piers",
            len(g.bridge.pier_stations),
            "length",
            g.bridge.bridge_length,
        )
    bu = cross_section_by_description(g.cross_sections, "upstream of bridge")
    bd = cross_section_by_description(g.cross_sections, "downstream of bridge")
    print("BU", bu.rm if bu else None, "ineff", len(bu.ineff_blocks) if bu else 0)
    print("BD", bd.rm if bd else None, "ineff", len(bd.ineff_blocks) if bd else 0)
    print("Q_steps", len(flow.upstream_q_cfs), "peak_Q", max(flow.upstream_q_cfs))
    print("HWM_count", len(flow.observed_hwm))

    payload, _ = build_beaver_unsteady_inputs(PROJECT)
    print("mapped_steps", payload.get("num_steps"), "dt", payload.get("dt"))
    print("coupling_mode", payload.get("unsteady_structure_coupling_mode"))
    print("friction_slope_method", payload.get("unsteady_friction_slope_method"))
    print("approach_friction_len", payload.get("bridge_approach_friction_lengths"))
    print("bridge_deck_pts", len(payload.get("bridge_deck_stations", [[]])[0]))
    print("has_BU", bool(payload.get("bridge_upstream_cross_sections")))
    print("has_BD", bool(payload.get("bridge_downstream_cross_sections")))
    assert g.bridge and len(g.bridge.deck_low_stations) > 0
    assert bu is not None and bd is not None
    assert payload.get("bridge_upstream_cross_sections")
    assert payload.get("bridge_downstream_cross_sections")
    assert payload.get("bridge_approach_friction_lengths")
    assert payload.get("bridge_roadway_embankments")
    print("smoke_beaver_parse: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
