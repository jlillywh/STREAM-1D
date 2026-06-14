#!/usr/bin/env python3
"""Write reach_mild.u02 with HEC-RAS-compatible compact CRLF format."""

from __future__ import annotations

from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
OUT = ORACLE / "projects" / "reach_mild" / "reach_mild.u02"

# Plan: 01JAN2000 00:00 -> 03JAN2000 00:00, Computation Interval=1HOUR => hours 0..48 (49 values).
NUM = 49
Q = 1000.0
STAGE = 30.51


def _hecras_table_lines(values: list[float], *, per_line: int = 10) -> list[str]:
    """HEC-RAS .uXX tables: values-only, 8 chars per field, 10 fields per line."""
    lines: list[str] = []
    for i in range(0, len(values), per_line):
        row = values[i : i + per_line]
        lines.append(
            "".join(f"{v:8.2f}" if abs(v) < 1e7 else f"{v:8.1f}" for v in row)
        )
    return lines


def build_u02_lines() -> list[str]:
    q_vals = [Q] * NUM
    stage_vals = [STAGE] * NUM
    return [
        "Flow Title=Reach mild constant Q unsteady",
        "Program Version=5.00",
        "Use Restart= 0 ",
        f"Initial Flow Loc=Spring Creek    ,Culvrt Reach    ,20.535  ,{Q:g}",
        "Boundary Location=Spring Creek    ,Culvrt Reach    ,20.535  ,        ,                ,                ,                ",
        "Interval=1HOUR",
        f"Flow Hydrograph= {NUM} ",
        *_hecras_table_lines(q_vals),
        "Boundary Location=Spring Creek    ,Culvrt Reach    ,20.0    ,        ,                ,                ,                ",
        "Interval=1HOUR",
        f"Stage Hydrograph= {NUM} ",
        *_hecras_table_lines(stage_vals),
    ]


def write_u02_bytes(path: Path = OUT) -> None:
    lines = build_u02_lines()
    path.write_bytes(("\r\n".join(lines) + "\r\n").encode("utf-8"))


if __name__ == "__main__":
    write_u02_bytes()
    print(f"Wrote {OUT} (8-char fixed-width stage values, {NUM} @ {STAGE:g} ft)")
