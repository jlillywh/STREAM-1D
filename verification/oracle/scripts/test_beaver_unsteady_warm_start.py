#!/usr/bin/env python3
"""Chunk 8 gate: Beaver steady warm-start initial WSEL vs solve_steady (±0.04 ft)."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

import stream1d as st
from lib.beaver_mapper import build_beaver_unsteady_inputs, rm_to_payload_index
from lib.hecras_geom_parser import parse_g01

TOLERANCE_FT = 0.04
CHECKPOINTS = [5.99, 5.875, 5.76, 5.685, 5.61, 5.41, 5.39, 5.13, 5.065, 5.0]

project = ORACLE / "projects" / "beaver"
geom = parse_g01(project / "beaver.g01")


def steady_reference(payload: dict, flow_cfs: float) -> list[float]:
    cross_sections = [st.CrossSection(**xs) for xs in payload["cross_sections"]]
    kwargs: dict = {
        "cross_sections": cross_sections,
        "flow_rate": flow_cfs,
        "regime": 0,
        "num_slices": payload.get("num_slices", 80),
        "max_spacing": payload.get("max_spacing", 200.0),
        "downstream_bc_type": payload.get("downstream_bc_type", 0),
        "coeff_contraction": payload.get("coeff_contraction", 0.3),
        "coeff_expansion": payload.get("coeff_expansion", 0.1),
    }
    if payload.get("downstream_bc_type") == 2:
        kwargs["downstream_bc_slope"] = payload.get("downstream_bc_slope", 0.002)
    else:
        ds = payload.get("downstream_wsel_hydrograph") or []
        kwargs["downstream_wsel"] = ds[0] if ds else 0.0
    for key in (
        "bridge_stations",
        "bridge_low_chords",
        "bridge_high_chords",
        "bridge_pier_widths",
        "bridge_num_piers",
        "bridge_weir_coeffs",
        "bridge_orifice_coeffs",
        "bridge_low_flow_methods",
        "bridge_wspro_coeffs",
        "bridge_lengths",
        "bridge_friction_weighting",
        "bridge_approach_friction_lengths",
        "bridge_departure_friction_lengths",
        "bridge_high_flow_methods",
        "bridge_pressure_flow_coeffs_inlet",
        "bridge_deck_stations",
        "bridge_deck_low_elevations",
        "bridge_deck_high_elevations",
        "bridge_pier_stations",
        "bridge_pier_base_elevations",
        "bridge_pier_top_elevations",
        "bridge_ineffective_left_stations_upstream",
        "bridge_ineffective_left_elevations_upstream",
        "bridge_ineffective_right_stations_upstream",
        "bridge_ineffective_right_elevations_upstream",
        "bridge_ineffective_left_stations_downstream",
        "bridge_ineffective_left_elevations_downstream",
        "bridge_ineffective_right_stations_downstream",
        "bridge_ineffective_right_elevations_downstream",
    ):
        if payload.get(key) is not None:
            kwargs[key] = payload[key]
    if payload.get("bridge_upstream_cross_sections"):
        kwargs["bridge_upstream_cross_sections"] = [
            st.CrossSection(**payload["bridge_upstream_cross_sections"][0])
        ]
    if payload.get("bridge_downstream_cross_sections"):
        kwargs["bridge_downstream_cross_sections"] = [
            st.CrossSection(**payload["bridge_downstream_cross_sections"][0])
        ]
    if payload.get("bridge_internal_cross_sections"):
        kwargs["bridge_internal_cross_sections"] = [
            [st.CrossSection(**xs) for xs in payload["bridge_internal_cross_sections"][0]]
        ]
    return list(st.solve_steady(st.SteadyInputs(**kwargs))["wsel"])


payload, flow = build_beaver_unsteady_inputs(project)
steady_wsel = steady_reference(payload, flow.initial_flow_cfs)

worst = 0.0
worst_rm = 0.0
for rm in CHECKPOINTS:
    idx = rm_to_payload_index(rm, geom.cross_sections)
    if idx is None:
        continue
    init = payload["initial_wsel"][idx]
    expected = steady_wsel[idx]
    delta = abs(init - expected)
    if delta > worst:
        worst = delta
        worst_rm = rm

passed = worst <= TOLERANCE_FT
status = "PASS" if passed else "FAIL"
print(
    f"initial_flow={flow.initial_flow_cfs:.0f} cfs: "
    f"max |initial - steady| = {worst:.4f} ft at RM {worst_rm:.3f} [{status}] "
    f"(tol ±{TOLERANCE_FT} ft)"
)
if not passed:
    sys.exit(1)
print("warm-start OK")
