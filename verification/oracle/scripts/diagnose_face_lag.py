#!/usr/bin/env python3
"""
Chunk 7 gate: probe one-step face lag at BU/BD (mode 0 vs mode 2).

Run after Chunk 5–6 acceptance. If max |ΔWSEL| on stiff pulse stays below
FACE_LAG_MODE2_SUFFICIENT_FT, mode 1 (reach–structure–reach outer loop) is
not required for subcritical mild/stiff synthetic cases.

HDF comparison (Beaver / ConSpan) remains the authority to re-open this gate.
"""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

import stream1d as st
from lib.bridge_mild_mapper import (
    FACE_LAG_MODE2_SUFFICIENT_FT,
    build_bridge_mild_unsteady_inputs,
    station_to_payload_index,
)

M_TO_FT = 3.280839895
BU_STA = 52.0
BD_STA = 48.0


def run(pulse_profile: str, coupling_mode: int) -> dict:
    payload, _ = build_bridge_mild_unsteady_inputs(
        ORACLE / "projects" / "bridge_mild",
        case="yarnell",
        coupling_mode=coupling_mode,
        pulse_profile=pulse_profile,
    )
    result = st.solve_unsteady(payload)
    bu_idx = station_to_payload_index(BU_STA, payload)
    bd_idx = station_to_payload_index(BD_STA, payload)
    if bu_idx is None or bd_idx is None:
        raise RuntimeError("BU/BD stations not found in payload")
    return {
        "payload": payload,
        "wsel": result["wsel"],
        "bu_idx": bu_idx,
        "bd_idx": bd_idx,
        "bridge_hw": result.get("bridge_wsel_upstream"),
        "implicit": result.get("structure_implicit_interval_count") or [],
        "converged": result.get("structure_coupling_converged") or [],
    }


def max_face_delta_ft(run_a: dict, run_b: dict) -> tuple[float, int, str]:
    worst = 0.0
    worst_step = 0
    worst_face = "BU"
    n = min(len(run_a["wsel"]), len(run_b["wsel"]))
    for step in range(n):
        for face, idx, label in (
            (run_a["bu_idx"], run_b["bu_idx"], "BU"),
            (run_a["bd_idx"], run_b["bd_idx"], "BD"),
        ):
            delta_ft = abs(run_a["wsel"][step][idx] - run_b["wsel"][step][idx]) * M_TO_FT
            if delta_ft > worst:
                worst = delta_ft
                worst_step = step
                worst_face = label
    return worst, worst_step, worst_face


def mode0_structure_hw_lag_ft(run0: dict) -> tuple[float, int]:
    """Mode 0 only: |face WSEL − bridge_wsel_upstream| at BU (post-step lag proxy)."""
    hw = run0.get("bridge_hw")
    if not hw:
        return 0.0, 0
    bu_idx = run0["bu_idx"]
    worst = 0.0
    worst_step = 0
    for step, wsel_step in enumerate(run0["wsel"]):
        if step >= len(hw) or not hw[step]:
            continue
        diag_hw = hw[step][0]
        face = wsel_step[bu_idx]
        delta_ft = abs(face - diag_hw) * M_TO_FT
        if delta_ft > worst:
            worst = delta_ft
            worst_step = step
    return worst, worst_step


def main() -> int:
    print("=== stiff pulse: mode 0 vs mode 2 face lag ===")
    mode0_stiff = run("stiff", 0)
    mode2_stiff = run("stiff", 2)
    worst, step, face = max_face_delta_ft(mode0_stiff, mode2_stiff)
    print(f"max |WSEL_mode0 − WSEL_mode2| = {worst:.4f} ft at step {step} ({face})")
    print(f"gate: mode 2 sufficient if ≤ {FACE_LAG_MODE2_SUFFICIENT_FT:.3f} ft")

    lag_hw, lag_step = mode0_structure_hw_lag_ft(mode0_stiff)
    print(f"mode 0 |face − bridge_hw_diag| max = {lag_hw:.4f} ft at step {lag_step}")

    print()
    print("=== mild pulse: mode 0 vs mode 2 (Chunk 6 baseline) ===")
    mode0_mild = run("mild", 0)
    mode2_mild = run("mild", 2)
    mild_worst, mild_step, mild_face = max_face_delta_ft(mode0_mild, mode2_mild)
    print(f"max |Δ| = {mild_worst:.4f} ft at step {mild_step} ({mild_face})")

    implicit_steps = sum(1 for c in mode2_stiff["implicit"] if c > 0)
    print()
    print(f"stiff mode 2 implicit steps: {implicit_steps}/{len(mode2_stiff['implicit'])}")
    print(f"stiff mode 2 all converged: {all(mode2_stiff['converged'])}")

    mode2_sufficient = worst <= FACE_LAG_MODE2_SUFFICIENT_FT
    if mode2_sufficient:
        print()
        print(
            "DECISION: mode 2 sufficient for synthetic subcritical bridge pulse — "
            "defer mode 1 until HDF shows systematic BU/BD lag vs RAS."
        )
        return 0

    print()
    print(
        "FAIL: stiff pulse shows mode 0 vs mode 2 face divergence above gate — "
        "consider implementing mode 1 (reach–structure–reach outer loop)."
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
