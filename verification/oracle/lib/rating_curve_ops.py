"""Build downstream rating curves for linked unsteady verify."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import stream1d as st


def build_channel_rating_curve(
    cross_sections: list,
    q_samples: list[float],
    *,
    friction_slope: float = 0.001,
) -> tuple[list[float], list[float]]:
    """
    Build a stage-discharge rating on the downstream cross section.

    Uses a two-node steady reach with friction-slope downstream BC (same geometry
    as ``hydrograph_ops.friction_slope_downstream_wsel``) so the curve is physical
    for the bundled channel.
    """
    import stream1d as st

    if not cross_sections or not q_samples:
        return [], []

    ordered = sorted(cross_sections, key=lambda xs: xs.station)
    pair = [ordered[-2], ordered[-1]] if len(ordered) >= 2 else [ordered[0], ordered[0]]

    rating_q: list[float] = []
    rating_wsel: list[float] = []
    for q in sorted({max(float(q), 1.0) for q in q_samples}):
        steady = st.SteadyInputs(
            cross_sections=pair,
            flow_rate=q,
            downstream_bc_type=2,
            downstream_bc_slope=friction_slope,
            regime=0,
        )
        result = st.solve_steady(steady)
        rating_q.append(q)
        rating_wsel.append(float(result["wsel"][-1]))

    return rating_q, rating_wsel


def interpolate_rating_wsel(q: float, rating_q: list[float], rating_wsel: list[float]) -> float:
    """Linear interpolate WSEL from rating curve (user units)."""
    if not rating_q or not rating_wsel:
        return 0.0
    pairs = sorted(zip(rating_q, rating_wsel), key=lambda row: row[0])
    if q <= pairs[0][0]:
        return pairs[0][1]
    if q >= pairs[-1][0]:
        return pairs[-1][1]
    for (q0, w0), (q1, w1) in zip(pairs, pairs[1:]):
        if q0 <= q <= q1:
            if q1 <= q0:
                return w0
            frac = (q - q0) / (q1 - q0)
            return w0 + frac * (w1 - w0)
    return pairs[-1][1]
