#!/usr/bin/env python3
"""
Investigate ConSpan culvert behavior under mild upstream Q transition.

1. Steady sweep: inlet vs outlet control vs Q at fixed DS stage 30.51 ft.
2. Unsteady ramp: 600 → 1000 cfs trapezoid; per-step control type and WSEL at BU/BD.

Usage:
  PYTHONPATH=python python3 verification/oracle/scripts/diagnose_conspan_transition.py
  PYTHONPATH=python python3 verification/oracle/scripts/diagnose_conspan_transition.py --mode 2
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ROOT / "python"))

import stream1d as st  # noqa: E402

from lib.conspan_mapper import build_conspan_unsteady_inputs  # noqa: E402
from lib.conspan_reference import rm_to_conspan_payload_index  # noqa: E402
from lib.hydrograph_ops import resample_hydrograph  # noqa: E402
from lib.write_hecras_unsteady_flow import mild_ramp_hydrograph  # noqa: E402

PROJECT = ORACLE / "projects" / "conspan"
FIXTURE = ROOT / "verification" / "fixtures" / "conspan_project_12.json"
CHECKPOINTS = (20.238, 20.227, 20.208, 20.095)
DS_STAGE = 30.51
Q_LOW = 600.0
Q_HIGH = 1000.0


def _load_fixture() -> dict:
    with FIXTURE.open(encoding="utf-8") as fh:
        return json.load(fh)


def _culvert_fields() -> dict:
    culverts = _load_fixture().get("culvert_stations", [])
    c = culverts[0]
    return {
        "culvert_stations": [float(c["station"]) for c in culverts],
        "culvert_shape_types": [int(c["shape_type"]) for c in culverts],
        "culvert_spans": [float(c["span"]) for c in culverts],
        "culvert_rises": [float(c["rise"]) for c in culverts],
        "culvert_roughness_ns": [float(c["roughness_n"]) for c in culverts],
        "culvert_lengths": [float(c["length"]) for c in culverts],
        "culvert_entrance_loss_coeffs": [float(c["entrance_loss_coeff"]) for c in culverts],
        "culvert_exit_loss_coeffs": [float(c["exit_loss_coeff"]) for c in culverts],
        "culvert_barrels": [int(c.get("num_barrels", 1)) for c in culverts],
        "culvert_roughness_n_bottoms": [float(c.get("roughness_n_bottom", c["roughness_n"])) for c in culverts],
        "culvert_depth_bottom_ns": [float(c.get("depth_bottom_n", 0.0)) for c in culverts],
        "culvert_depth_blockeds": [float(c.get("depth_blocked", 0.0)) for c in culverts],
        "culvert_inlet_types": [int(c.get("inlet_type", 21)) for c in culverts],
    }


def _cross_sections() -> list[st.CrossSection]:
    rows = sorted(_load_fixture()["geometry_data"], key=lambda xs: float(xs["station"]), reverse=True)
    out: list[st.CrossSection] = []
    for xs in rows:
        out.append(
            st.CrossSection(
                station=float(xs["station"]),
                x=[float(v) for v in xs["x"]],
                y=[float(v) for v in xs["y"]],
                n_stations=[float(v) for v in xs["n_stations"]],
                n_values=[float(v) for v in xs["n_values"]],
                unit_system=xs.get("unit_system", "USCustomary"),
                is_overbank=xs.get("is_overbank"),
            )
        )
    return out


def steady_regime_sweep() -> None:
    """Inlet/outlet control and BU/BD WSEL vs Q (steady solve)."""
    params = _load_fixture()["parameters"]
    xs = _cross_sections()
    culvert = _culvert_fields()
    print("=== Steady culvert regime sweep (DS stage = 30.51 ft) ===")
    print("Q(cfs)\tControl\tWSEL_BU\tWSEL_BD\tHW_inlet\tHW_outlet")
    print("-" * 72)
    for q in range(400, 1201, 50):
        inputs = st.SteadyInputs(
            cross_sections=xs,
            flow_rate=float(q),
            downstream_wsel=DS_STAGE,
            downstream_bc_type=0,
            num_slices=int(params.get("vertical_slices", 100)),
            max_spacing=float(params.get("max_spacing", 100.0)),
            regime=0,
            **culvert,
        )
        res = st.solve_steady(inputs)
        ctrl = (res.get("culvert_control_types") or ["?"])[0]
        i_bu = rm_to_conspan_payload_index(20.238)
        i_bd = rm_to_conspan_payload_index(20.227)
        wsel = res["wsel"]
        hw_in = (res.get("culvert_wsel_inlet") or [0.0])[0]
        hw_out = (res.get("culvert_wsel_outlet") or [0.0])[0]
        print(
            f"{q}\t{ctrl}\t{wsel[i_bu]:.3f}\t{wsel[i_bd]:.3f}\t{hw_in:.3f}\t{hw_out:.3f}"
        )
    print()


def run_ramp_unsteady(coupling_mode: int) -> None:
    """Mild Q ramp unsteady with diagnostics."""
    payload, _ = build_conspan_unsteady_inputs(PROJECT, coupling_mode=coupling_mode)
    q_hourly = mild_ramp_hydrograph(num_intervals=48, q_low=Q_LOW, q_high=Q_HIGH)
    dt_in = 3600.0
    dt_out = float(payload["dt"])
    q_steps, _ = resample_hydrograph(q_hourly, dt_in, dt_out)
    payload["upstream_q_hydrograph"] = q_steps
    payload["num_steps"] = len(q_steps)
    payload["unsteady_structure_coupling_mode"] = coupling_mode

    print(f"=== Unsteady mild Q ramp (coupling_mode={coupling_mode}) ===")
    print(f"  hourly Q: 12h@{Q_LOW:g} → 24h ramp → 12h@{Q_HIGH:g} cfs")
    print(f"  steps={len(q_steps)} dt={dt_out}s DS stage={DS_STAGE} ft")
    print()

    res = st.solve_unsteady(payload)
    wsel_ts = res["wsel"]
    ctrl_ts = res.get("culvert_control_types") or []
    implicit_n = res.get("structure_implicit_interval_count") or []
    fallback_n = res.get("structure_explicit_fallback_count") or []

    # Sample every 8 hours of sim time (32 steps @ 15MIN)
    sample_every = max(1, int(8 * 3600 / dt_out))
    idx_bu = rm_to_conspan_payload_index(20.238)
    idx_bd = rm_to_conspan_payload_index(20.227)

    print("step\thour\tQ(cfs)\tcontrol\tWSEL_BU\tWSEL_BD\timplicit\tfallback")
    print("-" * 80)
    for step in range(0, len(wsel_ts), sample_every):
        hour = step * dt_out / 3600.0
        q = q_steps[min(step, len(q_steps) - 1)]
        ctrl = ctrl_ts[step][0] if step < len(ctrl_ts) and ctrl_ts[step] else "?"
        w_bu = wsel_ts[step][idx_bu]
        w_bd = wsel_ts[step][idx_bd]
        imp = implicit_n[step] if step < len(implicit_n) else "?"
        fb = fallback_n[step] if step < len(fallback_n) else "?"
        print(f"{step}\t{hour:.1f}\t{q:.0f}\t{ctrl}\t{w_bu:.3f}\t{w_bd:.3f}\t{imp}\t{fb}")

    last = len(wsel_ts) - 1
    print("...")
    print(
        f"terminal\t{last * dt_out / 3600:.1f}\t{q_steps[-1]:.0f}\t"
        f"{ctrl_ts[last][0] if last < len(ctrl_ts) else '?'}\t"
        f"{wsel_ts[last][idx_bu]:.3f}\t{wsel_ts[last][idx_bd]:.3f}"
    )

    # Regime change count
    if ctrl_ts:
        changes = sum(
            1
            for i in range(1, len(ctrl_ts))
            if ctrl_ts[i][0] != ctrl_ts[i - 1][0]
        )
        inlet_steps = sum(1 for row in ctrl_ts if row and row[0] == "inlet")
        outlet_steps = sum(1 for row in ctrl_ts if row and row[0] == "outlet")
        print()
        print(f"  control switches: {changes}")
        print(f"  inlet steps: {inlet_steps} / {len(ctrl_ts)}")
        print(f"  outlet steps: {outlet_steps} / {len(ctrl_ts)}")
        if implicit_n:
            print(f"  max implicit_interval_count: {max(implicit_n)}")
            print(f"  max explicit_fallback_count: {max(fallback_n) if fallback_n else 0}")

    _quasi_steady_compare(q_steps, wsel_ts, sample_every, dt_out, idx_bu, idx_bd)
    print()


def _quasi_steady_compare(
    q_steps: list[float],
    wsel_ts: list[list[float]],
    sample_every: int,
    dt_out: float,
    idx_bu: int,
    idx_bd: int,
) -> None:
    """Compare unsteady WSEL to steady solve at the same Q (same DS stage)."""
    params = _load_fixture()["parameters"]
    xs = _cross_sections()
    culvert = _culvert_fields()
    print("=== Quasi-steady lag (unsteady − steady at same Q) ===")
    print("hour\tQ\tΔBU\tΔBD\tcontrol")
    print("-" * 48)
    max_bd = 0.0
    for step in range(0, len(wsel_ts), sample_every):
        q = q_steps[min(step, len(q_steps) - 1)]
        inputs = st.SteadyInputs(
            cross_sections=xs,
            flow_rate=float(q),
            downstream_wsel=DS_STAGE,
            downstream_bc_type=0,
            num_slices=int(params.get("vertical_slices", 100)),
            max_spacing=float(params.get("max_spacing", 100.0)),
            regime=0,
            **culvert,
        )
        steady = st.solve_steady(inputs)
        s_bu = steady["wsel"][idx_bu]
        s_bd = steady["wsel"][idx_bd]
        u_bu = wsel_ts[step][idx_bu]
        u_bd = wsel_ts[step][idx_bd]
        d_bu = u_bu - s_bu
        d_bd = u_bd - s_bd
        max_bd = max(max_bd, abs(d_bd))
        hour = step * dt_out / 3600.0
        ctrl = (steady.get("culvert_control_types") or ["?"])[0]
        print(f"{hour:.1f}\t{q:.0f}\t{d_bu:+.3f}\t{d_bd:+.3f}\t{ctrl}")
    last = len(wsel_ts) - 1
    q = q_steps[-1]
    inputs = st.SteadyInputs(
        cross_sections=xs,
        flow_rate=float(q),
        downstream_wsel=DS_STAGE,
        downstream_bc_type=0,
        num_slices=int(params.get("vertical_slices", 100)),
        max_spacing=float(params.get("max_spacing", 100.0)),
        regime=0,
        **culvert,
    )
    steady = st.solve_steady(inputs)
    print(
        f"terminal\t{q:.0f}\t"
        f"{wsel_ts[last][idx_bu] - steady['wsel'][idx_bu]:+.3f}\t"
        f"{wsel_ts[last][idx_bd] - steady['wsel'][idx_bd]:+.3f}\t"
        f"{(steady.get('culvert_control_types') or ['?'])[0]}"
    )
    print(f"  max |ΔBD| vs quasi-steady: {max_bd:.3f} ft")


def main() -> int:
    parser = argparse.ArgumentParser(description="ConSpan culvert transition diagnostics")
    parser.add_argument("--mode", type=int, default=0, choices=(0, 2), help="coupling_mode")
    parser.add_argument("--steady-only", action="store_true")
    parser.add_argument("--unsteady-only", action="store_true")
    args = parser.parse_args()

    if not args.unsteady_only:
        steady_regime_sweep()
    if not args.steady_only:
        run_ramp_unsteady(args.mode)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
