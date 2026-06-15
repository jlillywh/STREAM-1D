"""Map linked Beaver Creek HEC-RAS project to STREAM-1D UnsteadyInputs."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import stream1d as st

from .hecras_geom_parser import (
    ParsedBridge,
    ParsedCrossSection,
    cross_section_by_description,
    cross_section_at_rm,
    parse_g01,
    parsed_xs_to_dict,
    rm_to_station,
)
from .hecras_plan_parser import find_plan_file, parse_plan
from .hecras_unsteady_parser import parse_unsteady_flow, ParsedUnsteadyFlow
from .hydrograph_ops import friction_slope_downstream_wsel, resample_hydrograph
from .unsteady_bc import steady_initial_wsel

NUM_SLICES = 80
MAX_SPACING = 200.0


def build_beaver_unsteady_inputs(project_dir: Path) -> tuple[dict[str, Any], ParsedUnsteadyFlow]:
    geom = parse_g01(project_dir / "beaver.g01")
    flow = parse_unsteady_flow(project_dir / "beaver.u02")
    plan_path = find_plan_file(project_dir)
    plan = parse_plan(plan_path) if plan_path else None

    cross_sections = [st.CrossSection(**parsed_xs_to_dict(xs)) for xs in geom.cross_sections]

    bridge_fields = _bridge_fields(geom.bridge, geom.cross_sections)
    _apply_bu_bd_faces(bridge_fields, geom.cross_sections, cross_sections)

    upstream_q = list(flow.upstream_q_cfs)
    dt_seconds = flow.interval_seconds
    if plan and plan.computation_interval_seconds < dt_seconds:
        upstream_q, dt_seconds = resample_hydrograph(
            upstream_q,
            flow.interval_seconds,
            plan.computation_interval_seconds,
        )

    num_steps = len(upstream_q)
    ds_wsel, downstream_bc_type, downstream_bc_slope = _downstream_bc(
        cross_sections, flow, upstream_q, num_steps
    )

    bu_xs = cross_section_by_description(geom.cross_sections, "upstream of bridge")
    coeff_contraction = bu_xs.coeff_contraction if bu_xs else 0.3
    coeff_expansion = bu_xs.coeff_expansion if bu_xs else 0.1

    initial_wsel = steady_initial_wsel(
        cross_sections,
        flow.initial_flow_cfs,
        downstream_bc_type,
        downstream_bc_slope,
        ds_wsel[0],
        max_spacing=MAX_SPACING,
        num_slices=NUM_SLICES,
        structure_fields=bridge_fields,
        coeff_contraction=coeff_contraction,
        coeff_expansion=coeff_expansion,
    )
    theta = plan.unsteady_theta if plan else 0.6

    inputs = st.UnsteadyInputs(
        cross_sections=cross_sections,
        initial_wsel=initial_wsel,
        initial_q=[flow.initial_flow_cfs] * len(cross_sections),
        dt=dt_seconds,
        num_steps=num_steps,
        upstream_q_hydrograph=upstream_q,
        downstream_wsel_hydrograph=ds_wsel,
        downstream_bc_type=downstream_bc_type,
        downstream_bc_slope=downstream_bc_slope,
        theta=theta,
        num_slices=NUM_SLICES,
        max_spacing=MAX_SPACING,
        coeff_contraction=coeff_contraction,
        coeff_expansion=coeff_expansion,
        **bridge_fields,
    )
    return inputs.to_dict(), flow


def _bridge_fields(bridge: ParsedBridge | None, cross_sections: list[ParsedCrossSection]) -> dict[str, Any]:
    if bridge is None:
        return {}
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
    }
    if bridge.pier_stations:
        fields["bridge_pier_stations"] = [bridge.pier_stations]
    if bridge.pier_base_elevations:
        fields["bridge_pier_base_elevations"] = [bridge.pier_base_elevations[:n_piers]]
    if bridge.pier_top_elevations:
        fields["bridge_pier_top_elevations"] = [bridge.pier_top_elevations[:n_piers]]
    return fields


def _apply_bu_bd_faces(
    bridge_fields: dict[str, Any],
    parsed_sections: list[ParsedCrossSection],
    stream_sections: list[st.CrossSection],
) -> None:
    bu = cross_section_by_description(parsed_sections, "upstream of bridge")
    bd = cross_section_by_description(parsed_sections, "downstream of bridge")
    interior = cross_section_at_rm(parsed_sections, 5.425)

    if bu:
        bridge_fields["bridge_upstream_cross_sections"] = [_to_face_section(bu)]
        _apply_bridge_ineffective(bridge_fields, bu, suffix="_upstream")
    if bd:
        bridge_fields["bridge_downstream_cross_sections"] = [_to_face_section(bd)]
        _apply_bridge_ineffective(bridge_fields, bd, suffix="_downstream")
    if interior and bu and bd:
        bridge_fields["bridge_internal_cross_sections"] = [[_to_face_section(interior)]]


def _to_face_section(xs: ParsedCrossSection) -> st.CrossSection:
    return st.CrossSection(**parsed_xs_to_dict(xs))


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


def _downstream_bc(
    cross_sections: list[st.CrossSection],
    flow: ParsedUnsteadyFlow,
    upstream_q: list[float],
    num_steps: int,
) -> tuple[list[float], int | None, float | None]:
    """Return (WSEL hydrograph placeholder, bc_type, slope) for downstream boundary."""
    slope = flow.downstream_friction_slope
    if slope is not None and slope > 0:
        ds_base = friction_slope_downstream_wsel(
            cross_sections, [flow.initial_flow_cfs], slope
        )[0]
        return [ds_base] * num_steps, 2, slope
    return _downstream_wsel_hydrograph(cross_sections, flow, upstream_q), 0, None


def _downstream_wsel_hydrograph(
    cross_sections: list[st.CrossSection],
    flow: ParsedUnsteadyFlow,
    upstream_q: list[float],
) -> list[float]:
    slope = flow.downstream_friction_slope
    if slope is not None and slope > 0:
        return friction_slope_downstream_wsel(cross_sections, upstream_q, slope)

    base = flow.observed_hwm.get(flow.downstream_rm or 0.0)
    if base is None and flow.observed_hwm:
        base = min(flow.observed_hwm.values())
    if base is None:
        ds_xs = min(cross_sections, key=lambda xs: xs.station)
        base = min(ds_xs.y) + 2.0
    q_max = max(upstream_q) if upstream_q else 1.0
    return [base + 0.5 * (q / q_max if q_max > 0 else 0.0) for q in upstream_q]


def rm_to_payload_index(rm: float, parsed_xs: list[ParsedCrossSection]) -> int | None:
    """Map river mile to index in payload cross_sections (same sort order as g01 parser)."""
    ordered = sorted(parsed_xs, key=lambda xs: xs.rm, reverse=True)
    best_idx = None
    best_dist = float("inf")
    for idx, xs in enumerate(ordered):
        dist = abs(xs.rm - rm)
        if dist < best_dist:
            best_dist = dist
            best_idx = idx
    return best_idx if best_idx is not None and best_dist <= 0.05 else None


def nearest_station_for_rm(cross_sections, rm: float) -> float:
    return rm_to_station(cross_sections, rm)
