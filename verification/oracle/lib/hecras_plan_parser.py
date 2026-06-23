"""Parse HEC-RAS plan files (.pXX) for linked verify."""

from __future__ import annotations

import re
from dataclasses import dataclass
from datetime import datetime, timedelta
from pathlib import Path

_HEC_MONTHS = {
    "JAN": 1,
    "FEB": 2,
    "MAR": 3,
    "APR": 4,
    "MAY": 5,
    "JUN": 6,
    "JUL": 7,
    "AUG": 8,
    "SEP": 9,
    "OCT": 10,
    "NOV": 11,
    "DEC": 12,
}


@dataclass
class ParsedPlan:
    plan_number: str
    title: str
    geometry_file: str
    flow_file: str
    computation_interval_seconds: float
    output_interval_seconds: float
    unsteady_theta: float
    unsteady_friction_slope_method: int
    run_unet: bool


def _parse_hec_date_token(token: str) -> datetime:
    """Parse ``10FEB1990`` style HEC-RAS date tokens."""
    token = token.strip().upper()
    if not token:
        raise ValueError("empty HEC date token")
    match = re.match(r"^(\d{1,2})([A-Z]{3})(\d{4})$", token)
    if not match:
        raise ValueError(f"invalid HEC date token: {token!r}")
    day = int(match.group(1))
    month = _HEC_MONTHS[match.group(2)]
    year = int(match.group(3))
    return datetime(year, month, day)


def _parse_hec_time_token(token: str) -> timedelta:
    """Parse ``0000`` / ``2400`` / ``0:00`` HEC-RAS time tokens as offset from midnight."""
    token = token.strip().upper().replace(":", "")
    if not token:
        return timedelta()
    if token == "2400":
        return timedelta(days=1)
    if len(token) <= 2:
        hours = int(token)
        minutes = 0
    else:
        hours = int(token[:-2])
        minutes = int(token[-2:])
    return timedelta(hours=hours, minutes=minutes)


def parse_simulation_duration_seconds(plan_text: str) -> float | None:
    """
    Return simulation span in seconds from ``Simulation Date=start,end`` line.

    HEC-RAS ordinate rule: boundary count = duration / input interval + 1.
    """
    for raw in plan_text.splitlines():
        line = raw.strip()
        if not line.startswith("Simulation Date="):
            continue
        payload = line.split("=", 1)[1]
        parts = [p.strip() for p in payload.split(",")]
        if len(parts) < 4 or not parts[0] or not parts[2]:
            return None
        start = _parse_hec_date_token(parts[0]) + _parse_hec_time_token(parts[1])
        end = _parse_hec_date_token(parts[2]) + _parse_hec_time_token(parts[3])
        duration = end - start
        if duration.total_seconds() <= 0:
            raise ValueError(f"non-positive simulation duration in plan: {line}")
        return duration.total_seconds()
    return None


def required_boundary_ordinals(*, duration_seconds: float, interval_seconds: float) -> int:
    """HEC-RAS boundary hydrograph ordinate count (duration / interval + 1)."""
    if interval_seconds <= 0:
        raise ValueError("interval_seconds must be positive")
    intervals = duration_seconds / interval_seconds
    if abs(intervals - round(intervals)) > 1e-6:
        raise ValueError(
            f"simulation duration {duration_seconds}s is not an integer multiple of "
            f"interval {interval_seconds}s"
        )
    return int(round(intervals)) + 1


def _interval_to_seconds(token: str) -> float:
    token = token.strip().upper()
    if token.endswith("HOUR"):
        return float(re.sub(r"[^0-9.]", "", token) or "1") * 3600.0
    if token.endswith("MIN"):
        return float(re.sub(r"[^0-9.]", "", token) or "1") * 60.0
    if token.endswith("SEC"):
        return float(re.sub(r"[^0-9.]", "", token) or "1")
    return 3600.0


def parse_plan(path: Path) -> ParsedPlan:
    text = path.read_text(encoding="utf-8", errors="replace")
    m = re.match(r"\.p(\d+)$", path.suffix.lower())
    plan_number = m.group(1) if m else "01"

    title = ""
    geometry_file = "g01"
    flow_file = "f01"
    comp_interval = 3600.0
    output_interval = 3600.0
    theta = 1.0
    unsteady_friction = 2
    run_unet = False

    for raw in text.splitlines():
        line = raw.strip()
        if line.startswith("Plan Title="):
            title = line.split("=", 1)[1].strip()
        elif line.startswith("Geom File="):
            geometry_file = line.split("=", 1)[1].strip()
        elif line.startswith("Flow File="):
            flow_file = line.split("=", 1)[1].strip()
        elif line.startswith("Computation Interval="):
            comp_interval = _interval_to_seconds(line.split("=", 1)[1])
        elif line.startswith("Output Interval="):
            output_interval = _interval_to_seconds(line.split("=", 1)[1])
        elif line.startswith("UNET Theta="):
            try:
                theta = float(line.split("=", 1)[1].strip().split()[0])
            except ValueError:
                pass
        elif line.startswith("Unsteady Friction Slope Method="):
            try:
                unsteady_friction = int(line.split("=", 1)[1].strip().split()[0])
            except ValueError:
                pass
        elif line.startswith("Run UNet="):
            run_unet = line.split("=", 1)[1].strip() not in ("0", "")

    return ParsedPlan(
        plan_number=plan_number,
        title=title,
        geometry_file=geometry_file,
        flow_file=flow_file,
        computation_interval_seconds=comp_interval,
        output_interval_seconds=output_interval,
        unsteady_theta=theta,
        unsteady_friction_slope_method=unsteady_friction,
        run_unet=run_unet,
    )


def find_plan_file(project_dir: Path, *, flow_name: str | None = None) -> Path | None:
    def _plan_num(path: Path) -> int:
        m = re.match(r"\.p(\d+)$", path.suffix.lower())
        return int(m.group(1)) if m else 0

    if flow_name:
        m = re.match(r"\.u(\d+)$", Path(flow_name).suffix.lower())
        if m:
            stem = project_dir.name
            candidate = project_dir / f"{stem}.p{m.group(1).zfill(2)}"
            if candidate.is_file():
                return candidate

    plans = [p for p in project_dir.iterdir() if p.is_file() and re.match(r"\.p\d+$", p.suffix.lower())]
    if not plans:
        return None
    return max(plans, key=_plan_num)
