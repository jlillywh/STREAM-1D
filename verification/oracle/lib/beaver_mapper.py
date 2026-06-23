"""Map linked Beaver Creek HEC-RAS project to STREAM-1D UnsteadyInputs."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import stream1d as st

from .bridge_mapper import build_bridge_fields
from .hecras_geom_parser import (
    ParsedCrossSection,
    cross_section_by_description,
    parse_g01,
    parsed_xs_to_dict,
    parsed_xs_to_reach_dict,
    rm_to_station,
)
from .hecras_plan_parser import find_plan_file, parse_plan
from .hecras_unsteady_parser import parse_unsteady_flow, ParsedUnsteadyFlow
from .hydrograph_ops import resample_hydrograph
from .unsteady_bc import downstream_bc_from_flow, steady_initial_wsel

NUM_SLICES = 80
MAX_SPACING = 200.0


def build_beaver_unsteady_inputs(
    project_dir: Path,
    *,
    coupling_mode: int = 2,
    unsteady_friction_slope_method: int | None = None,
) -> tuple[dict[str, Any], ParsedUnsteadyFlow]:
    geom = parse_g01(project_dir / "beaver.g01")
    flow = parse_unsteady_flow(project_dir / "beaver.u02")
    plan_path = find_plan_file(project_dir)
    plan = parse_plan(plan_path) if plan_path else None

    cross_sections = [st.CrossSection(**parsed_xs_to_reach_dict(xs)) for xs in geom.cross_sections]
    bridge_fields = build_bridge_fields(geom)
    roadway_embankments = bridge_fields.pop("bridge_roadway_embankments", None)
    approach_sections = bridge_fields.pop("bridge_approach_cross_sections", None)
    departure_sections = bridge_fields.pop("bridge_departure_cross_sections", None)
    structure_fields = dict(bridge_fields)
    if roadway_embankments is not None:
        structure_fields["bridge_roadway_embankments"] = roadway_embankments
    if approach_sections is not None:
        structure_fields["bridge_approach_cross_sections"] = approach_sections
    if departure_sections is not None:
        structure_fields["bridge_departure_cross_sections"] = departure_sections

    upstream_q = list(flow.upstream_q_cfs)
    dt_seconds = flow.interval_seconds
    if plan and plan.computation_interval_seconds < dt_seconds:
        upstream_q, dt_seconds = resample_hydrograph(
            upstream_q,
            flow.interval_seconds,
            plan.computation_interval_seconds,
        )

    num_steps = len(upstream_q)
    ds_bc = downstream_bc_from_flow(flow, num_steps)

    bu_xs = cross_section_by_description(geom.cross_sections, "upstream of bridge")
    coeff_contraction = bu_xs.coeff_contraction if bu_xs else 0.3
    coeff_expansion = bu_xs.coeff_expansion if bu_xs else 0.1

    initial_wsel = steady_initial_wsel(
        cross_sections,
        flow.initial_flow_cfs,
        ds_bc.bc_type,
        ds_bc.slope,
        ds_bc.wsel_series[0],
        max_spacing=MAX_SPACING,
        num_slices=NUM_SLICES,
        structure_fields=structure_fields,
        coeff_contraction=coeff_contraction,
        coeff_expansion=coeff_expansion,
        downstream_bc_rating_q=ds_bc.rating_q,
        downstream_bc_rating_wsel=ds_bc.rating_wsel,
    )
    theta = plan.unsteady_theta if plan else 0.6
    friction_method = (
        unsteady_friction_slope_method
        if unsteady_friction_slope_method is not None
        else (plan.unsteady_friction_slope_method if plan else 2)
    )

    inputs = st.UnsteadyInputs(
        cross_sections=cross_sections,
        initial_wsel=initial_wsel,
        initial_q=[flow.initial_flow_cfs] * len(cross_sections),
        dt=dt_seconds,
        num_steps=num_steps,
        upstream_q_hydrograph=upstream_q,
        downstream_wsel_hydrograph=ds_bc.wsel_series,
        downstream_bc_type=ds_bc.bc_type,
        downstream_bc_slope=ds_bc.slope,
        downstream_bc_rating_q=ds_bc.rating_q or [],
        downstream_bc_rating_wsel=ds_bc.rating_wsel or [],
        theta=theta,
        unsteady_friction_slope_method=friction_method,
        num_slices=NUM_SLICES,
        max_spacing=MAX_SPACING,
        coeff_contraction=coeff_contraction,
        coeff_expansion=coeff_expansion,
        structure_coupling_order=0,
        unsteady_structure_coupling_mode=coupling_mode,
        **bridge_fields,
    )
    payload = inputs.to_dict()
    if roadway_embankments is not None:
        payload["bridge_roadway_embankments"] = roadway_embankments
    if approach_sections is not None:
        payload["bridge_approach_cross_sections"] = [xs.to_dict() for xs in approach_sections]
    if departure_sections is not None:
        payload["bridge_departure_cross_sections"] = [xs.to_dict() for xs in departure_sections]
    return payload, flow


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
