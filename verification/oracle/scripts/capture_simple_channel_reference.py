#!/usr/bin/env python3
"""Capture terminal WSEL reference for simple trapezoidal channel oracle.

Usage:
  # After running HEC-RAS plan 01, paste Observed HWM into u02 then:
  python3 verification/oracle/scripts/capture_simple_channel_reference.py --from-u02

  # Bootstrap STREAM-1D-only reference (self-check, not HEC-RAS parity):
  python3 verification/oracle/scripts/capture_simple_channel_reference.py --from-stream1d

  # Update u02 Observed HWM lines from reference JSON:
  python3 verification/oracle/scripts/capture_simple_channel_reference.py --write-u02
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

import stream1d as st  # noqa: E402

from lib.hecras_geom_parser import parse_g01  # noqa: E402
from lib.hecras_unsteady_parser import parse_unsteady_flow  # noqa: E402
from lib.simple_channel_mapper import (  # noqa: E402
    build_simple_channel_unsteady_inputs,
    rm_to_payload_index,
)

CHECKPOINTS_RM = [3.0, 2.0, 1.0, 0.0]
PROJECT = ORACLE / "projects" / "simple_channel"
REF_PATH = PROJECT / "reference_wsel_simple_channel_unsteady.json"
U02_PATH = PROJECT / "simple_channel.u02"


def capture_from_stream1d() -> dict:
    payload, _ = build_simple_channel_unsteady_inputs(PROJECT)
    result = st.solve_unsteady(payload)
    last = result["wsel"][-1]
    geom = parse_g01(PROJECT / "simple_channel.g01")
    checkpoints = []
    for rm in CHECKPOINTS_RM:
        idx = rm_to_payload_index(rm, geom.cross_sections)
        if idx is None:
            raise RuntimeError(f"no payload index for RM {rm}")
        checkpoints.append({"rm": rm, "max_wsel_ft": last[idx]})
    return {
        "source": "STREAM-1D terminal WSEL (constant Q=150 cfs, DS stage 101 ft) — replace with HEC-RAS after plan 01 run",
        "coupling_mode": 0,
        "checkpoints": checkpoints,
    }


def capture_from_u02() -> dict:
    flow = parse_unsteady_flow(U02_PATH)
    if not flow.observed_hwm:
        raise SystemExit(
            "No Observed HWM elevations in u02 — run HEC-RAS plan 01 and paste terminal WSEL "
            "into Observed HWM= lines, or use --from-stream1d for bootstrap."
        )
    checkpoints = []
    for rm in CHECKPOINTS_RM:
        wsel = flow.observed_hwm.get(rm)
        if wsel is None:
            closest = min(flow.observed_hwm.keys(), key=lambda k: abs(k - rm))
            if abs(closest - rm) > 0.02:
                raise SystemExit(f"missing Observed HWM near RM {rm}")
            wsel = flow.observed_hwm[closest]
        checkpoints.append({"rm": rm, "max_wsel_ft": wsel})
    return {
        "source": "HEC-RAS Observed HWM from simple_channel.u02 (terminal WSEL after plan 01)",
        "coupling_mode": 0,
        "checkpoints": checkpoints,
    }


def write_u02_from_reference(doc: dict) -> None:
    text = U02_PATH.read_text(encoding="utf-8")
    rm_to_wsel = {float(c["rm"]): float(c["max_wsel_ft"]) for c in doc["checkpoints"]}
    lines = []
    for line in text.splitlines():
        if line.startswith("Observed HWM="):
            parts = [p.strip() for p in line.split("=", 1)[1].split(",")]
            rm = float(re.sub(r"[^0-9.\-]", "", parts[2]))
            wsel = rm_to_wsel.get(rm)
            if wsel is not None:
                prefix = line.split("=", 1)[0]
                river = parts[0]
                reach = parts[1]
                line = f"{prefix}={river}    ,{reach} ,{rm:.1f}    ,,{wsel:.4f}"
        lines.append(line)
    U02_PATH.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"updated Observed HWM in {U02_PATH}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--from-stream1d", action="store_true")
    group.add_argument("--from-u02", action="store_true")
    group.add_argument("--write-u02", action="store_true")
    args = parser.parse_args()

    if args.write_u02:
        if not REF_PATH.is_file():
            raise SystemExit(f"reference not found: {REF_PATH}")
        doc = json.loads(REF_PATH.read_text(encoding="utf-8"))
        write_u02_from_reference(doc)
        return 0

    doc = capture_from_stream1d() if args.from_stream1d else capture_from_u02()
    REF_PATH.parent.mkdir(parents=True, exist_ok=True)
    REF_PATH.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {REF_PATH}")
    for c in doc["checkpoints"]:
        print(f"  RM {c['rm']:.1f}: {c['max_wsel_ft']:.4f} ft")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
