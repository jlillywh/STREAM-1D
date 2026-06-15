"""Write HEC-RAS unsteady flow files (.uXX) for linked oracle scenarios."""

from __future__ import annotations

import re
from pathlib import Path


def _format_hydrograph(values: list[float], *, per_line: int = 10) -> list[str]:
    lines: list[str] = []
    for i in range(0, len(values), per_line):
        chunk = values[i : i + per_line]
        lines.append("    " + "    ".join(f"{v:g}" for v in chunk))
    return lines


def _format_stage_hydrograph(values: list[float], *, per_line: int = 10) -> list[str]:
    """RAS 7.x stage lines use tighter spacing (see ConSpan.u01)."""
    lines: list[str] = []
    for i in range(0, len(values), per_line):
        chunk = values[i : i + per_line]
        lines.append("   " + "   ".join(f"{v:g}" for v in chunk))
    return lines


def write_q_stage_ras701_from_template(
    path: Path,
    template_path: Path,
    *,
    title: str,
    q_values: list[float],
    stage_values: list[float],
) -> Path:
    """
    Clone a RAS 7.01 unsteady flow file, replacing title and hydrographs only.

    Preserves boundary metadata, Met BC footer, and extended Boundary Location fields.
    Writes CRLF line endings for HEC-RAS on Windows.
    """
    if len(q_values) != len(stage_values):
        raise ValueError("q_values and stage_values must have the same length")

    lines = template_path.read_text(encoding="utf-8", errors="replace").splitlines()
    out: list[str] = []
    i = 0
    hydro_line = re.compile(r"^\s+[-+]?\d")

    while i < len(lines):
        line = lines[i]
        if line.startswith("Flow Title="):
            out.append(f"Flow Title={title}")
            i += 1
            continue
        if line.startswith("Flow Hydrograph="):
            out.append(f"Flow Hydrograph= {len(q_values)} ")
            out.extend(_format_hydrograph(q_values))
            i += 1
            while i < len(lines) and hydro_line.match(lines[i]):
                i += 1
            continue
        if line.startswith("Stage Hydrograph="):
            out.append(f"Stage Hydrograph= {len(stage_values)} ")
            out.extend(_format_stage_hydrograph(stage_values))
            i += 1
            while i < len(lines) and hydro_line.match(lines[i]):
                i += 1
            continue
        out.append(line)
        i += 1

    text = "\r\n".join(out) + "\r\n"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(text.encode("utf-8"))
    return path


def mild_ramp_hydrograph(
    *,
    num_intervals: int = 48,
    q_low: float = 600.0,
    q_high: float = 1000.0,
    hold_low_hours: int = 12,
    ramp_hours: int = 24,
) -> list[float]:
    """
    Trapezoid upstream Q: hold low → ramp to high → hold high (within num_intervals).

    Default 48 × 1HOUR: 12h @ 600 cfs, 24h ramp to 1000, 12h @ 1000 cfs.
    """
    if hold_low_hours + ramp_hours > num_intervals:
        raise ValueError("hold_low_hours + ramp_hours exceeds num_intervals")
    hold_high = num_intervals - hold_low_hours - ramp_hours
    out: list[float] = []
    for i in range(num_intervals):
        if i < hold_low_hours:
            out.append(float(q_low))
        elif i < hold_low_hours + ramp_hours:
            frac = (i - hold_low_hours) / max(ramp_hours - 1, 1)
            out.append(float(q_low + frac * (q_high - q_low)))
        else:
            out.append(float(q_high))
    return out


def write_constant_q_known_stage_u02(
    path: Path,
    *,
    title: str,
    river: str,
    reach: str,
    upstream_rm: float,
    downstream_rm: float,
    flow_cfs: float,
    downstream_stage_ft: float,
    num_intervals: int = 48,
    interval: str = "1HOUR",
    program_version: str = "5.00",
    observed_hwm: dict[float, float] | None = None,
) -> Path:
    """
    Constant upstream Q + fixed downstream stage hydrograph (Chunk 4 mild pattern).

    Matches reach_mild / conspan_unsteady_mild_linked forcing used by STREAM-1D oracle.
    """
    q_values = [float(flow_cfs)] * num_intervals
    stage_values = [float(downstream_stage_ft)] * num_intervals
    return write_q_stage_u02(
        path,
        title=title,
        river=river,
        reach=reach,
        upstream_rm=upstream_rm,
        downstream_rm=downstream_rm,
        q_values=q_values,
        stage_values=stage_values,
        initial_flow_cfs=flow_cfs,
        interval=interval,
        program_version=program_version,
        observed_hwm=observed_hwm,
    )


def write_q_stage_u02(
    path: Path,
    *,
    title: str,
    river: str,
    reach: str,
    upstream_rm: float,
    downstream_rm: float,
    q_values: list[float],
    stage_values: list[float],
    initial_flow_cfs: float | None = None,
    interval: str = "1HOUR",
    program_version: str = "5.00",
    observed_hwm: dict[float, float] | None = None,
) -> Path:
    """Write unsteady flow file with arbitrary Q and stage hydrographs."""
    if len(q_values) != len(stage_values):
        raise ValueError("q_values and stage_values must have the same length")
    num_intervals = len(q_values)
    q0 = float(initial_flow_cfs if initial_flow_cfs is not None else q_values[0])

    river_field = f"{river:<16},{reach:<16}"
    lines = [
        f"Flow Title={title}",
        f"Program Version={program_version}",
        "Use Restart= 0 ",
        f"Initial Flow Loc={river_field},{upstream_rm:g}  ,{q0:g}",
        f"Boundary Location={river_field},{upstream_rm:g}  ,        ,                ,                ,                ",
        f"Interval={interval}",
        f"Flow Hydrograph= {num_intervals} ",
        *_format_hydrograph(q_values),
        f"Boundary Location={river_field},{downstream_rm:g}  ,        ,                ,                ,                ",
        f"Stage Hydrograph= {num_intervals} ",
        *_format_hydrograph(stage_values),
    ]

    if observed_hwm:
        for rm in sorted(observed_hwm.keys(), reverse=True):
            wsel = observed_hwm[rm]
            lines.append(f"Observed HWM={river_field},{rm:g}  ,,{wsel:g}")

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    return path
