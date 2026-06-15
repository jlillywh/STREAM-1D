#!/usr/bin/env python3
"""Extract p08 HDF WSEL timeseries reference for ConSpan ramp linked verify."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))

from lib.hecras_plan_parser import parse_plan  # noqa: E402
from lib.ras_headless import (  # noqa: E402
    extract_wsel_timeseries_at_rms,
    timeseries_checkpoints_to_reference_doc,
    write_reference_json,
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--hdf", type=Path, required=True)
    parser.add_argument(
        "--scenario",
        type=Path,
        default=ORACLE / "scenarios" / "conspan_unsteady_ramp_linked.json",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=ORACLE / "projects" / "conspan" / "reference_wsel_timeseries_ramp.json",
    )
    parser.add_argument(
        "--checkpoints-rm",
        type=str,
        default=None,
        help="Override scenario compare.checkpoints_rm (comma-separated)",
    )
    parser.add_argument(
        "--time-checkpoints-hr",
        type=str,
        default=None,
        help="Override scenario compare.time_checkpoints_hr (comma-separated)",
    )
    args = parser.parse_args()

    scenario = json.loads(args.scenario.read_text(encoding="utf-8"))
    compare = scenario.get("compare", {})

    def _parse_list(raw: str | None, key: str) -> list[float]:
        if raw:
            return [float(p.strip()) for p in raw.split(",") if p.strip()]
        return [float(v) for v in compare.get(key, [])]

    checkpoints_rm = _parse_list(args.checkpoints_rm, "checkpoints_rm")
    time_checkpoints_hr = _parse_list(args.time_checkpoints_hr, "time_checkpoints_hr")
    if not checkpoints_rm or not time_checkpoints_hr:
        raise SystemExit("scenario missing compare.checkpoints_rm or time_checkpoints_hr")

    linked = scenario["linked_project"]
    project_dir = ORACLE / linked["directory"]
    plan_path = project_dir / linked["plan"]
    plan = parse_plan(plan_path) if plan_path.is_file() else None
    dt_seconds = plan.computation_interval_seconds if plan else 3600.0

    hdf_path = args.hdf.resolve()
    if not hdf_path.is_file():
        raise SystemExit(f"HDF not found: {hdf_path}")

    checkpoints = extract_wsel_timeseries_at_rms(
        hdf_path,
        checkpoints_rm,
        time_checkpoints_hr,
        rm_tol=0.02,
    )
    doc = timeseries_checkpoints_to_reference_doc(
        checkpoints,
        source=f"HEC-RAS p08 HDF ({hdf_path.name})",
        time_checkpoints_hr=time_checkpoints_hr,
        hdf_path=hdf_path,
        coupling_mode=0,
    )
    doc["dt_seconds"] = dt_seconds
    write_reference_json(args.output, doc)
    print(f"Wrote {args.output} ({len(checkpoints)} samples, dt={dt_seconds}s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())