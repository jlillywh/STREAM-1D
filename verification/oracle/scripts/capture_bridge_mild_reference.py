#!/usr/bin/env python3
"""Capture mode-0 terminal WSEL reference for bridge mild oracle scenarios."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

import stream1d as st
from lib.bridge_mild_mapper import (
    build_bridge_mild_unsteady_inputs,
    station_to_payload_index,
)

CHECKPOINTS_M = [52.0, 48.0, 25.0, 0.0]
M_TO_FT = 3.280839895


def capture(case: str, out_path: Path) -> None:
    payload, _ = build_bridge_mild_unsteady_inputs(
        ORACLE / "projects" / "bridge_mild",
        case=case,
        coupling_mode=0,
    )
    result = st.solve_unsteady(payload)
    last = result["wsel"][-1]
    checkpoints = []
    for sta in CHECKPOINTS_M:
        idx = station_to_payload_index(sta, payload)
        if idx is None:
            raise RuntimeError(f"no payload index for station {sta}")
        checkpoints.append({"rm": sta, "max_wsel_ft": last[idx] * M_TO_FT})
    doc = {
        "source": f"STREAM-1D mode 0 terminal WSEL ({case} bridge mild pulse)",
        "case": case,
        "coupling_mode": 0,
        "checkpoints": checkpoints,
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {out_path}")


def main() -> int:
    project = ORACLE / "projects" / "bridge_mild"
    capture("yarnell", project / "reference_wsel_bridge_mild_unsteady.json")
    capture("wspro", project / "reference_wsel_bridge_mild_wspro_unsteady.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
