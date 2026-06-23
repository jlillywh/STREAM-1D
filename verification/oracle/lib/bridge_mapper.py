"""Map parsed HEC-RAS g01 bridge geometry to STREAM-1D bridge input fields."""

from __future__ import annotations

from typing import Any

import stream1d as st

from .hecras_geom_parser import (
    ParsedBridge,
    ParsedCrossSection,
    ParsedGeometry,
    cross_section_at_rm,
    cross_section_by_description,
    parsed_xs_to_dict,
    parsed_xs_to_reach_dict,
    rm_to_station,
)


def build_bridge_fields(geom: ParsedGeometry) -> dict[str, Any]:
    """Build bridge_* kwargs from parsed g01 geometry (deck, piers, BU/BD faces)."""
    if geom.bridge is None:
        return {}

    bridge_fields = _bridge_core_fields(geom.bridge, geom.cross_sections)
    bu = cross_section_by_description(geom.cross_sections, "upstream of bridge")
    bd = cross_section_by_description(geom.cross_sections, "downstream of bridge")

    embankment = _build_roadway_embankment(geom.bridge, bu, bd)
    if embankment:
        bridge_fields["bridge_roadway_embankments"] = [embankment]
        _apply_bu_bd_faces(bridge_fields, geom.cross_sections, apply_flat_ineffective=False)
    else:
        _apply_bu_bd_faces(bridge_fields, geom.cross_sections, apply_flat_ineffective=True)
    _apply_approach_departure_cross_sections(bridge_fields, geom.cross_sections, bu, bd)
    return bridge_fields


def _bridge_core_fields(
    bridge: ParsedBridge,
    cross_sections: list[ParsedCrossSection],
) -> dict[str, Any]:
    station = rm_to_station(cross_sections, bridge.rm)
    method = 4 if bridge.low_flow_method == "wspro" else 0
    n_piers = len(bridge.pier_stations) or 0

    fields: dict[str, Any] = {
        "bridge_stations": [station],
        "bridge_low_chords": [min(bridge.deck_low_elevations) if bridge.deck_low_elevations else 215.7],
        "bridge_high_chords": [max(bridge.deck_high_elevations) if bridge.deck_high_elevations else 216.93],
        "bridge_deck_stations": [bridge.deck_low_stations],
        "bridge_deck_low_elevations": [bridge.deck_low_elevations],
        "bridge_deck_high_elevations": [bridge.deck_high_elevations],
        "bridge_pier_widths": [bridge.pier_width],
        "bridge_num_piers": [n_piers],
        "bridge_weir_coeffs": [bridge.weir_coeff],
        "bridge_orifice_coeffs": [bridge.orifice_coeff],
        "bridge_low_flow_methods": [method],
        "bridge_wspro_coeffs": [bridge.wspro_coeff],
        "bridge_lengths": [bridge.bridge_length],
        "bridge_max_weir_submergence": [bridge.max_weir_submergence],
        "bridge_friction_weighting": [1],
        "bridge_high_flow_methods": [bridge.high_flow_method],
    }
    if bridge.pressure_coeff_inlet > 0:
        fields["bridge_pressure_flow_coeffs_inlet"] = [bridge.pressure_coeff_inlet]
    if bridge.pier_stations:
        fields["bridge_pier_stations"] = [bridge.pier_stations]
    if bridge.pier_base_elevations:
        fields["bridge_pier_base_elevations"] = [bridge.pier_base_elevations[:n_piers]]
    if bridge.pier_top_elevations:
        fields["bridge_pier_top_elevations"] = [bridge.pier_top_elevations[:n_piers]]
    return fields


def _build_roadway_embankment(
    bridge: ParsedBridge,
    bu: ParsedCrossSection | None,
    bd: ParsedCrossSection | None,
) -> dict[str, Any] | None:
    """
    Unified roadway embankment from piecewise deck + BU/BD ineffective (opening frame).

    Deck stations align with reach lateral x on the BU face (opening origin = min x).
    Embankment grade lines cover abutment fill outside the opening low-chord span.
    """
    stas = bridge.deck_low_stations
    lows = bridge.deck_low_elevations
    highs = bridge.deck_high_elevations
    if len(stas) < 2 or len(stas) != len(lows) or len(stas) != len(highs):
        return None

    s_left, s_right = _deck_opening_edges(stas, lows)
    s_outer_left = stas[0]
    s_outer_right = stas[-1]
    if s_left <= s_outer_left or s_right >= s_outer_right:
        return None

    left_side = {
        "embankment_profile": {
            "stations": [s_outer_left, s_left],
            "elevations": [lows[0], _interp(stas, highs, s_left)],
        }
    }
    right_side = {
        "embankment_profile": {
            "stations": [s_right, s_outer_right],
            "elevations": [_interp(stas, highs, s_right), lows[-1]],
        }
    }

    ineffective_faces: dict[str, Any] = {}
    if bu:
        bu_face = _ineffective_face_override(bu)
        if bu_face:
            ineffective_faces["upstream"] = bu_face
    if bd:
        bd_face = _ineffective_face_override(bd)
        if bd_face:
            ineffective_faces["downstream"] = bd_face

    emb: dict[str, Any] = {
        "deck": {
            "stations": list(stas),
            "low_elevations": list(lows),
            "high_elevations": list(highs),
        },
        "left": left_side,
        "right": right_side,
    }
    if ineffective_faces:
        emb["ineffective_faces"] = ineffective_faces
    return emb


def _deck_opening_edges(stations: list[float], low_elevs: list[float]) -> tuple[float, float]:
    """Stations where deck low chord is at the opening soffit (max low elevation)."""
    opening_low = max(low_elevs)
    tol = 0.05
    opening_stations = [s for s, lo in zip(stations, low_elevs) if lo >= opening_low - tol]
    if len(opening_stations) < 2:
        return stations[0], stations[-1]
    return min(opening_stations), max(opening_stations)


def _interp(stations: list[float], elevations: list[float], station: float) -> float:
    if station <= stations[0]:
        return elevations[0]
    if station >= stations[-1]:
        return elevations[-1]
    for i in range(len(stations) - 1):
        if station <= stations[i + 1]:
            t = (station - stations[i]) / (stations[i + 1] - stations[i])
            return elevations[i] + t * (elevations[i + 1] - elevations[i])
    return elevations[-1]


def _ineffective_face_override(xs: ParsedCrossSection) -> dict[str, Any] | None:
    if not xs.ineff_blocks or not xs.x:
        return None
    max_sta = max(xs.x)
    left_blocks: list[dict[str, float]] = []
    right_blocks: list[dict[str, float]] = []
    for lo, hi, elev in xs.ineff_blocks:
        if hi <= lo or hi <= 0:
            right_blocks.append({"station": lo, "elevation": elev})
            right_blocks.append({"station": max_sta, "elevation": elev})
        else:
            left_blocks.append({"station": lo, "elevation": elev})
            left_blocks.append({"station": hi, "elevation": elev})
    face: dict[str, Any] = {}
    if left_blocks:
        face["left_blocks"] = left_blocks
    if right_blocks:
        face["right_blocks"] = right_blocks
    return face or None


def _apply_bu_bd_faces(
    bridge_fields: dict[str, Any],
    parsed_sections: list[ParsedCrossSection],
    *,
    apply_flat_ineffective: bool,
) -> None:
    bu = cross_section_by_description(parsed_sections, "upstream of bridge")
    bd = cross_section_by_description(parsed_sections, "downstream of bridge")
    interior = None
    if bu and bd:
        mid_rm = (bu.rm + bd.rm) / 2.0
        interior = cross_section_at_rm(parsed_sections, mid_rm)

    if bu:
        bridge_fields["bridge_upstream_cross_sections"] = [_to_face_section(bu)]
        if apply_flat_ineffective:
            _apply_bridge_ineffective(bridge_fields, bu, suffix="_upstream")
    if bd:
        bridge_fields["bridge_downstream_cross_sections"] = [_to_face_section(bd)]
        if apply_flat_ineffective:
            _apply_bridge_ineffective(bridge_fields, bd, suffix="_downstream")
    if interior and bu and bd:
        bridge_fields["bridge_internal_cross_sections"] = [[_to_face_section(interior)]]

    if bu and bd:
        # Let resolve_bridge_friction_lengths_metric auto-compute approach/departure
        # from reach layout (nearest upstream/downstream of BU/BD). Explicit BU–BD
        # spacing (~100 ft) is the opening reach interval, not approach length.
        pass


def _to_face_section(xs: ParsedCrossSection) -> st.CrossSection:
    return st.CrossSection(**parsed_xs_to_reach_dict(xs))


def _apply_approach_departure_cross_sections(
    bridge_fields: dict[str, Any],
    cross_sections: list[ParsedCrossSection],
    bu: ParsedCrossSection | None,
    bd: ParsedCrossSection | None,
) -> None:
    """
    Provide explicit approach/departure cross sections for bridge interior friction.

    We use `parsed_xs_to_dict` so `#XS Ineff` blocks are preserved on the approach/departure
    cuts (unlike reach-node serialization which strips ineffective/blocked content).
    """
    if bu is None or bd is None:
        return

    tol = 1e-6
    approach_candidates = [xs for xs in cross_sections if xs.rm > bu.rm + tol]
    departure_candidates = [xs for xs in cross_sections if xs.rm < bd.rm - tol]

    # Upstream: closest higher RM. Downstream: closest lower RM.
    approach_xs = max(approach_candidates, key=lambda xs: xs.rm, default=None)
    departure_xs = max(departure_candidates, key=lambda xs: xs.rm, default=None)

    if approach_xs is not None:
        bridge_fields["bridge_approach_cross_sections"] = [
            st.CrossSection(**parsed_xs_to_dict(approach_xs))
        ]
    if departure_xs is not None:
        bridge_fields["bridge_departure_cross_sections"] = [
            st.CrossSection(**parsed_xs_to_dict(departure_xs))
        ]


def _apply_bridge_ineffective(
    fields: dict[str, Any],
    xs: ParsedCrossSection,
    *,
    suffix: str,
) -> None:
    if not xs.ineff_blocks or not xs.x:
        return
    max_sta = max(xs.x)
    left_stations: list[float] = []
    left_elevs: list[float] = []
    right_stations: list[float] = []
    right_elevs: list[float] = []
    for lo, hi, elev in xs.ineff_blocks:
        if hi <= lo or hi <= 0:
            right_stations.extend([lo, max_sta])
            right_elevs.extend([elev, elev])
        else:
            left_stations.extend([lo, hi])
            left_elevs.extend([elev, elev])
    if left_stations:
        fields[f"bridge_ineffective_left_stations{suffix}"] = [left_stations]
        fields[f"bridge_ineffective_left_elevations{suffix}"] = [left_elevs]
    if right_stations:
        fields[f"bridge_ineffective_right_stations{suffix}"] = [right_stations]
        fields[f"bridge_ineffective_right_elevations{suffix}"] = [right_elevs]
