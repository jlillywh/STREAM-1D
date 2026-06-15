"""Hydrograph resampling and friction-slope downstream BC helpers."""

from __future__ import annotations


def resample_hydrograph(values: list[float], dt_from: float, dt_to: float) -> tuple[list[float], float]:
    """Linearly resample a uniform hydrograph onto a finer (or coarser) interval."""
    if not values:
        return [], dt_to
    if abs(dt_to - dt_from) < 1e-9 or len(values) == 1:
        return list(values), dt_from

    duration = (len(values) - 1) * dt_from
    n_out = max(2, int(round(duration / dt_to)) + 1)
    times_in = [i * dt_from for i in range(len(values))]
    out: list[float] = []
    for step in range(n_out):
        t = step * dt_to
        if t <= times_in[0]:
            out.append(values[0])
            continue
        if t >= times_in[-1]:
            out.append(values[-1])
            continue
        idx = int(t // dt_from)
        idx = min(idx, len(values) - 2)
        t0, t1 = times_in[idx], times_in[idx + 1]
        frac = (t - t0) / (t1 - t0) if t1 > t0 else 0.0
        out.append(values[idx] + frac * (values[idx + 1] - values[idx]))
    return out, dt_to


def friction_slope_downstream_wsel(
    cross_sections: list,
    q_cfs_series: list[float],
    friction_slope: float,
    *,
    sample_count: int = 24,
) -> list[float]:
    """
    Approximate RAS friction-slope downstream BC as normal-depth WSEL on the
    downstream cross section for each discharge.
    """
    import stream1d as st

    if not cross_sections or not q_cfs_series:
        return []

    ordered = sorted(cross_sections, key=lambda xs: xs.station)
    if len(ordered) == 1:
        pair = [ordered[0], ordered[0]]
    else:
        pair = [ordered[-2], ordered[-1]]

    q_min = min(q for q in q_cfs_series if q > 0) if any(q > 0 for q in q_cfs_series) else 1.0
    q_max = max(q_cfs_series) if q_cfs_series else q_min
    if q_max <= q_min:
        q_samples = [q_max]
    else:
        import math

        log_min = math.log(max(q_min, 1.0))
        log_max = math.log(max(q_max, 1.0))
        q_samples = sorted(
            {
                q_min,
                q_max,
                *[
                    math.exp(log_min + (log_max - log_min) * i / max(sample_count - 1, 1))
                    for i in range(sample_count)
                ],
            }
        )

    rating: list[tuple[float, float]] = []
    for q in q_samples:
        if q <= 0:
            continue
        steady = st.SteadyInputs(
            cross_sections=pair,
            flow_rate=q,
            downstream_bc_type=2,
            downstream_bc_slope=friction_slope,
            regime=0,
        )
        result = st.solve_steady(steady)
        rating.append((q, float(result["wsel"][-1])))

    if not rating:
        return [211.8] * len(q_cfs_series)

    rating.sort(key=lambda row: row[0])
    out: list[float] = []
    for q in q_cfs_series:
        q_eff = max(q, rating[0][0])
        if q_eff <= rating[0][0]:
            out.append(rating[0][1])
            continue
        if q_eff >= rating[-1][0]:
            out.append(rating[-1][1])
            continue
        for (q0, w0), (q1, w1) in zip(rating, rating[1:]):
            if q0 <= q_eff <= q1:
                frac = (q_eff - q0) / (q1 - q0) if q1 > q0 else 0.0
                out.append(w0 + frac * (w1 - w0))
                break
        else:
            out.append(rating[-1][1])
    return out
