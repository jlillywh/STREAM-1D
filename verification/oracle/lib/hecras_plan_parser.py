"""Parse HEC-RAS plan files (.pXX) for linked verify."""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path


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
