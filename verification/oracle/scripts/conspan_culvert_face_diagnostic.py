#!/usr/bin/env python3
"""
Step-1 diagnostic: culvert face WSEL vs isolated solve_culvert at ramp checkpoints.

Compares HEC-RAS reference faces to STREAM-1D unsteady faces and asks whether
`solve_culvert` (via compute_culvert_rating_curve) explains the mismatch.

Usage:
  PYTHONPATH=python python3 verification/oracle/scripts/conspan_culvert_face_diagnostic.py
"""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE_ROOT = Path(__file__).resolve().parents[1]
REPO_ROOT = ORACLE_ROOT.parents[1]
_PYTHON_PKG = REPO_ROOT / "python"
if str(_PYTHON_PKG) not in sys.path:
    sys.path.insert(0, str(_PYTHON_PKG))
if str(ORACLE_ROOT) not in sys.path:
    sys.path.insert(0, str(ORACLE_ROOT))

import stream1d as st  # noqa: E402

from lib.conspan_mapper import build_conspan_unsteady_inputs  # noqa: E402
from lib.conspan_reference import rm_to_conspan_payload_index  # noqa: E402
from lib.ras_reference import load_unsteady_timeseries_reference  # noqa: E402
from lib.scenario import load_scenario  # noqa: E402

RM_BU = 20.238
RM_BD = 20.227
CHECKPOINT_HRS = (24, 36, 48)
SCENARIO = ORACLE_ROOT / "scenarios" / "conspan_unsteady_ramp_matrix.json"


def _culvert_rating_params(payload: dict) -> dict:
    """Build compute_culvert_rating_curve payload from unsteady mapper output."""
    idx = 0
    return {
        "units": "USCustomary",
        "shape_type": payload["culvert_shape_types"][idx],
        "inlet_type": payload.get("culvert_inlet_types", [21])[idx],
        "span": payload["culvert_spans"][idx],
        "rise": payload["culvert_rises"][idx],
        "roughness_n": payload["culvert_roughness_ns"][idx],
        "length": payload["culvert_lengths"][idx],
        "entrance_loss_coeff": payload["culvert_entrance_loss_coeffs"][idx],
        "exit_loss_coeff": payload["culvert_exit_loss_coeffs"][idx],
        "num_barrels": payload.get("culvert_barrels", [1])[idx],
        "manning_n_bottom": payload.get("culvert_roughness_n_bottoms", [0.013])[idx],
        "depth_bottom_n": payload.get("culvert_depth_bottom_ns", [0.0])[idx],
        "depth_blocked": payload.get("culvert_depth_blockeds", [0.0])[idx],
        "z_up": payload.get("culvert_z_ups", [0.0])[idx],
        "z_down": payload.get("culvert_z_downs", [0.0])[idx],
        "weir_coeff": payload.get("culvert_weir_coeffs", [2.6])[idx]
        if payload.get("culvert_weir_coeffs")
        else 2.6,
        "weir_length": payload.get("culvert_weir_lengths", [0.0])[idx]
        if payload.get("culvert_weir_lengths")
        else 0.0,
        **(
            {"crest_elev": payload["culvert_crest_elevs"][idx]}
            if payload.get("culvert_crest_elevs")
            else {}
        ),
    }


def _solve_hw(q_cfs: float, tw_wsel: float, base: dict) -> dict:
    payload = dict(base)
    payload["q_values"] = [q_cfs]
    payload["tw_wsel"] = tw_wsel
    curve = st.compute_culvert_rating_curve(payload)
    return {
        "hw": curve["wsel"][0],
        "control": curve["control_types"][0],
        "wsel_inlet": curve["wsel_inlet"][0],
        "wsel_outlet": curve["wsel_outlet"][0],
    }


def main() -> int:
    scenario = load_scenario(SCENARIO)
    project_dir = scenario.linked_project_dir()
    linked = scenario.raw["linked_project"]
    coupling = int(scenario.raw.get("stream1d", {}).get("coupling_mode", 2))

    payload, flow = build_conspan_unsteady_inputs(
        project_dir,
        geometry_name=linked["geometry"],
        flow_name=linked["unsteady_flow"],
        plan_name=linked.get("plan"),
        coupling_mode=coupling,
    )
    dt = float(payload["dt"])
    result = st.solve_unsteady(payload)
    wsel_ts = result["wsel"]
    q_ts = result["q"]

    ref_series, ref_label = load_unsteady_timeseries_reference(scenario)
    culvert_base = _culvert_rating_params(payload)

    bu_idx = rm_to_conspan_payload_index(RM_BU)
    bd_idx = rm_to_conspan_payload_index(RM_BD)
    if bu_idx is None or bd_idx is None:
        raise RuntimeError(f"RM map failed: BU={bu_idx} BD={bd_idx}")

    print("=" * 72)
    print("ConSpan culvert face diagnostic (Step 1)")
    print(f"Scenario: {scenario.id}  coupling_mode={coupling}  dt={dt}s")
    print(f"Reference: {ref_label}")
    print(f"BU RM {RM_BU} → payload idx {bu_idx}   BD RM {RM_BD} → idx {bd_idx}")
    print("=" * 72)

    culvert_inlet = result.get("culvert_wsel_inlet")
    culvert_outlet = result.get("culvert_wsel_outlet")
    culvert_control = result.get("culvert_control_types")

    for hour in CHECKPOINT_HRS:
        step = int(round(float(hour) * 3600.0 / max(dt, 1.0)))
        step = min(step, len(wsel_ts) - 1)

        hec_bu = ref_series[RM_BU].get(float(hour)) or ref_series[RM_BU].get(float(int(hour)))
        hec_bd = ref_series[RM_BD].get(float(hour)) or ref_series[RM_BD].get(float(int(hour)))
        our_bu = wsel_ts[step][bu_idx]
        our_bd = wsel_ts[step][bd_idx]
        q_face = q_ts[step][bu_idx]

        hw_at_hec_tw = _solve_hw(q_face, hec_bd, culvert_base)
        hw_at_our_tw = _solve_hw(q_face, our_bd, culvert_base)

        diag_in = (
            culvert_inlet[step][0]
            if culvert_inlet and step < len(culvert_inlet)
            else None
        )
        diag_out = (
            culvert_outlet[step][0]
            if culvert_outlet and step < len(culvert_outlet)
            else None
        )
        diag_ctrl = (
            culvert_control[step][0]
            if culvert_control and step < len(culvert_control)
            else None
        )

        print(f"\n--- t = {hour} hr (step {step})  Q ≈ {q_face:.1f} cfs ---")
        print(f"  Face WSEL (ft):")
        print(f"    BU  HEC={hec_bu:.3f}  Ours={our_bu:.3f}  Δ(ours-hec)={our_bu - hec_bu:+.3f}")
        print(f"    BD  HEC={hec_bd:.3f}  Ours={our_bd:.3f}  Δ(ours-hec)={our_bd - hec_bd:+.3f}")
        print(f"  Isolated solve_culvert @ Q, TW (ft):")
        print(
            f"    @ HEC TW → HW={hw_at_hec_tw['hw']:.3f}  "
            f"control={hw_at_hec_tw['control']}  "
            f"Δ(HW - HEC BU)={hw_at_hec_tw['hw'] - hec_bu:+.3f}"
        )
        print(
            f"    @ Our TW → HW={hw_at_our_tw['hw']:.3f}  "
            f"control={hw_at_our_tw['control']}  "
            f"Δ(HW - Our BU)={hw_at_our_tw['hw'] - our_bu:+.3f}"
        )
        print(
            f"    @ Our TW vs HEC BU → Δ={hw_at_our_tw['hw'] - hec_bu:+.3f}  "
            f"(rating+coupling split)"
        )
        if diag_in is not None:
            print(
                f"  Unsteady culvert diagnostics: inlet={diag_in:.3f}  "
                f"outlet={diag_out:.3f}  control={diag_ctrl}"
            )

    print("\n" + "=" * 72)
    print("Interpretation:")
    print("  • |HW@HEC TW − HEC BU| small  → culvert solver OK; coupling / face mapping issue")
    print("  • |HW@HEC TW − HEC BU| large  → culvert hydraulics or mapper inputs vs HEC")
    print("  • |HW@Our TW − Our BU| large   → implicit/post-step not satisfying HW residual")
    print("=" * 72)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
