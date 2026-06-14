#!/usr/bin/env python3
"""Generate simple_channel ramp Q hydrograph and HEC-RAS u04/u05 + p04/p05 files."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
PROJECT = ORACLE / "projects" / "simple_channel"
RATING_JSON = PROJECT / "rating_curve_ds.json"

NUM_STEPS = 49
INITIAL_Q_CFS = 100.0
LOW_Q_CFS = 100.0
HIGH_Q_CFS = 200.0
RAMP_UP_END_HR = 12
HOLD_HIGH_END_HR = 24
RAMP_DOWN_END_HR = 36


def ramp_hydrograph_cfs() -> list[float]:
    """49 hourly values: 100 → 200 (12 hr), hold, 200 → 100 (12 hr), hold."""
    values: list[float] = []
    for hour in range(NUM_STEPS):
        if hour <= RAMP_UP_END_HR:
            frac = hour / RAMP_UP_END_HR
            values.append(LOW_Q_CFS + (HIGH_Q_CFS - LOW_Q_CFS) * frac)
        elif hour <= HOLD_HIGH_END_HR:
            values.append(HIGH_Q_CFS)
        elif hour <= RAMP_DOWN_END_HR:
            frac = (hour - HOLD_HIGH_END_HR) / (RAMP_DOWN_END_HR - HOLD_HIGH_END_HR)
            values.append(HIGH_Q_CFS - (HIGH_Q_CFS - LOW_Q_CFS) * frac)
        else:
            values.append(LOW_Q_CFS)
    return values


def _format_hydrograph_block(values: list[float]) -> list[str]:
    lines = [f"Flow Hydrograph= {len(values)} "]
    row: list[str] = []
    for value in values:
        row.append(f"{value:7.2f}")
        if len(row) == 10:
            lines.append("  " + "  ".join(row))
            row = []
    if row:
        lines.append("  " + "  ".join(row))
    return lines


def _format_rating_curve_block(rating_q: list[float], rating_wsel: list[float]) -> list[str]:
    pairs = list(zip(rating_wsel, rating_q))
    lines = [f"Rating Curve= {len(pairs)} "]
    row: list[str] = []
    for stage, discharge in pairs:
        row.append(f"{stage:8.2f}{discharge:8.2f}")
        if len(row) == 5:
            lines.append("  " + "  ".join(row))
            row = []
    if row:
        lines.append("  " + "  ".join(row))
    return lines


def _flow_header(title: str) -> list[str]:
    return [
        f"Flow Title={title}",
        "Program Version=7.01",
        "Use Restart= 0 ",
        f"Initial Flow Loc=Simple Creek    ,Trapezoid Reach ,3.0    ,{INITIAL_Q_CFS:.0f}",
        "Boundary Location=Simple Creek    ,Trapezoid Reach ,3.0    ,        ,                ,                ,                ",
        "Interval=1HOUR",
        "Use Fixed Start Time=False",
        "Fixed Start Date/Time=,",
    ]


def _write_ras_text(path: Path, lines: list[str]) -> None:
    """Write HEC-RAS text with Windows CRLF (required for reliable GUI reads)."""
    path.write_bytes(("\r\n".join(lines) + "\r\n").encode("utf-8"))


def write_u04_friction(project_dir: Path, hydrograph: list[float]) -> Path:
    """Friction-slope DS layout matches working simple_channel.u02 (no DSS before DS BC)."""
    lines = [
        "Flow Title=Simple channel ramp Q unsteady (friction DS)",
        "Program Version=7.01",
        "Use Restart= 0 ",
        f"Initial Flow Loc=Simple Creek    ,Trapezoid Reach ,3.0    ,{INITIAL_Q_CFS:.0f}",
        "Boundary Location=Simple Creek    ,Trapezoid Reach ,3.0    ,        ,                ,                ,                ",
        "Interval=1HOUR",
        "Use Fixed Start Time=False",
        "Fixed Start Date/Time=,",
    ]
    lines.extend(_format_hydrograph_block(hydrograph))
    lines.extend(
        [
            "Boundary Location=Simple Creek    ,Trapezoid Reach ,0.0    ,        ,                ,                ,                ",
            "Friction Slope=0.001,0",
            "DSS File=",
            "Use DSS=False",
        ]
    )
    path = project_dir / "simple_channel.u04"
    _write_ras_text(path, lines)
    return path


def write_u05_rating(project_dir: Path, hydrograph: list[float]) -> Path:
    data = json.loads(RATING_JSON.read_text(encoding="utf-8"))
    rating_q = [float(v) for v in data["rating_q_cfs"]]
    rating_wsel = [float(v) for v in data["rating_wsel_ft"]]

    lines = _flow_header("Simple channel ramp Q unsteady (rating DS)")
    lines.extend(_format_hydrograph_block(hydrograph))
    lines.extend(
        [
            "DSS File=",
            "Use DSS=False",
            "Boundary Location=Simple Creek    ,Trapezoid Reach ,0.0    ,        ,                ,                ,                ",
        ]
    )
    lines.extend(_format_rating_curve_block(rating_q, rating_wsel))

    path = project_dir / "simple_channel.u05"
    _write_ras_text(path, lines)
    return path


def write_plan(
    project_dir: Path,
    *,
    plan_name: str,
    title: str,
    short_id: str,
    flow_file: str,
) -> Path:
    lines = [
        f"Plan Title={title}",
        "Program Version=7.01",
        f"Short Identifier={short_id}",
        "Simulation Date=01JAN2000,0000,03JAN2000,0000",
        "Geom File=g01",
        f"Flow File={flow_file}",
        "Subcritical Flow",
        "Friction Slope Method= 1 ",
        "Unsteady Friction Slope Method= 2 ",
        "Computation Interval=1HOUR",
        "Output Interval=1HOUR",
        "Write Detailed= 1 ",
        "Run HTab= 1 ",
        "Run UNet= 1 ",
        "Run PostProcess= 1 ",
        "UNET Theta= 1 ",
        "UNET ZTol= 0.01 ",
        "UNET MxIter= 20 ",
    ]
    path = project_dir / plan_name
    _write_ras_text(path, lines)
    return path


def update_prj_current_plan(project_dir: Path, *, plan_key: str, flow_key: str, titles: tuple[str, str]) -> None:
    prj_path = project_dir / "simple_channel.prj"
    plan_title, flow_title = titles
    text = prj_path.read_text(encoding="utf-8") if prj_path.is_file() else ""
    replacements = {
        "Current Plan=": f"Current Plan=p{plan_key}",
        "Unsteady File=": f"Unsteady File={flow_key}",
        "Unsteady Title=": f"Unsteady Title={flow_title}",
        "Plan File=": f"Plan File=p{plan_key}",
        "Plan Title=": f"Plan Title={plan_title}",
    }
    for key, value in replacements.items():
        if key in text:
            text = "\n".join(
                value if line.startswith(key) else line for line in text.splitlines()
            )
        else:
            text = text.rstrip() + f"\n{value}\n"
    prj_path.write_text(text.rstrip() + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--project-dir",
        type=Path,
        default=PROJECT,
        help="Bundled simple_channel project directory",
    )
    parser.add_argument(
        "--set-prj-plan",
        choices=("04", "05", "none"),
        default="none",
        help="Optionally set simple_channel.prj Current Plan to p04 or p05",
    )
    args = parser.parse_args()

    project_dir = args.project_dir.resolve()
    project_dir.mkdir(parents=True, exist_ok=True)

    hydrograph = ramp_hydrograph_cfs()
    u04 = write_u04_friction(project_dir, hydrograph)
    u05 = write_u05_rating(project_dir, hydrograph)
    p04 = write_plan(
        project_dir,
        plan_name="simple_channel.p04",
        title="Simple trapezoidal channel unsteady (ramp Q, friction DS)",
        short_id="SimpleCh04",
        flow_file="u04",
    )
    p05 = write_plan(
        project_dir,
        plan_name="simple_channel.p05",
        title="Simple trapezoidal channel unsteady (ramp Q, rating DS)",
        short_id="SimpleCh05",
        flow_file="u05",
    )

    if args.set_prj_plan == "04":
        update_prj_current_plan(
            project_dir,
            plan_key="04",
            flow_key="u04",
            titles=(
                "Simple trapezoidal channel unsteady (ramp Q, friction DS)",
                "Simple channel ramp Q unsteady (friction DS)",
            ),
        )
    elif args.set_prj_plan == "05":
        update_prj_current_plan(
            project_dir,
            plan_key="05",
            flow_key="u05",
            titles=(
                "Simple trapezoidal channel unsteady (ramp Q, rating DS)",
                "Simple channel ramp Q unsteady (rating DS)",
            ),
        )

    print(f"Wrote {u04.name}  Q: {hydrograph[0]:.1f} → {hydrograph[12]:.1f} → {hydrograph[24]:.1f} → {hydrograph[36]:.1f} → {hydrograph[-1]:.1f} cfs")
    print(f"Wrote {u05.name}")
    print(f"Wrote {p04.name}")
    print(f"Wrote {p05.name}")
    if args.set_prj_plan != "none":
        print(f"Updated simple_channel.prj → Plan p{args.set_prj_plan}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
