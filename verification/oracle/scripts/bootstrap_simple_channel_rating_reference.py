#!/usr/bin/env python3
"""Bootstrap rating-curve scenario reference from STREAM-1D (pre-HDF)."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ROOT / "python"))

import stream1d as st  # noqa: E402

from lib.simple_channel_mapper import (  # noqa: E402
    build_simple_channel_unsteady_inputs,
    rm_to_payload_index,
)
from lib.hecras_geom_parser import parse_g01  # noqa: E402

PROJECT = ORACLE / "projects" / "simple_channel"
SCENARIO_CHECKPOINTS = [3.0, 2.0, 1.0, 0.0]
OUT = PROJECT / "reference_wsel_simple_channel_rating_unsteady.json"


def main() -> int:
    payload, _flow = build_simple_channel_unsteady_inputs(
        PROJECT,
        flow_name="simple_channel.u03",
    )
    if payload.get("downstream_bc_type") != 3:
        print("ERROR: expected downstream_bc_type=3", file=sys.stderr)
        return 1

    result = st.solve_unsteady(payload)
    geom = parse_g01(PROJECT / "simple_channel.g01")
    checkpoints = []
    for rm in SCENARIO_CHECKPOINTS:
        idx = rm_to_payload_index(rm, geom.cross_sections)
        if idx is None:
            print(f"ERROR: no XS near RM {rm}", file=sys.stderr)
            return 1
        wsel = float(result["wsel"][-1][idx])
        checkpoints.append({"rm": rm, "max_wsel_ft": wsel})
        print(f"  RM {rm:.1f}: {wsel:.4f} ft")

    doc = {
        "source": "STREAM-1D bootstrap (rating-curve DS, pending HEC-RAS HDF)",
        "coupling_mode": payload.get("unsteady_structure_coupling_mode", 0),
        "checkpoints": checkpoints,
    }
    OUT.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
