"""Map simple trapezoidal-channel HEC-RAS project to STREAM-1D UnsteadyInputs."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import stream1d as st

from .hecras_geom_parser import parse_g01, parsed_xs_to_dict, ParsedCrossSection
from .hecras_plan_parser import find_plan_file, parse_plan
from .hecras_unsteady_parser import parse_unsteady_flow, ParsedUnsteadyFlow
from .unsteady_bc import downstream_bc_from_flow, steady_initial_wsel
from .hydrograph_ops import resample_hydrograph

NUM_SLICES = 50
MAX_SPACING = 600.0


def build_simple_channel_unsteady_inputs(
    project_dir: Path,
    *,
    geometry_name: str = "simple_channel.g01",
    flow_name: str = "simple_channel.u02",
) -> tuple[dict[str, Any], ParsedUnsteadyFlow]:
    """Build reach-only unsteady inputs from bundled g01 + u02 (no structures)."""
    geom = parse_g01(project_dir / geometry_name)
    if geom.bridge is not None:
        raise ValueError(f"simple_channel expects no bridge in {geometry_name}")

    flow = parse_unsteady_flow(project_dir / flow_name)
    plan_path = find_plan_file(project_dir, flow_name=flow_name)
    plan = parse_plan(plan_path) if plan_path else None

    cross_sections = [st.CrossSection(**parsed_xs_to_dict(xs)) for xs in geom.cross_sections]
    if len(cross_sections) < 2:
        raise ValueError(f"expected ≥2 cross sections in {geometry_name}")

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
    first_xs = geom.cross_sections[0]
    initial_wsel = steady_initial_wsel(
        cross_sections,
        flow.initial_flow_cfs,
        ds_bc.bc_type,
        ds_bc.slope,
        ds_bc.wsel_series[0],
        max_spacing=MAX_SPACING,
        num_slices=NUM_SLICES,
        coeff_contraction=first_xs.coeff_contraction,
        coeff_expansion=first_xs.coeff_expansion,
        downstream_bc_rating_q=ds_bc.rating_q,
        downstream_bc_rating_wsel=ds_bc.rating_wsel,
    )
    theta = plan.unsteady_theta if plan else 1.0

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
        downstream_bc_rating_q=ds_bc.rating_q,
        downstream_bc_rating_wsel=ds_bc.rating_wsel,
        theta=theta,
        num_slices=NUM_SLICES,
        max_spacing=MAX_SPACING,
        coeff_contraction=first_xs.coeff_contraction,
        coeff_expansion=first_xs.coeff_expansion,
        unsteady_structure_coupling_mode=0,
    )
    return inputs.to_dict(), flow


def rm_to_payload_index(rm: float, parsed_xs: list[ParsedCrossSection]) -> int | None:
    best_idx = None
    best_dist = float("inf")
    for idx, xs in enumerate(parsed_xs):
        dist = abs(xs.rm - rm)
        if dist < best_dist:
            best_dist = dist
            best_idx = idx
    return best_idx if best_idx is not None and best_dist <= 0.02 else None
