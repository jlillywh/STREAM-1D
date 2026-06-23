#!/usr/bin/env python3
"""Regression tests for Beaver g01 bridge coefficient and deck parsing."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))

from lib.beaver_mapper import build_beaver_unsteady_inputs  # noqa: E402
from lib.hecras_geom_parser import parse_g01  # noqa: E402

BEAVER = ORACLE / "projects" / "beaver"


def test_beaver_br_coef_and_deck_chords() -> None:
    geom = parse_g01(BEAVER / "beaver.g01")
    bridge = geom.bridge
    assert bridge is not None

    assert abs(bridge.pressure_coeff_inlet - 0.34) < 1e-6, bridge.pressure_coeff_inlet
    assert abs(bridge.orifice_coeff - 0.7) < 1e-6, bridge.orifice_coeff
    assert bridge.high_flow_method == 1, bridge.high_flow_method

    assert bridge.deck_high_elevations[2] == 216.93
    assert bridge.deck_low_elevations[2] == 215.7
    assert min(bridge.deck_low_elevations) == 200.0
    assert max(bridge.deck_high_elevations) == 216.93

    payload, _ = build_beaver_unsteady_inputs(BEAVER)
    assert payload["bridge_orifice_coeffs"] == [0.7]
    assert payload["bridge_pressure_flow_coeffs_inlet"] == [0.34]
    assert payload["bridge_high_flow_methods"] == [1]
    assert payload["bridge_low_chords"] == [200.0]
    assert payload["bridge_high_chords"] == [216.93]
    emb = payload.get("bridge_roadway_embankments")
    assert emb and len(emb) == 1, "expected bridge_roadway_embankments"
    assert emb[0]["deck"]["stations"][0] == 0.0
    assert emb[0]["left"]["embankment_profile"]["stations"] == [0.0, 450.0]
    assert emb[0]["ineffective_faces"]["upstream"]["left_blocks"][0]["station"] == 0.0
    assert bridge.bc_design_num_piers == 9
    assert abs(bridge.bc_design_pier_width - 1.25) < 1e-6
    assert bridge.bc_design_opening_station == 470.0
    print("  beaver BR Coef + deck + roadway embankment: OK")


def main() -> int:
    print("=== Beaver g01 parser regression ===")
    test_beaver_br_coef_and_deck_chords()
    print("test_beaver_g01_parser: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
