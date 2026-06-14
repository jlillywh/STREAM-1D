"""Map linked ConSpan culvert HEC-RAS project to STREAM-1D UnsteadyInputs."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import stream1d as st

from .conspan_reference import (
    conspan_culvert_fields,
    conspan_solver_params,
    load_all_conspan_cross_sections,
)
from .hecras_geom_parser import parse_g01
from .hecras_plan_parser import find_plan_file, parse_plan
from .hecras_unsteady_parser import parse_unsteady_flow, ParsedUnsteadyFlow
from .hydrograph_ops import resample_hydrograph
from .unsteady_bc import downstream_bc_from_flow, steady_initial_wsel


def build_conspan_unsteady_inputs(
    project_dir: Path,
    *,
    geometry_name: str = "ConSpan.g01",
    flow_name: str = "conspan.u02",
    coupling_mode: int = 0,
) -> tuple[dict[str, Any], ParsedUnsteadyFlow]:
    """
    Build ConSpan unsteady inputs from bundled g01 + u02 + plan.

    Cross sections and culvert fields come from `conspan_project_12.json`
    (verified steady parity). The g01 is used for smoke validation only.
    """
    geom = parse_g01(project_dir / geometry_name)
    if len(geom.cross_sections) < 8:
        raise ValueError(f"expected ≥8 open-channel XS in {geometry_name}")

    flow = parse_unsteady_flow(project_dir / flow_name)
    plan_path = find_plan_file(project_dir, flow_name=flow_name)
    plan = parse_plan(plan_path) if plan_path else None

    cross_sections = load_all_conspan_cross_sections()
    culvert_fields = conspan_culvert_fields()
    if not culvert_fields.get("culvert_stations"):
        raise ValueError("ConSpan fixture must include culvert_stations")

    solver = conspan_solver_params()
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
    initial_wsel = steady_initial_wsel(
        cross_sections,
        flow.initial_flow_cfs,
        ds_bc.bc_type,
        ds_bc.slope,
        ds_bc.wsel_series[0],
        max_spacing=solver["max_spacing"],
        num_slices=solver["num_slices"],
        structure_fields=culvert_fields,
    )
    theta = plan.unsteady_theta if plan else 1.0

    first_xs = geom.cross_sections[0]
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
        theta=theta,
        num_slices=solver["num_slices"],
        max_spacing=solver["max_spacing"],
        coeff_contraction=first_xs.coeff_contraction,
        coeff_expansion=first_xs.coeff_expansion,
        structure_coupling_order=0,
        unsteady_structure_coupling_mode=coupling_mode,
        **culvert_fields,
    )
    return inputs.to_dict(), flow
