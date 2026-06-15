#!/usr/bin/env python3
"""
Write ConSpan mild Q-ramp unsteady flow (plan 04 / u04).

Trapezoid: 12h @ 600 cfs → 24h ramp → 12h @ 1000 cfs (48 × 1HOUR).
Downstream stage constant 30.51 ft. Matches diagnose_conspan_transition.py.

Examples:
  python3 verification/oracle/scripts/write_conspan_ramp_u04.py

  python3 verification/oracle/scripts/write_conspan_ramp_u04.py \\
    --output-dir /mnt/c/Users/jason/Documents/hecras_testing/ConSpan \\
    --flow-name ConSpan.u04 --write-plan
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))

from lib.hecras_geom_parser import parse_g01  # noqa: E402
from lib.write_hecras_unsteady_flow import (  # noqa: E402
    mild_ramp_hydrograph,
    write_q_stage_ras701_from_template,
    write_q_stage_u02,
)

DEFAULT_G01 = ORACLE / "projects" / "conspan" / "ConSpan.g01"
DEFAULT_OUT = ORACLE / "projects" / "conspan"
DEFAULT_TEMPLATE = ORACLE / "projects" / "conspan" / "ConSpan.u01.ras701.template"

DS_STAGE_FT = 30.51
Q_LOW = 600.0
Q_HIGH = 1000.0
NUM_INTERVALS = 48
INTERVAL = "1HOUR"


def _river_reach(g01_path: Path) -> tuple[str, str, float, float]:
    geom = parse_g01(g01_path)
    xs_sorted = sorted(geom.cross_sections, key=lambda xs: xs.rm, reverse=True)
    upstream = xs_sorted[0]
    downstream = xs_sorted[-1]
    return upstream.river.strip(), upstream.reach.strip(), upstream.rm, downstream.rm


def _write_plan_stub(path: Path, *, flow_slot: str = "u04") -> None:
    text = f"""Plan Title=ConSpan mild Q ramp unsteady
Program Version=5.00
Short Identifier=ConSpanRmp
Simulation Date=01JAN2000,0000,03JAN2000,0000
Geom File=g01
Flow File={flow_slot}
Subcritical Flow
Friction Slope Method= 1 
Unsteady Friction Slope Method= 2 
Computation Interval=15MIN
Output Interval=1HOUR
Run HTab= 1 
Run UNet= 1 
Run PostProcess= 1 
UNET Theta= 1 
UNET ZTol= 0.01 
UNET MxIter= 20 
"""
    path.write_text(text, encoding="utf-8", newline="\n")


def main() -> int:
    parser = argparse.ArgumentParser(description="Write ConSpan Q-ramp unsteady flow u04")
    parser.add_argument("--g01", type=Path, default=DEFAULT_G01)
    parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--flow-name", default="conspan.u04")
    parser.add_argument(
        "--template",
        type=Path,
        default=None,
        help="RAS 7.01 flow file to clone (default: ConSpan.u01.ras701.template, else bundled)",
    )
    parser.add_argument(
        "--legacy-format",
        action="store_true",
        help="Write legacy 5.00 format (may crash HEC-RAS 7.x GUI)",
    )
    parser.add_argument("--write-plan", action="store_true", help="Also write conspan.p04")
    args = parser.parse_args()

    g01_path = args.g01.resolve()
    if not g01_path.is_file():
        raise SystemExit(f"Geometry not found: {g01_path}")

    river, reach, upstream_rm, downstream_rm = _river_reach(g01_path)
    out_dir = args.output_dir.resolve()
    flow_path = out_dir / args.flow_name

    q_values = mild_ramp_hydrograph(
        num_intervals=NUM_INTERVALS,
        q_low=Q_LOW,
        q_high=Q_HIGH,
        hold_low_hours=12,
        ramp_hours=24,
    )
    stage_values = [DS_STAGE_FT] * NUM_INTERVALS

    if args.legacy_format:
        write_q_stage_u02(
            flow_path,
            title="ConSpan mild Q ramp unsteady",
            river=river,
            reach=reach,
            upstream_rm=upstream_rm,
            downstream_rm=downstream_rm,
            q_values=q_values,
            stage_values=stage_values,
            initial_flow_cfs=Q_LOW,
            interval=INTERVAL,
        )
    else:
        template = args.template
        if template is None:
            win_u01 = Path("C:/Users/jason/Documents/hecras_testing/ConSpan/ConSpan.u01")
            template = win_u01 if win_u01.is_file() else DEFAULT_TEMPLATE
        template = template.resolve()
        if not template.is_file():
            raise SystemExit(
                f"RAS 7.01 template not found: {template}\n"
                "Pass --template path/to/ConSpan.u01 or use --legacy-format."
            )
        write_q_stage_ras701_from_template(
            flow_path,
            template,
            title="ConSpan mild Q ramp unsteady",
            q_values=q_values,
            stage_values=stage_values,
        )

    print(f"Wrote {flow_path}")
    print(f"  upstream RM {upstream_rm:g}: 12h @ {Q_LOW:g} -> 24h ramp -> 12h @ {Q_HIGH:g} cfs")
    print(f"  downstream RM {downstream_rm:g}: stage {DS_STAGE_FT:g} ft constant")
    print(f"  intervals: {NUM_INTERVALS} x {INTERVAL} (matches 01Jan-03Jan2000 sim window)")

    if args.write_plan:
        plan_path = out_dir / "conspan.p04"
        stem = Path(args.flow_name).stem.lower()
        flow_slot = stem if stem.startswith("u") and len(stem) == 3 else "u04"
        _write_plan_stub(plan_path, flow_slot=flow_slot)
        print(f"Wrote {plan_path} (Flow File=u04; Geom File=g01)")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
