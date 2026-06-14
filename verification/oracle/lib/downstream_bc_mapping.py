"""Map parsed HEC-RAS unsteady flow to STREAM-1D downstream BC fields."""

from __future__ import annotations

from dataclasses import dataclass

from .hecras_unsteady_parser import ParsedUnsteadyFlow
from .rating_curve_ops import interpolate_rating_wsel


@dataclass(frozen=True)
class DownstreamBcMapping:
    wsel_series: list[float]
    bc_type: int | None
    slope: float | None
    rating_q: list[float] | None = None
    rating_wsel: list[float] | None = None


def downstream_bc_from_flow(flow: ParsedUnsteadyFlow, num_steps: int) -> DownstreamBcMapping:
    if flow.downstream_rating_q and flow.downstream_rating_wsel:
        initial = interpolate_rating_wsel(flow.initial_flow_cfs, flow.downstream_rating_q, flow.downstream_rating_wsel)
        return DownstreamBcMapping(
            wsel_series=[initial] * num_steps,
            bc_type=3,
            slope=None,
            rating_q=list(flow.downstream_rating_q),
            rating_wsel=list(flow.downstream_rating_wsel),
        )

    if flow.downstream_friction_slope is not None and flow.downstream_friction_slope > 0:
        return DownstreamBcMapping(
            wsel_series=[0.0] * num_steps,
            bc_type=2,
            slope=flow.downstream_friction_slope,
        )

    if flow.downstream_stage_hydrograph:
        from .unsteady_bc import resample_series

        return DownstreamBcMapping(
            wsel_series=resample_series(flow.downstream_stage_hydrograph, num_steps),
            bc_type=0,
            slope=None,
        )

    ds_rm = flow.downstream_rm
    if ds_rm is not None and ds_rm in flow.observed_hwm:
        wsel = flow.observed_hwm[ds_rm]
        return DownstreamBcMapping([wsel] * num_steps, 0, None)

    if flow.observed_hwm:
        wsel = min(flow.observed_hwm.values())
        return DownstreamBcMapping([wsel] * num_steps, 0, None)

    return DownstreamBcMapping([30.0] * num_steps, 0, None)
