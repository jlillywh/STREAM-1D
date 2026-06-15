#!/usr/bin/env python3
"""Smoke test for ConSpan g01 culvert parse + full structure mapping."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ROOT / "python"))

from lib.conspan_mapper import build_conspan_unsteady_inputs  # noqa: E402
from lib.culvert_mapper import parsed_xs_ineffective_flow_areas  # noqa: E402
from lib.hecras_geom_parser import cross_section_at_rm, parse_g01  # noqa: E402

PROJECT = ORACLE / "projects" / "conspan"


def main() -> int:
    g = parse_g01(PROJECT / "ConSpan.g01")
    assert len(g.culverts) == 1, f"expected 1 culvert, got {len(g.culverts)}"
    c = g.culverts[0]
    assert c.hec_shape == 9
    assert c.span == 28 and c.rise == 6 and c.length == 50
    assert abs(c.z_up - 25.1) < 1e-6 and abs(c.z_down - 25.0) < 1e-6
    assert c.chart == 61 and c.scale == 3
    assert abs(c.roughness_n_bottom - 0.03) < 1e-6
    assert abs(c.depth_bottom_n - 0.5) < 1e-6

    bu = cross_section_at_rm(g.cross_sections, 20.238)
    bd = cross_section_at_rm(g.cross_sections, 20.227)
    assert bu is not None and bd is not None
    assert len(bu.ineff_blocks) == 2
    assert len(bd.ineff_blocks) == 2
    bu_ineff = parsed_xs_ineffective_flow_areas(bu)
    assert bu_ineff and bu_ineff.get("left_blocks")

    payload, _ = build_conspan_unsteady_inputs(PROJECT)
    assert payload["culvert_z_ups"] == [25.1]
    assert payload["culvert_z_downs"] == [25.0]
    assert payload["culvert_inlet_types"] == [20]
    assert payload.get("culvert_crest_elevs") == [33.7]

    xs_by_rm = {20.238: None, 20.227: None}
    for xs in payload["cross_sections"]:
        for rm, station in ((20.238, 1257.0), (20.227, 1200.0)):
            if abs(xs["station"] - station) < 1.0:
                xs_by_rm[rm] = xs
    for rm in xs_by_rm:
        assert xs_by_rm[rm] is not None, f"missing XS at RM {rm}"
        assert xs_by_rm[rm].get("ineffective_flow_areas"), f"RM {rm} missing ineffective areas"
        if rm == 20.238:
            assert xs_by_rm[rm]["coeff_expansion"] == 0.5
            assert xs_by_rm[rm]["coeff_contraction"] == 0.3

    print("smoke_conspan_parse: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
