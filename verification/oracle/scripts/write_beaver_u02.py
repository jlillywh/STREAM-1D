#!/usr/bin/env python3
"""Rewrite beaver.u02 in RAS 7.x-compatible layout (CRLF, 8-char hydrograph fields)."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))

from lib.hecras_plan_parser import (  # noqa: E402
    parse_simulation_duration_seconds,
    required_boundary_ordinals,
)
from lib.hecras_text_io import write_ras_lines  # noqa: E402

PROJECT = ORACLE / "projects" / "beaver"
U02 = PROJECT / "beaver.u02"
P03 = PROJECT / "beaver.p03"

RIVER = "Beaver Creek"
REACH = "Kentwood"
UPSTREAM_RM = 5.99
DOWNSTREAM_RM = 5.0
INITIAL_Q = 500.0
FRICTION_SLOPE = 0.002

# Canonical 100-value hydrograph from the bundled Beaver Creek example (jammed lines repaired).
BEAVER_Q_100: list[float] = [
    317.58, 429.91, 615.75, 873.08, 1199.07, 1590.14, 2042.03, 2549.76, 3107.79, 3710,
    4349.79, 5020.14, 5713.72, 6422.94, 7140, 7857.06, 8566.28, 9259.86, 9930.21, 10570,
    11172.21, 11730.24, 12237.97, 12689.86, 13080.93, 13406.92, 13664.25, 13850.09, 13962.42, 14000,
    13962.42, 13850.09, 13664.25, 13406.92, 13080.93, 12689.86, 12237.97, 11730.24, 11172.21, 10570,
    9930.21, 9259.86, 8566.28, 7857.06, 7487.52, 7136.01, 6801.64, 6483.58, 6181.02, 5893.23,
    5619.47, 5359.06, 5111.35, 4875.72, 4651.59, 4438.38, 4235.58, 4042.66, 3859.15, 3684.6,
    3518.55, 3360.6, 3210.36, 3067.44, 2931.5, 2802.18, 2679.18, 2562.17, 2450.87, 2344.99,
    2244.28, 2148.48, 2057.36, 1970.67, 1888.22, 1809.79, 1735.18, 1664.21, 1596.69, 1532.48,
    1471.39, 1413.29, 1358.02, 1305.44, 1255.43, 1207.86, 1162.61, 1119.56, 1078.62, 1039.67,
    1002.62, 967.38, 933.85, 901.96, 871.63, 842.78, 815.33, 789.22, 764.39, 740.76,
]


def _river_reach_field() -> str:
    return f"{RIVER:<16},{REACH:<16}"


def _fmt_rm_field(rm: float) -> str:
    if rm == int(rm):
        text = f"{int(rm)}.0"
    else:
        text = f"{rm:g}"
    return f"{text}    ,"


def _hecras_field(value: float) -> str:
    """One 8-character HEC-RAS hydrograph field (reach_mild / GUI export rule)."""
    if abs(value) >= 10000:
        return f"{value:8.1f}"
    return f"{value:8.2f}"


def _hecras_table_lines(values: list[float], *, per_line: int = 10) -> list[str]:
    """HEC-RAS .uXX tables: 8-char fixed-width fields, 10 per line (see write_reach_mild_u02.py)."""
    lines: list[str] = []
    for i in range(0, len(values), per_line):
        row = values[i : i + per_line]
        line = "".join(_hecras_field(v) for v in row)
        if any(len(_hecras_field(v)) != 8 for v in row):
            bad = [(v, _hecras_field(v), len(_hecras_field(v))) for v in row if len(_hecras_field(v)) != 8]
            raise ValueError(f"hydrograph field width != 8: {bad[:3]}")
        lines.append(line)
    return lines


def _observed_hwm_lines(text: str) -> list[str]:
    return [ln for ln in text.splitlines() if ln.startswith("Observed HWM=")]


def build_beaver_u02(*, q_values: list[float]) -> list[str]:
    river_field = _river_reach_field()
    lines = [
        "Flow Title=Unsteady flow data -  Smaller Event",
        "Program Version=5.00",
        "Use Restart= 0 ",
        f"Initial Flow Loc={river_field},{_fmt_rm_field(UPSTREAM_RM)}{INITIAL_Q:g}",
        f"Boundary Location={river_field},{_fmt_rm_field(UPSTREAM_RM)}        ,                ,                ,                ",
        "Interval=1HOUR",
        f"Flow Hydrograph= {len(q_values)} ",
        *_hecras_table_lines(q_values),
        f"Boundary Location={river_field},{_fmt_rm_field(DOWNSTREAM_RM)}        ,                ,                ,                ",
        f"Friction Slope={FRICTION_SLOPE:g}",
    ]
    return lines


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--ordinates",
        type=int,
        default=0,
        help="Upstream hydrograph length (0 = trim to plan simulation span)",
    )
    args = parser.parse_args()

    old_text = U02.read_text(encoding="utf-8", errors="replace") if U02.is_file() else ""

    plan_text = P03.read_text(encoding="utf-8")
    duration = parse_simulation_duration_seconds(plan_text)
    if duration is None:
        print("ERROR: beaver.p03 missing Simulation Date", file=sys.stderr)
        return 1
    required = required_boundary_ordinals(duration_seconds=duration, interval_seconds=3600.0)
    q_count = args.ordinates if args.ordinates > 0 else required
    q_values = BEAVER_Q_100[:q_count]
    if len(q_values) < q_count:
        print(f"ERROR: canonical hydrograph has {len(BEAVER_Q_100)} values but {q_count} requested", file=sys.stderr)
        return 1

    lines = build_beaver_u02(q_values=q_values)
    lines.extend(_observed_hwm_lines(old_text))
    write_ras_lines(U02, lines)
    print(f"Wrote {U02} ({len(q_values)} upstream ordinates, 8-char hydrograph fields, CRLF)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
