"""Shared downstream BC and steady warm-start helpers for linked unsteady mappers."""

from __future__ import annotations

from typing import Any

import stream1d as st

from .downstream_bc_mapping import DownstreamBcMapping, downstream_bc_from_flow
from .hecras_unsteady_parser import ParsedUnsteadyFlow
from .rating_curve_ops import interpolate_rating_wsel

__all__ = [
    "DownstreamBcMapping",
    "downstream_bc_from_flow",
    "resample_series",
    "steady_initial_wsel",
]


def resample_series(values: list[float], n_out: int) -> list[float]:
    if not values:
        return [0.0] * n_out
    if len(values) == n_out:
        return list(values)
    if len(values) == 1:
        return [values[0]] * n_out
    out: list[float] = []
    for i in range(n_out):
        t = i / max(n_out - 1, 1)
        idx = t * (len(values) - 1)
        lo = int(idx)
        hi = min(lo + 1, len(values) - 1)
        frac = idx - lo
        out.append(values[lo] + frac * (values[hi] - values[lo]))
    return out


def steady_initial_wsel(
    cross_sections: list[st.CrossSection],
    flow_cfs: float,
    downstream_bc_type: int | None,
    downstream_bc_slope: float | None,
    downstream_wsel: float,
    *,
    max_spacing: float,
    num_slices: int,
    structure_fields: dict[str, Any] | None = None,
    coeff_contraction: float | None = None,
    coeff_expansion: float | None = None,
    downstream_bc_rating_q: list[float] | None = None,
    downstream_bc_rating_wsel: list[float] | None = None,
) -> list[float]:
    bc_type = downstream_bc_type or 0
    kwargs: dict[str, Any] = {"downstream_bc_type": bc_type}
    if bc_type == 2:
        kwargs["downstream_bc_slope"] = downstream_bc_slope or 0.001
    elif bc_type == 3:
        kwargs["downstream_bc_rating_q"] = downstream_bc_rating_q or []
        kwargs["downstream_bc_rating_wsel"] = downstream_bc_rating_wsel or []
        kwargs["downstream_wsel"] = interpolate_rating_wsel(
            flow_cfs,
            downstream_bc_rating_q or [],
            downstream_bc_rating_wsel or [],
        )
    else:
        kwargs["downstream_wsel"] = downstream_wsel
    if coeff_contraction is not None:
        kwargs["coeff_contraction"] = coeff_contraction
    if coeff_expansion is not None:
        kwargs["coeff_expansion"] = coeff_expansion
    if structure_fields:
        kwargs.update(structure_fields)
    steady = st.SteadyInputs(
        cross_sections=cross_sections,
        flow_rate=flow_cfs,
        regime=0,
        num_slices=num_slices,
        max_spacing=max_spacing,
        **kwargs,
    )
    return list(st.solve_steady(steady)["wsel"])
