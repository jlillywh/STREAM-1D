"""Preflight checks for linked HEC-RAS headless runs."""

from __future__ import annotations

import re
from pathlib import Path

from .hecras_plan_parser import (
    parse_simulation_duration_seconds,
    required_boundary_ordinals,
)
from .hecras_unsteady_parser import parse_unsteady_flow


def _upstream_hydrograph_declared_count(u02_text: str) -> int | None:
    match = re.search(r"Flow Hydrograph=\s*(\d+)", u02_text)
    return int(match.group(1)) if match else None


def validate_unsteady_flow_hydrographs(
    plan_path: Path,
    u02_path: Path,
) -> list[str]:
    """
    Validate upstream flow hydrograph ordinate counts against plan simulation span.

    Returns human-readable error strings (empty list = ok).
    """
    errors: list[str] = []
    plan_text = plan_path.read_text(encoding="utf-8", errors="replace")
    duration_seconds = parse_simulation_duration_seconds(plan_text)
    if duration_seconds is None:
        errors.append(f"{plan_path.name}: missing or empty Simulation Date line")
        return errors

    flow = parse_unsteady_flow(u02_path)
    if not flow.upstream_q_cfs:
        errors.append(f"{u02_path.name}: no upstream Flow Hydrograph parsed")
        return errors

    required = required_boundary_ordinals(
        duration_seconds=duration_seconds,
        interval_seconds=flow.interval_seconds,
    )
    declared = _upstream_hydrograph_declared_count(
        u02_path.read_text(encoding="utf-8", errors="replace")
    )
    actual = len(flow.upstream_q_cfs)
    duration_hr = duration_seconds / 3600.0
    interval_hr = flow.interval_seconds / 3600.0

    if declared is not None and declared != actual:
        errors.append(
            f"{u02_path.name}: Flow Hydrograph declares {declared} values but parser "
            f"found {actual} (check for jammed decimals without spaces)"
        )

    if actual != required:
        errors.append(
            f"{u02_path.name}: upstream hydrograph has {actual} ordinates but plan "
            f"{plan_path.name} requires {required} for "
            f"{duration_hr:g} h simulation @ {interval_hr:g} h input interval "
            f"(duration/interval + 1)"
        )
    return errors


def validate_linked_unsteady_project(
    project_dir: Path,
    *,
    plan_path: Path,
    u02_path: Path,
) -> list[str]:
    """Run linked-project preflight checks before headless HEC-RAS."""
    errors: list[str] = []
    if not plan_path.is_file():
        errors.append(f"plan file not found: {plan_path}")
    if not u02_path.is_file():
        errors.append(f"unsteady flow file not found: {u02_path}")
    if errors:
        return errors

    prj_files = sorted(project_dir.glob("*.prj"))
    if not prj_files:
        errors.append(f"{project_dir.name}: missing .prj file (required for Ras.exe -c)")

    errors.extend(validate_unsteady_flow_hydrographs(plan_path, u02_path))
    return errors
