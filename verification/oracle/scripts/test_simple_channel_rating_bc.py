#!/usr/bin/env python3
"""Smoke tests for rating-curve downstream BC parsing and mapping."""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))

from lib.hecras_text_io import write_ras_lines  # noqa: E402
from lib.hecras_unsteady_parser import parse_unsteady_flow  # noqa: E402
from lib.downstream_bc_mapping import downstream_bc_from_flow  # noqa: E402


def test_parse_rating_curve_block() -> None:
    lines = [
        "Flow Title=test",
        "Initial Flow Loc=Simple Creek    ,Trapezoid Reach ,3.0    ,150",
        "Boundary Location=Simple Creek    ,Trapezoid Reach ,3.0    ,        ,                ,                ,                ",
        "Flow Hydrograph= 2 ",
        " 150.00  150.00",
        "Boundary Location=Simple Creek    ,Trapezoid Reach ,0.0    ,        ,                ,                ,                ",
        "Rating Curve= 3 ",
        "  100.50  100.00  101.00  150.00  101.50  200.00",
    ]
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "test.u03"
        write_ras_lines(path, lines)
        flow = parse_unsteady_flow(path)
    assert flow.downstream_rating_q == [100.0, 150.0, 200.0]
    assert flow.downstream_rating_wsel == [100.5, 101.0, 101.5]
    bc = downstream_bc_from_flow(flow, num_steps=3)
    assert bc.bc_type == 3
    assert bc.rating_q == [100.0, 150.0, 200.0]
    assert bc.rating_wsel == [100.5, 101.0, 101.5]
    print("OK: rating curve parse + BC mapping")


def test_simple_channel_u03_if_present() -> None:
    u03 = ORACLE / "projects" / "simple_channel" / "simple_channel.u03"
    if not u03.is_file():
        print("SKIP: simple_channel.u03 not generated yet")
        return
    flow = parse_unsteady_flow(u03)
    assert flow.downstream_rating_q, "expected rating Q from u03"
    assert flow.downstream_rating_wsel, "expected rating WSEL from u03"
    bc = downstream_bc_from_flow(flow, num_steps=len(flow.upstream_q_cfs))
    assert bc.bc_type == 3
    print(f"OK: u03 has {len(flow.downstream_rating_q)} rating pairs")


def main() -> int:
    test_parse_rating_curve_block()
    test_simple_channel_u03_if_present()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
