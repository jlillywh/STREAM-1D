"""Map HEC-RAS culvert geometry to STREAM-1D culvert input arrays."""

from __future__ import annotations

from typing import Any

import stream1d as st

from .hecras_geom_parser import ParsedCrossSection, ParsedCulvert, ParsedGeometry, rm_to_station

# HEC-RAS culvert shape code → STREAM-1D `culvert_shape_types`
HEC_TO_STREAM_CULVERT_SHAPE: dict[int, int] = {
    1: 0,  # Circular
    2: 1,  # Box
    3: 2,  # Arch
    9: 3,  # ConSpan arch
    4: 4,  # Pipe arch
    5: 5,  # Elliptical
    6: 6,  # Horseshoe
}


def hec_shape_to_stream_type(hec_shape: int) -> int:
    try:
        return HEC_TO_STREAM_CULVERT_SHAPE[hec_shape]
    except KeyError as exc:
        raise ValueError(f"unsupported HEC-RAS culvert shape code {hec_shape}") from exc


def hecras_inlet_type(
    stream_shape: int,
    *,
    chart: int,
    scale: int,
    entrance_loss_coeff: float,
) -> int:
    """
    Map HEC-RAS Chart/Scale + Ke to STREAM-1D `culvert_inlet_types`.

    ConSpan (shape 3): Chart 61 / Scale 3 (90° wingwalls) with Ke=0.5 uses the
    projecting/wingwall nomograph branch (20). Smooth headwall (21) applies for Ke ≤ 0.2.
    """
    if stream_shape == 3:
        if entrance_loss_coeff <= 0.2:
            return 21
        if chart == 61 and scale == 3:
            return 20
        return 20 if entrance_loss_coeff > 0.2 else 21
    if stream_shape == 0:
        if entrance_loss_coeff <= 0.2:
            return 2
        return 4 if entrance_loss_coeff > 0.5 else 1
    if stream_shape == 1:
        if entrance_loss_coeff <= 0.2:
            return 11
        return 10
    return 20 if entrance_loss_coeff > 0.2 else 21


def parsed_xs_ineffective_flow_areas(xs: ParsedCrossSection) -> dict[str, list[dict[str, float]]] | None:
    """Convert HEC-RAS #XS Ineff blocks to `CrossSection.ineffective_flow_areas`."""
    if not xs.ineff_blocks or not xs.x:
        return None
    max_sta = max(xs.x)
    left_blocks: list[dict[str, float]] = []
    right_blocks: list[dict[str, float]] = []
    for lo, hi, elev in xs.ineff_blocks:
        if hi <= lo or hi <= 0:
            right_blocks.append({"station": lo, "elevation": elev})
            if max_sta > lo + 1e-6:
                right_blocks.append({"station": max_sta, "elevation": elev})
        else:
            left_blocks.append({"station": lo, "elevation": elev})
            if hi > lo + 1e-6:
                left_blocks.append({"station": hi, "elevation": elev})
    if not left_blocks and not right_blocks:
        return None
    return {"left_blocks": left_blocks, "right_blocks": right_blocks}


def culvert_fields_from_parsed(culvert: ParsedCulvert, station: float) -> dict[str, Any]:
    """Single culvert → STREAM-1D parallel-array fields (one element each)."""
    stream_shape = hec_shape_to_stream_type(culvert.hec_shape)
    inlet_type = hecras_inlet_type(
        stream_shape,
        chart=culvert.chart,
        scale=culvert.scale,
        entrance_loss_coeff=culvert.entrance_loss_coeff,
    )
    fields: dict[str, Any] = {
        "culvert_stations": [station],
        "culvert_shape_types": [stream_shape],
        "culvert_spans": [culvert.span],
        "culvert_rises": [culvert.rise],
        "culvert_roughness_ns": [culvert.roughness_n],
        "culvert_lengths": [culvert.length],
        "culvert_entrance_loss_coeffs": [culvert.entrance_loss_coeff],
        "culvert_exit_loss_coeffs": [culvert.exit_loss_coeff],
        "culvert_barrels": [culvert.num_barrels],
        "culvert_roughness_n_bottoms": [culvert.roughness_n_bottom or culvert.roughness_n],
        "culvert_depth_bottom_ns": [culvert.depth_bottom_n],
        "culvert_depth_blockeds": [culvert.depth_blocked],
        "culvert_inlet_types": [inlet_type],
        "culvert_z_ups": [culvert.z_up],
        "culvert_z_downs": [culvert.z_down],
    }
    if culvert.crest_elev is not None:
        fields["culvert_crest_elevs"] = [culvert.crest_elev]
    return fields


def culvert_fields_from_fixture_rows(rows: list[dict[str, Any]]) -> dict[str, Any]:
    """Build culvert arrays from a project fixture `culvert_stations` list."""
    if not rows:
        return {}
    fields: dict[str, Any] = {
        "culvert_stations": [float(c["station"]) for c in rows],
        "culvert_shape_types": [int(c["shape_type"]) for c in rows],
        "culvert_spans": [float(c["span"]) for c in rows],
        "culvert_rises": [float(c["rise"]) for c in rows],
        "culvert_roughness_ns": [float(c["roughness_n"]) for c in rows],
        "culvert_lengths": [float(c["length"]) for c in rows],
        "culvert_entrance_loss_coeffs": [float(c["entrance_loss_coeff"]) for c in rows],
        "culvert_exit_loss_coeffs": [float(c["exit_loss_coeff"]) for c in rows],
        "culvert_barrels": [int(c.get("num_barrels", 1)) for c in rows],
        "culvert_roughness_n_bottoms": [
            float(c.get("roughness_n_bottom", c["roughness_n"])) for c in rows
        ],
        "culvert_depth_bottom_ns": [float(c.get("depth_bottom_n", 0.0)) for c in rows],
        "culvert_depth_blockeds": [float(c.get("depth_blocked", 0.0)) for c in rows],
    }
    if any("inlet_type" in c for c in rows):
        fields["culvert_inlet_types"] = [int(c.get("inlet_type", 21)) for c in rows]
    if any("z_up" in c for c in rows):
        fields["culvert_z_ups"] = [float(c["z_up"]) for c in rows]
    if any("z_down" in c for c in rows):
        fields["culvert_z_downs"] = [float(c["z_down"]) for c in rows]
    if any("crest_elev" in c for c in rows):
        fields["culvert_crest_elevs"] = [float(c["crest_elev"]) for c in rows]
    return fields


def build_culvert_fields(
    geom: ParsedGeometry,
    *,
    fixture_rows: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    """
    Prefer parsed g01 culverts; fall back to fixture rows when g01 has none.
    """
    if geom.culverts:
        merged: dict[str, Any] = {}
        for culvert in geom.culverts:
            station = rm_to_station(geom.cross_sections, culvert.rm)
            piece = culvert_fields_from_parsed(culvert, station)
            for key, values in piece.items():
                merged.setdefault(key, []).extend(values)
        return merged
    if fixture_rows:
        return culvert_fields_from_fixture_rows(fixture_rows)
    return {}


def overlay_g01_cross_section_modifiers(
    cross_sections: list[st.CrossSection],
    parsed_xs: list[ParsedCrossSection],
    *,
    station_to_rm: dict[float, float] | None = None,
) -> list[st.CrossSection]:
    """Apply g01 Exp/Cntr and #XS Ineff onto fixture reach cross sections."""
    by_rm = {float(xs.rm): xs for xs in parsed_xs}
    out: list[st.CrossSection] = []
    for xs in cross_sections:
        rm = None
        if station_to_rm:
            rm = station_to_rm.get(float(xs.station))
        if rm is None:
            parsed = next(
                (p for p in parsed_xs if abs(float(p.station) - float(xs.station)) < 1e-3),
                None,
            )
        else:
            parsed = by_rm.get(rm)
        if parsed is None:
            out.append(xs)
            continue
        ineff = parsed_xs_ineffective_flow_areas(parsed)
        out.append(
            st.CrossSection(
                station=xs.station,
                x=xs.x,
                y=xs.y,
                n_stations=xs.n_stations,
                n_values=xs.n_values,
                unit_system=xs.unit_system,
                is_overbank=xs.is_overbank,
                blocked_obstructions=getattr(xs, "blocked_obstructions", None),
                ineffective_flow_areas=ineff or getattr(xs, "ineffective_flow_areas", None),
                coeff_contraction=parsed.coeff_contraction,
                coeff_expansion=parsed.coeff_expansion,
            )
        )
    return out
