"""Parse HEC-RAS unsteady flow files (.uXX) for linked verify."""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class ParsedUnsteadyFlow:
    upstream_q_cfs: list[float]
    downstream_friction_slope: float | None
    initial_flow_cfs: float
    interval_seconds: float
    downstream_stage_hydrograph: list[float] = field(default_factory=list)
    downstream_rating_q: list[float] = field(default_factory=list)
    downstream_rating_wsel: list[float] = field(default_factory=list)
    observed_hwm: dict[float, float] = field(default_factory=dict)
    upstream_rm: float | None = None
    downstream_rm: float | None = None


def _parse_hydrograph_values(block_text: str, key: str, *, paired: bool = False) -> list[float]:
    """Parse `Flow Hydrograph=` / `Stage Hydrograph=` blocks (skip leading count)."""
    if key not in block_text:
        return []
    tail = block_text.split(key, 1)[1]
    count_match = re.search(r"^\s*(\d+)", tail)
    if not count_match:
        return []
    n = int(count_match.group(1))
    rest = tail[count_match.end() :]
    nums = [float(v) for v in re.findall(r"[-+]?\d*\.?\d+(?:[eE][-+]?\d+)?", rest)]
    if paired and len(nums) >= 2 * n:
        return [nums[i] for i in range(1, 2 * n, 2)]
    return nums[:n]


def _parse_observed_hwm_line(line: str, observed_hwm: dict[float, float]) -> None:
    if not line.startswith("Observed HWM="):
        return
    parts = [p.strip() for p in line.split("=", 1)[1].split(",")]
    if len(parts) < 3:
        return
    rm = float(re.sub(r"[^0-9.\-]", "", parts[2]))
    elev_token = next((p for p in parts[3:] if p), None)
    if elev_token:
        try:
            observed_hwm[rm] = float(elev_token)
        except ValueError:
            pass


def parse_unsteady_flow(path: Path) -> ParsedUnsteadyFlow:
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()

    upstream_q: list[float] = []
    ds_slope: float | None = None
    ds_stage: list[float] = []
    ds_rating_q: list[float] = []
    ds_rating_wsel: list[float] = []
    initial_q = 500.0
    saw_initial_flow_loc = False
    interval_seconds = 3600.0
    observed_hwm: dict[float, float] = {}
    upstream_rm: float | None = None
    downstream_rm: float | None = None

    i = 0
    while i < len(lines):
        line = lines[i].strip()
        i += 1
        if not line:
            continue

        if line.startswith("Initial Flow Loc="):
            saw_initial_flow_loc = True
            parts = line.split("=")[1].split(",")
            if len(parts) >= 4:
                initial_q = float(parts[3].strip())
                upstream_rm = float(re.sub(r"[^0-9.\-]", "", parts[2].strip()))
            continue

        if line.startswith("Boundary Location="):
            loc = line.split("=")[1]
            parts = [p.strip() for p in loc.split(",")]
            rm = float(re.sub(r"[^0-9.\-]", "", parts[2])) if len(parts) > 2 else None
            block: list[str] = []
            while i < len(lines):
                nxt = lines[i].strip()
                if nxt.startswith("Boundary Location=") or nxt.startswith("DSS File="):
                    break
                if nxt.startswith("Observed HWM="):
                    _parse_observed_hwm_line(nxt, observed_hwm)
                    i += 1
                    continue
                block.append(nxt)
                i += 1
            block_text = "\n".join(block)
            if "Flow Hydrograph=" in block_text:
                upstream_rm = rm
                upstream_q = _parse_hydrograph_values(block_text, "Flow Hydrograph=")
            if "Stage Hydrograph=" in block_text:
                downstream_rm = rm
                ds_stage = _parse_hydrograph_values(block_text, "Stage Hydrograph=")
            elif "Rating Curve=" in block_text:
                downstream_rm = rm
                ds_rating_q, ds_rating_wsel = _parse_rating_curve(block_text)
            elif "Friction Slope=" in block_text:
                downstream_rm = rm
                payload = block_text.split("Friction Slope=")[1].strip().split()[0]
                ds_slope = float(payload.split(",")[0])
            elif rm is not None and "Flow Hydrograph=" not in block_text:
                downstream_rm = rm
            if "Interval=" in block_text:
                token = block_text.split("Interval=")[1].split()[0]
                interval_seconds = _interval_to_seconds(token)
            continue

        if line.startswith("Interval="):
            interval_seconds = _interval_to_seconds(line.split("=")[1].strip())
            continue

        if line.startswith("Observed HWM="):
            _parse_observed_hwm_line(line, observed_hwm)
            continue

    if upstream_q and not saw_initial_flow_loc:
        initial_q = float(upstream_q[0])

    return ParsedUnsteadyFlow(
        upstream_q_cfs=upstream_q,
        downstream_friction_slope=ds_slope,
        downstream_stage_hydrograph=ds_stage,
        downstream_rating_q=ds_rating_q,
        downstream_rating_wsel=ds_rating_wsel,
        initial_flow_cfs=initial_q,
        interval_seconds=interval_seconds,
        observed_hwm=observed_hwm,
        upstream_rm=upstream_rm,
        downstream_rm=downstream_rm,
    )


def _parse_rating_curve(block_text: str) -> tuple[list[float], list[float]]:
    """Parse ``Rating Curve=`` block (HEC-RAS: stage, discharge pairs)."""
    if "Rating Curve=" not in block_text:
        return [], []
    tail = block_text.split("Rating Curve=", 1)[1]
    count_match = re.search(r"^\s*(\d+)", tail)
    if not count_match:
        return [], []
    n = int(count_match.group(1))
    rest = tail[count_match.end() :]
    nums = [float(v) for v in re.findall(r"[-+]?\d*\.?\d+(?:[eE][-+]?\d+)?", rest)]
    if len(nums) < 2 * n:
        return [], []
    stages = [nums[i] for i in range(0, 2 * n, 2)]
    flows = [nums[i] for i in range(1, 2 * n, 2)]
    return flows, stages


def _interval_to_seconds(token: str) -> float:
    token = token.upper()
    if token.endswith("HOUR"):
        return float(re.sub(r"[^0-9.]", "", token) or "1") * 3600.0
    if token.endswith("MIN"):
        return float(re.sub(r"[^0-9.]", "", token) or "1") * 60.0
    if token.endswith("SEC"):
        return float(re.sub(r"[^0-9.]", "", token) or "1")
    return 3600.0
