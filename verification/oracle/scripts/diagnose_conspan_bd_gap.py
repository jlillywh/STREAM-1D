#!/usr/bin/env python3
"""Decompose ConSpan BD WSEL gap vs HEC-RAS at Q=1000 cfs."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "python"))

import stream1d as st  # noqa: E402

FIX = ROOT / "verification" / "fixtures" / "conspan_project_12.json"
REF = (
    Path(__file__).resolve().parents[1]
    / "projects/conspan/reference_wsel_timeseries_ramp.json"
)
Q = 1000.0
DS = 30.51
RM_BU, RM_BD = 20.238, 20.227
RAS_BD = 31.6492


def _load() -> tuple[list[st.CrossSection], dict, dict[float, int]]:
    data = json.loads(FIX.read_text())
    rows = sorted(data["geometry_data"], key=lambda r: float(r["station"]), reverse=True)
    css = [
        st.CrossSection(
            station=float(r["station"]),
            x=[float(v) for v in r["x"]],
            y=[float(v) for v in r["y"]],
            n_stations=[float(v) for v in r["n_stations"]],
            n_values=[float(v) for v in r["n_values"]],
            unit_system=r.get("unit_system", "USCustomary"),
            is_overbank=r.get("is_overbank"),
        )
        for r in rows
    ]
    c = data["culvert_stations"][0]
    culvert = {
        "culvert_stations": [float(c["station"])],
        "culvert_shape_types": [int(c["shape_type"])],
        "culvert_spans": [float(c["span"])],
        "culvert_rises": [float(c["rise"])],
        "culvert_roughness_ns": [float(c["roughness_n"])],
        "culvert_lengths": [float(c["length"])],
        "culvert_entrance_loss_coeffs": [float(c["entrance_loss_coeff"])],
        "culvert_exit_loss_coeffs": [float(c["exit_loss_coeff"])],
        "culvert_barrels": [int(c.get("num_barrels", 1))],
        "culvert_roughness_n_bottoms": [float(c.get("roughness_n_bottom", c["roughness_n"]))],
        "culvert_depth_bottom_ns": [float(c.get("depth_bottom_n", 0.0))],
        "culvert_depth_blockeds": [float(c.get("depth_blocked", 0.0))],
        "culvert_inlet_types": [int(c.get("inlet_type", 21))],
    }
    rm_idx = {float(r["rm"]): i for i, r in enumerate(rows)}
    return css, culvert, rm_idx


def _steady(css, culvert, **extra):
    params = json.loads(FIX.read_text())["parameters"]
    return st.solve_steady(
        st.SteadyInputs(
            cross_sections=css,
            flow_rate=Q,
            downstream_wsel=DS,
            downstream_bc_type=0,
            num_slices=int(params.get("vertical_slices", 100)),
            max_spacing=float(params.get("max_spacing", 100.0)),
            regime=0,
            **culvert,
            **extra,
        )
    )


def main() -> int:
    css, culvert, rm_idx = _load()
    i_bu, i_bd = rm_idx[RM_BU], rm_idx[RM_BD]

    base = _steady(css, culvert)
    stream_bd = base["wsel"][i_bd]
    print("=== ConSpan BD gap decomposition @ Q=1000 cfs, DS stage 30.51 ft ===")
    print(f"HEC-RAS p08 terminal BD (RM {RM_BD}): {RAS_BD:.4f} ft")
    print(f"STREAM steady BD:                      {stream_bd:.4f} ft  (Δ {stream_bd - RAS_BD:+.4f})")
    print(f"Control: {base['culvert_control_types'][0]}")
    print(f"HW inlet/outlet: {base['culvert_wsel_inlet'][0]:.4f} / {base['culvert_wsel_outlet'][0]:.4f} ft")
    print()

    cases = [
        ("bed inverts (default z from XS min)", {}),
        ("HEC culvert inverts z_up=25.1 z_dn=25.0", {"culvert_z_ups": [25.1], "culvert_z_downs": [25.0]}),
        ("z_dn=25.1 (BU channel invert both sides)", {"culvert_z_ups": [25.1], "culvert_z_downs": [25.1]}),
        ("Ke=0.5 Kx=1.0 (default)", {}),
        ("Kx=0.0 (no exit loss)", {"culvert_exit_loss_coeffs": [0.0]}),
        ("Ke=0.0", {"culvert_entrance_loss_coeffs": [0.0]}),
        ("inlet_type=20 (projecting)", {"culvert_inlet_types": [20]}),
        ("bottom n strip 0.5 ft (HEC Culvert Bottom n)", {}),
    ]

    print("Sensitivity (steady BD WSEL):")
    print(f"{'case':<42} {'BD WSEL':>8} {'Δ vs RAS':>10}")
    print("-" * 62)
    for label, extra in cases:
        r = _steady(css, culvert, **extra)
        bd = r["wsel"][i_bd]
        print(f"{label:<42} {bd:8.4f} {bd - RAS_BD:+10.4f}")

    if REF.is_file():
        ref = json.loads(REF.read_text())
        ras_by_hour = ref["checkpoints"][1]["wsel_ft_by_hour"]
        print()
        print("RAS ramp trajectory BD:", ", ".join(f"h{k}={float(v):.3f}" for k, v in ras_by_hour.items()))

    print()
    print("Geometry notes:")
    bu_row = next(r for r in json.loads(FIX.read_text())["geometry_data"] if float(r["rm"]) == RM_BU)
    bd_row = next(r for r in json.loads(FIX.read_text())["geometry_data"] if float(r["rm"]) == RM_BD)
    print(f"  BU RM {RM_BU} channel invert (min y): {min(bu_row['y']):.2f} ft")
    print(f"  BD RM {RM_BD} channel invert (min y): {min(bd_row['y']):.2f} ft")
    print(f"  HEC-RAS Culvert= line US/DS invert: 25.1 / 25.0 ft")
    print(f"  Culvert station (fixture): {culvert['culvert_stations'][0]:.2f}")
    print(f"  Shape ConspanArch (type 3), span×rise 28×6 ft, L=50 ft")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
