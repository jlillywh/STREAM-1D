#!/usr/bin/env python3
"""Write simple_channel.u02 — constant Q upstream, friction-slope downstream."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))

from lib.hecras_text_io import assert_compact_ras_text, write_ras_lines  # noqa: E402
from lib.patch_simple_channel_u02 import patch_friction_slope_ds  # noqa: E402

OUT = ORACLE / "projects" / "simple_channel" / "simple_channel.u02"

# Plan: 01JAN2000 00:00 -> 03JAN2000 00:00, Computation Interval=1HOUR => hours 0..48 (49 values).
NUM = 49
Q = 150.0
FRICTION_SLOPE = 0.001


def _hecras_table_lines(values: list[float], *, per_line: int = 10) -> list[str]:
    """HEC-RAS .uXX tables: values-only, 8 chars per field (reach_mild pattern)."""
    lines: list[str] = []
    for i in range(0, len(values), per_line):
        row = values[i : i + per_line]
        lines.append(
            "".join(f"{v:8.2f}" if abs(v) < 1e7 else f"{v:8.1f}" for v in row)
        )
    return lines


def build_u02_lines() -> list[str]:
    """Compact layout like reach_mild.u02; DS Normal Depth with RAS 7 `,0` flag."""
    q_vals = [Q] * NUM
    return [
        "Flow Title=Simple channel constant Q unsteady",
        "Program Version=5.00",
        "Use Restart= 0 ",
        f"Initial Flow Loc=Simple Creek    ,Trapezoid Reach ,3.0    ,{Q:g}",
        "Boundary Location=Simple Creek    ,Trapezoid Reach ,3.0    ,        ,                ,                ,                ",
        "Interval=1HOUR",
        f"Flow Hydrograph= {NUM} ",
        *_hecras_table_lines(q_vals),
        "Boundary Location=Simple Creek    ,Trapezoid Reach ,0.0    ,        ,                ,                ,                ",
        f"Friction Slope={FRICTION_SLOPE:g},0",
    ]


def write_u02_bytes(path: Path = OUT) -> None:
    lines = build_u02_lines()
    write_ras_lines(path, lines)
    patch_friction_slope_ds(path, friction_slope=FRICTION_SLOPE)
    assert_compact_ras_text(path)


if __name__ == "__main__":
    write_u02_bytes()
    friction = next(
        ln for ln in OUT.read_text(encoding="utf-8").splitlines() if ln.startswith("Friction Slope=")
    )
    print(f"Wrote {OUT} ({len(build_u02_lines())} seed records)")
    print(f"  downstream: {friction}")
