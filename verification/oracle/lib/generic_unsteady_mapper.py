"""Build STREAM-1D UnsteadyInputs from emitted HEC-RAS project files (generic mapper)."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import stream1d as st

from .culvert_mapper import build_culvert_fields
from .hecras_geom_parser import parse_g01, parsed_xs_to_dict
from .hecras_plan_parser import find_plan_file, parse_plan
from .hecras_unsteady_parser import ParsedUnsteadyFlow, parse_unsteady_flow
from .hydrograph_ops import resample_hydrograph
from .unsteady_bc import downstream_bc_from_flow, steady_initial_wsel

NUM_SLICES_DEFAULT = 100
MAX_SPACING_DEFAULT = 100.0


def build_generic_unsteady_inputs(
    project_dir: Path,
    *,
    geometry_name: str,
    flow_name: str,
    plan_name: str | None = None,
    coupling_mode: int = 0,
    unsteady_friction_slope_method: int | None = None,
    num_slices: int | None = None,
    max_spacing: float | None = None,
) -> tuple[dict[str, Any], ParsedUnsteadyFlow]:
    """
    Map any emitted (or hand-authored) HEC-RAS single-reach project to UnsteadyInputs.

    Geometry, culverts, and BCs come from parsed g01/u02/plan — no per-case fixture overlay.
    """
    geom_path = project_dir / geometry_name
    flow_path = project_dir / flow_name
    geom = parse_g01(geom_path)
    if not geom.cross_sections:
        raise ValueError(f"No cross sections in {geom_path}")

    flow = parse_unsteady_flow(flow_path)
    if plan_name:
        plan_path = project_dir / plan_name
    else:
        plan_path = find_plan_file(project_dir, flow_name=flow_name)
    plan = parse_plan(plan_path) if plan_path and plan_path.is_file() else None

    cross_sections = [st.CrossSection(**parsed_xs_to_dict(xs)) for xs in geom.cross_sections]
    culvert_fields = build_culvert_fields(geom)

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

    coeff_expansion = geom.cross_sections[0].coeff_expansion
    coeff_contraction = geom.cross_sections[0].coeff_contraction
    slices = int(num_slices) if num_slices is not None else NUM_SLICES_DEFAULT
    spacing = float(max_spacing) if max_spacing is not None else MAX_SPACING_DEFAULT

    initial_wsel = steady_initial_wsel(
        cross_sections,
        flow.initial_flow_cfs,
        ds_bc.bc_type,
        ds_bc.slope,
        ds_bc.wsel_series[0],
        max_spacing=spacing,
        num_slices=slices,
        structure_fields=culvert_fields or None,
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
        num_slices=slices,
        max_spacing=spacing,
        coeff_contraction=coeff_contraction,
        coeff_expansion=coeff_expansion,
        structure_coupling_order=0,
        unsteady_structure_coupling_mode=coupling_mode,
        **(culvert_fields or {}),
    )
    return inputs.to_dict(), flow
