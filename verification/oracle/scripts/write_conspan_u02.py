#!/usr/bin/env python3
"""
Write ConSpan mild unsteady flow file (Chunk 4) for HEC-RAS.

Mirrors the reach_mild constant-Q / known-stage pattern on the full ConSpan reach
(with inline arch culvert). Use after opening the steady ConSpan project in HEC-RAS
and creating an unsteady plan (plan 02) that points Flow File=u02.

Examples:
  # Oracle bundle (default)
  python3 verification/oracle/scripts/write_conspan_u02.py

  # Windows GUI staging folder (override with STREAM1D_HECRAS_STAGE)
  python3 verification/oracle/scripts/write_conspan_u02.py \\
    --output-dir "$HOME/Documents/hecras_testing/ConSpan"

  # Also emit plan 02 stub next to the flow file
  python3 verification/oracle/scripts/write_conspan_u02.py --write-plan
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))

from lib.conspan_reference import peak_wsel_by_rm  # noqa: E402
from lib.hecras_geom_parser import parse_g01  # noqa: E402
from lib.write_hecras_unsteady_flow import write_constant_q_known_stage_u02  # noqa: E402

DEFAULT_G01 = ORACLE / "projects" / "conspan" / "ConSpan.g01"
DEFAULT_OUT = ORACLE / "projects" / "conspan"

# Chunk 4 mild case — same forcing as 50 yr steady profile terminal state.
FLOW_CFS = 1000.0
DS_STAGE_FT = 30.51
NUM_INTERVALS = 48  # 48 hr @ 1HOUR → matches p02 Simulation Date span
INTERVAL = "1HOUR"
CHECKPOINT_RMS = (20.535, 20.238, 20.227, 20.208, 20.095)


def _river_reach(g01_path: Path) -> tuple[str, str, float, float]:
    geom = parse_g01(g01_path)
    if not geom.cross_sections:
        raise SystemExit(f"No cross sections in {g01_path}")
    xs_sorted = sorted(geom.cross_sections, key=lambda xs: xs.rm, reverse=True)
    upstream = xs_sorted[0]
    downstream = xs_sorted[-1]
    return upstream.river.strip(), upstream.reach.strip(), upstream.rm, downstream.rm


def _observed_hwm_placeholders() -> dict[float, float]:
    """Steady 50 yr WSEL at checkpoint RMs — replace after RAS unsteady HDF capture."""
    peaks = peak_wsel_by_rm("50 yr")
    return {rm: peaks[rm] for rm in CHECKPOINT_RMS if rm in peaks}


def _write_plan_stub(path: Path, *, flow_name: str = "u02") -> Path:
    text = f"""Plan Title=ConSpan mild unsteady constant Q
Program Version=5.00
Short Identifier=ConSpanU01
Simulation Date=01JAN2000,0000,03JAN2000,0000
Geom File=g01
Flow File={flow_name}
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
    return path


def main() -> int:
    parser = argparse.ArgumentParser(description="Write ConSpan Chunk 4 unsteady flow file")
    parser.add_argument(
        "--g01",
        type=Path,
        default=DEFAULT_G01,
        help="Geometry file to read river/reach/upstream-downstream RMs",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=DEFAULT_OUT,
        help="Directory for conspan.u02 (and optional conspan.p02)",
    )
    parser.add_argument("--flow-name", default="conspan.u02", help="Output unsteady flow filename")
    parser.add_argument("--write-plan", action="store_true", help="Also write conspan.p02 plan stub")
    parser.add_argument(
        "--no-observed-hwm",
        action="store_true",
        help="Omit Observed HWM lines (RAS will still run)",
    )
    args = parser.parse_args()

    g01_path = args.g01.resolve()
    if not g01_path.is_file():
        raise SystemExit(f"Geometry not found: {g01_path}")

    river, reach, upstream_rm, downstream_rm = _river_reach(g01_path)
    out_dir = args.output_dir.resolve()
    flow_path = out_dir / args.flow_name

    observed = None if args.no_observed_hwm else _observed_hwm_placeholders()
    write_constant_q_known_stage_u02(
        flow_path,
        title="ConSpan mild constant Q unsteady",
        river=river,
        reach=reach,
        upstream_rm=upstream_rm,
        downstream_rm=downstream_rm,
        flow_cfs=FLOW_CFS,
        downstream_stage_ft=DS_STAGE_FT,
        num_intervals=NUM_INTERVALS,
        interval=INTERVAL,
        observed_hwm=observed,
    )

    print(f"Wrote {flow_path}")
    print(f"  river/reach: {river!r} / {reach!r}")
    print(f"  upstream RM {upstream_rm:g}: Q={FLOW_CFS:g} cfs constant ({NUM_INTERVALS} x {INTERVAL})")
    print(f"  downstream RM {downstream_rm:g}: stage={DS_STAGE_FT:g} ft constant")
    if observed:
        print("  Observed HWM placeholders (50 yr steady — refresh after RAS run):")
        for rm in sorted(observed.keys(), reverse=True):
            print(f"    RM {rm:g}: {observed[rm]:g} ft")

    if args.write_plan:
        plan_path = out_dir / "conspan.p02"
        _write_plan_stub(plan_path, flow_name="u02")
        print(f"Wrote {plan_path} (Flow File=u02; Geom File=g01)")

    print("\nHEC-RAS GUI checklist:")
    print("  1. Open your ConSpan project (steady geometry already loaded).")
    print("  2. File → New Plan → Unsteady Flow (plan 02).")
    print("  3. Plan Data: Geom=g01, Flow=u02, Comp Interval=15MIN, Theta=1.0, Write HDF5=ON.")
    print("  4. Unsteady Flow Editor: verify upstream Flow Hydrograph and downstream Stage Hydrograph.")
    print("  5. Run plan 02 → extract terminal WSEL for HDF certification.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
