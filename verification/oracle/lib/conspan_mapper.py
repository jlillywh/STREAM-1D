"""Map linked ConSpan culvert HEC-RAS project to STREAM-1D UnsteadyInputs."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import stream1d as st

from .conspan_reference import (
    conspan_solver_params,
    load_all_conspan_cross_sections,
    _load_conspan_project,
)
from .culvert_mapper import build_culvert_fields, overlay_g01_cross_section_modifiers
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
    plan_name: str | None = None,
    coupling_mode: int = 0,
    unsteady_friction_slope_method: int | None = None,
) -> tuple[dict[str, Any], ParsedUnsteadyFlow]:
    """
    Build ConSpan unsteady inputs from bundled g01 + u02 + plan.

    Reach geometry comes from `conspan_project_12.json`. Culvert parameters and
    per-XS modifiers (Exp/Cntr, #XS Ineff) are overlaid from the g01.
    """
    geom = parse_g01(project_dir / geometry_name)
    if len(geom.cross_sections) < 8:
        raise ValueError(f"expected ≥8 open-channel XS in {geometry_name}")

    flow = parse_unsteady_flow(project_dir / flow_name)
    if plan_name:
        plan_path = project_dir / plan_name
    else:
        plan_path = find_plan_file(project_dir, flow_name=flow_name)
    plan = parse_plan(plan_path) if plan_path and plan_path.is_file() else None

    fixture = _load_conspan_project()
    station_to_rm = {
        float(row["station"]): float(row["rm"]) for row in fixture["geometry_data"]
    }
    cross_sections = overlay_g01_cross_section_modifiers(
        load_all_conspan_cross_sections(),
        geom.cross_sections,
        station_to_rm=station_to_rm,
    )
    culvert_fields = build_culvert_fields(
        geom,
        fixture_rows=fixture.get("culvert_stations"),
    )
    if not culvert_fields.get("culvert_stations"):
        raise ValueError("ConSpan must define culvert_stations in g01 or fixture")

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
        theta=theta,
        unsteady_friction_slope_method=friction_method,
        num_slices=solver["num_slices"],
        max_spacing=solver["max_spacing"],
        structure_coupling_order=0,
        unsteady_structure_coupling_mode=coupling_mode,
        **culvert_fields,
    )
    return inputs.to_dict(), flow
