#!/usr/bin/env python3
"""
Chunk 3.3 — g01 parser coverage and duplicate-obstruction checks.

Exercises Beaver (bridge) and ConSpan (culvert) bundled .g01 files used by linked scenarios.
"""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE / "scripts"))
sys.path.insert(0, str(ROOT / "python"))

from lib.beaver_mapper import build_beaver_unsteady_inputs  # noqa: E402
from lib.conspan_reference import (  # noqa: E402
    conspan_culvert_fields,
    load_all_conspan_cross_sections,
    rm_to_conspan_station,
)
from lib.hecras_geom_parser import (  # noqa: E402
    cross_section_by_description,
    parse_g01,
    parsed_xs_to_dict,
    parsed_xs_to_reach_dict,
)
from test_beaver_g01_parser import test_beaver_br_coef_and_deck_chords  # noqa: E402

BEAVER = ORACLE / "projects" / "beaver"
CONSPAN = ORACLE / "projects" / "conspan"


def _assert(condition: bool, msg: str) -> None:
    if not condition:
        raise AssertionError(msg)


def test_beaver_deck_piers_bu_bd_ineffective() -> None:
    """Deck profile, piers, BU/BD descriptions, ineffective blocks → mapper fields."""
    geom = parse_g01(BEAVER / "beaver.g01")
    bridge = geom.bridge
    _assert(bridge is not None, "Beaver g01 must parse a bridge block")
    _assert(len(bridge.deck_low_stations) >= 4, "piecewise deck: expected ≥4 low stations")
    _assert(
        len(bridge.deck_low_stations) == len(bridge.deck_low_elevations) == len(bridge.deck_high_elevations),
        "deck station/elevation arrays must align",
    )
    _assert(len(bridge.pier_stations) == 9, f"expected 9 pier stations, got {len(bridge.pier_stations)}")
    _assert(
        len(bridge.pier_base_elevations) >= 9,
        f"expected ≥9 pier base elevations, got {len(bridge.pier_base_elevations)}",
    )
    _assert(
        len(bridge.pier_top_elevations) >= 9,
        f"expected ≥9 pier top elevations, got {len(bridge.pier_top_elevations)}",
    )

    bu = cross_section_by_description(geom.cross_sections, "upstream of bridge")
    bd = cross_section_by_description(geom.cross_sections, "downstream of bridge")
    _assert(bu is not None and bd is not None, "BU/BD XS descriptions must parse")
    _assert(len(bu.ineff_blocks) >= 1, "BU must carry ineffective-flow blocks from g01")
    _assert(len(bd.ineff_blocks) >= 1, "BD must carry ineffective-flow blocks from g01")

    payload, _ = build_beaver_unsteady_inputs(BEAVER)
    _assert(payload.get("bridge_deck_stations"), "mapper must emit bridge_deck_stations")
    _assert(payload.get("bridge_pier_stations"), "mapper must emit bridge_pier_stations")
    _assert(payload.get("bridge_upstream_cross_sections"), "mapper must emit BU face")
    _assert(payload.get("bridge_downstream_cross_sections"), "mapper must emit BD face")
    emb = payload.get("bridge_roadway_embankments")
    _assert(emb and len(emb) == 1, "mapper must emit bridge_roadway_embankments")
    faces = emb[0].get("ineffective_faces") or {}
    _assert(
        faces.get("upstream") and faces.get("downstream"),
        "roadway embankment must carry BU/BD ineffective_faces overrides",
    )
    _assert(
        payload.get("bridge_approach_cross_sections"),
        "mapper must emit bridge_approach_cross_sections",
    )
    _assert(
        payload.get("bridge_departure_cross_sections"),
        "mapper must emit bridge_departure_cross_sections",
    )
    _assert(
        len(payload["bridge_approach_cross_sections"]) == 1,
        "beaver has 1 bridge → 1 bridge_approach_cross_sections element",
    )
    _assert(
        len(payload["bridge_departure_cross_sections"]) == 1,
        "beaver has 1 bridge → 1 bridge_departure_cross_sections element",
    )
    _assert(
        not payload.get("bridge_ineffective_left_stations_upstream"),
        "flat BU ineffective must not duplicate roadway compose",
    )
    print("  beaver deck/piers/BU/BD/ineffective: OK")


def test_beaver_no_duplicate_obstruction() -> None:
    """
    Piers and BU/BD ineffective must not duplicate reach blocked/ineffective on parent XS.

    - Reach cross_sections from mapper omit ineffective_flow_areas (parsed_xs_to_dict).
    - No blocked_obstructions on reach nodes.
    - Pier geometry lives only under bridge_* arrays.
    """
    geom = parse_g01(BEAVER / "beaver.g01")
    payload, _ = build_beaver_unsteady_inputs(BEAVER)

    for xs in geom.cross_sections:
        d = parsed_xs_to_reach_dict(xs)
        _assert(
            "ineffective_flow_areas" not in d and "blocked_obstructions" not in d,
            f"reach XS RM {xs.rm} must not export ineffective/blocked on parent dict",
        )

    for idx, xs_dict in enumerate(payload["cross_sections"]):
        _assert(
            not xs_dict.get("blocked_obstructions"),
            f"payload cross_sections[{idx}] must not carry blocked_obstructions",
        )
        _assert(
            not xs_dict.get("ineffective_flow_areas"),
            f"payload cross_sections[{idx}] must not carry ineffective_flow_areas",
        )

    pier_stations = payload["bridge_pier_stations"][0]
    _assert(len(pier_stations) == 9, "piers only on bridge_pier_stations")
    _assert(
        all("pier" not in str(xs.get("station", "")).lower() for xs in payload["cross_sections"]),
        "pier stations must not appear as fake reach XS stations",
    )
    print("  beaver no duplicate obstruction: OK")


def test_conspan_g01_reach_and_fixture_culvert() -> None:
    """
    ConSpan g01 parses open-channel + embankment XS; culvert arrays come from verified fixture.

    Runtime g01 culvert extraction is not implemented; fixture fields must match g01 culvert RM.
    """
    geom = parse_g01(CONSPAN / "ConSpan.g01")
    _assert(len(geom.cross_sections) >= 10, "ConSpan g01 must parse ≥10 cross sections")

    bu_rm = 20.238
    bd_rm = 20.227
    bu = next((xs for xs in geom.cross_sections if abs(xs.rm - bu_rm) < 0.01), None)
    bd = next((xs for xs in geom.cross_sections if abs(xs.rm - bd_rm) < 0.01), None)
    _assert(bu is not None and bd is not None, "ConSpan embankment RMs 20.238/20.227 must parse")

    culvert = conspan_culvert_fields()
    stations = culvert["culvert_stations"]
    _assert(stations, "fixture must define culvert_stations")

    bu_sta = rm_to_conspan_station(bu_rm)
    bd_sta = rm_to_conspan_station(bd_rm)
    _assert(bu_sta is not None and bd_sta is not None, "fixture must map g01 embankment RMs to stations")
    lo, hi = sorted((bu_sta, bd_sta))
    cul_sta = stations[0]
    _assert(
        lo <= cul_sta <= hi,
        f"fixture culvert station {cul_sta} must lie between embankment XS ({lo}–{hi})",
    )

    fixture_xs = load_all_conspan_cross_sections()
    _assert(len(fixture_xs) >= 10, "ConSpan fixture must supply full reach XS for linked verify")
    print("  conspan g01 reach + fixture culvert: OK")


def main() -> int:
    print("=== Chunk 3.3 g01 mapping tests ===")
    test_beaver_deck_piers_bu_bd_ineffective()
    test_beaver_br_coef_and_deck_chords()
    test_beaver_no_duplicate_obstruction()
    test_conspan_g01_reach_and_fixture_culvert()
    print("test_chunk3_g01_mapping: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
