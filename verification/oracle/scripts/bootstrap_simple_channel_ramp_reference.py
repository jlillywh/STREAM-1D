#!/usr/bin/env python3
"""
Bootstrap ramp reference JSON from STREAM-1D only (pre-HEC-RAS capture).

Writes development references so mapper/solver smoke can run before Plan 04/05 HDF exists.
Replace with chunk1_simple_channel_ramp_capture.py after HEC-RAS compute.

Usage:
  PYTHONPATH=python python3 verification/oracle/scripts/bootstrap_simple_channel_ramp_reference.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ROOT / "python"))

import stream1d as st  # noqa: E402

from lib.ras_headless import write_reference_json  # noqa: E402
from lib.scenario import load_scenario  # noqa: E402
from lib.simple_channel_mapper import build_simple_channel_unsteady_inputs  # noqa: E402

CASES = (
    ("simple_channel_ramp_unsteady_linked.json", "simple_channel.u04"),
    ("simple_channel_ramp_rating_unsteady_linked.json", "simple_channel.u05"),
)


def _bootstrap_scenario(scenario_path: Path, flow_name: str) -> Path:
    scenario = load_scenario(scenario_path)
    project_dir = scenario.linked_project_dir()
    compare = scenario.raw["compare"]
    checkpoints_rm = [float(r) for r in compare["checkpoints_rm"]]
    time_checkpoints_hr = [float(h) for h in compare["time_checkpoints_hr"]]

    payload, flow = build_simple_channel_unsteady_inputs(
        project_dir,
        geometry_name=scenario.raw["linked_project"]["geometry"],
        flow_name=flow_name,
    )
    result = st.solve_unsteady(payload)
    wsel = result["wsel"]

    geom = __import__("lib.hecras_geom_parser", fromlist=["parse_g01"]).parse_g01(
        project_dir / scenario.raw["linked_project"]["geometry"]
    )
    rm_to_idx = {xs.rm: idx for idx, xs in enumerate(geom.cross_sections)}

    checkpoints = []
    for rm in checkpoints_rm:
        idx = rm_to_idx.get(rm)
        if idx is None:
            continue
        by_hour: dict[str, float] = {}
        for hour in time_checkpoints_hr:
            step = int(round(hour))
            by_hour[str(int(hour) if float(hour).is_integer() else hour)] = float(
                wsel[step][idx]
            )
        checkpoints.append(
            {
                "rm": rm,
                "max_wsel_ft": max(by_hour.values()),
                "wsel_ft_by_hour": dict(sorted(by_hour.items(), key=lambda kv: float(kv[0]))),
            }
        )

    ref_rel = scenario.raw["reference"]["file"]
    ref_path = (ORACLE / ref_rel).resolve()
    doc = {
        "source": f"STREAM-1D bootstrap (replace with HEC-RAS HDF) — {scenario.id}",
        "coupling_mode": 0,
        "time_checkpoints_hr": time_checkpoints_hr,
        "checkpoints": sorted(checkpoints, key=lambda row: -row["rm"]),
        "bootstrap": True,
        "notes": "Development placeholder until Plan 04/05 HDF capture.",
    }
    write_reference_json(ref_path, doc)
    print(f"Wrote bootstrap reference: {ref_path}")
    return ref_path


def main() -> int:
    for scenario_name, flow_name in CASES:
        _bootstrap_scenario(ORACLE / "scenarios" / scenario_name, flow_name)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
