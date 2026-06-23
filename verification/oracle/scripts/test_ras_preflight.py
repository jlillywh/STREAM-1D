#!/usr/bin/env python3
"""Tests for linked HEC-RAS preflight (hydrograph ordinate counts)."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))

from lib.hecras_plan_parser import (  # noqa: E402
    parse_simulation_duration_seconds,
    required_boundary_ordinals,
)
from lib.hecras_unsteady_parser import parse_unsteady_flow  # noqa: E402
from lib.ras_preflight import validate_linked_unsteady_project  # noqa: E402


def test_beaver_plan_duration_is_48h() -> None:
    plan = (ORACLE / "projects" / "beaver" / "beaver.p03").read_text(encoding="utf-8")
    duration = parse_simulation_duration_seconds(plan)
    assert duration is not None
    assert abs(duration - 48 * 3600) < 1.0


def test_beaver_u02_has_49_ordinals() -> None:
    u02 = ORACLE / "projects" / "beaver" / "beaver.u02"
    flow = parse_unsteady_flow(u02)
    assert len(flow.upstream_q_cfs) == 49
    assert max(flow.upstream_q_cfs) == 14000.0


def test_beaver_preflight_passes() -> None:
    project = ORACLE / "projects" / "beaver"
    errors = validate_linked_unsteady_project(
        project,
        plan_path=project / "beaver.p03",
        u02_path=project / "beaver.u02",
    )
    assert errors == []


def test_required_ordinals_formula() -> None:
    assert required_boundary_ordinals(duration_seconds=48 * 3600, interval_seconds=3600) == 49


def main() -> int:
    test_beaver_plan_duration_is_48h()
    test_beaver_u02_has_49_ordinals()
    test_beaver_preflight_passes()
    test_required_ordinals_formula()
    print("test_ras_preflight: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
