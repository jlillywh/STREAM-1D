#!/usr/bin/env python3
"""Chunk 6 gate: steady warm-start vs constant-Q unsteady (WSPRO + Yarnell, ±0.04 ft)."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

import stream1d as st
from lib.bridge_mild_mapper import build_bridge_mild_unsteady_inputs, station_to_payload_index

TOLERANCE_FT = 0.04
M_TO_FT = 3.280839895


def warm_start_case(case: str, coupling_mode: int) -> tuple[bool, float, float]:
    payload, _ = build_bridge_mild_unsteady_inputs(
        ORACLE / "projects" / "bridge_mild",
        case=case,
        coupling_mode=coupling_mode,
    )
    base_q = payload["upstream_q_hydrograph"][0]
    steady = st.SteadyInputs(
        cross_sections=[st.CrossSection(**xs) for xs in payload["cross_sections"]],
        flow_rate=base_q,
        regime=0,
        downstream_wsel=payload["downstream_wsel_hydrograph"][0],
        downstream_bc_type=0,
        num_slices=payload.get("num_slices", 50),
        max_spacing=payload.get("max_spacing", 50.0),
        bridge_stations=payload.get("bridge_stations"),
        bridge_low_chords=payload.get("bridge_low_chords"),
        bridge_high_chords=payload.get("bridge_high_chords"),
        bridge_pier_widths=payload.get("bridge_pier_widths"),
        bridge_num_piers=payload.get("bridge_num_piers"),
        bridge_low_flow_methods=payload.get("bridge_low_flow_methods"),
        bridge_wspro_coeffs=payload.get("bridge_wspro_coeffs"),
        bridge_friction_weighting=payload.get("bridge_friction_weighting"),
        bridge_approach_friction_lengths=payload.get("bridge_approach_friction_lengths"),
        bridge_departure_friction_lengths=payload.get("bridge_departure_friction_lengths"),
        bridge_upstream_cross_sections=[
            st.CrossSection(**payload["bridge_upstream_cross_sections"][0])
        ]
        if payload.get("bridge_upstream_cross_sections")
        else None,
        bridge_downstream_cross_sections=[
            st.CrossSection(**payload["bridge_downstream_cross_sections"][0])
        ]
        if payload.get("bridge_downstream_cross_sections")
        else None,
    )
    steady_wsel = st.solve_steady(steady)["wsel"]

    run = st.solve_unsteady(payload)
    last = run["wsel"][-1]
    worst_ft = 0.0
    worst_sta = 0.0
    for sta in (52.0, 48.0, 25.0, 0.0):
        idx = station_to_payload_index(sta, payload)
        if idx is None:
            continue
        delta_ft = abs(last[idx] - steady_wsel[idx]) * M_TO_FT
        if delta_ft > worst_ft:
            worst_ft = delta_ft
            worst_sta = sta
    passed = worst_ft <= TOLERANCE_FT
    return passed, worst_ft, worst_sta


def main() -> int:
    ok = True
    for case in ("yarnell", "wspro"):
        for mode in (0, 2):
            passed, worst, sta = warm_start_case(case, mode)
            status = "PASS" if passed else "FAIL"
            print(
                f"{case} mode {mode}: max |terminal - steady| = {worst:.4f} ft "
                f"at station {sta:.1f} m [{status}] (tol ±{TOLERANCE_FT} ft)"
            )
            ok = ok and passed
    if not ok:
        sys.exit(1)
    print("warm-start OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
