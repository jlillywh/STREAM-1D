#!/usr/bin/env python3
"""
Beaver Creek bridge — layered restart diagnostic.

Replaces the old Chunk 8 pass/fail gate. Isolates mapping, steady physics, and
unsteady integration before comparing to HEC-RAS HDF reference.

Layers (run in order):
  1. Mapping — g01/u02/p03 parse + bridge field audit (no solve)
  2. Steady   — solve_steady at initial Q and peak Q vs Observed HWM (dev proxy)
  3. Unsteady — optional full hydrograph max WSEL (diagnostic only)

Exit 0 always (reports gaps; certification requires HDF WSEL(t)).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

import stream1d as st
from lib.beaver_mapper import build_beaver_unsteady_inputs, rm_to_payload_index
from lib.bridge_mapper import build_bridge_fields
from lib.hecras_geom_parser import cross_section_by_description, parse_g01
from lib.hecras_plan_parser import find_plan_file, parse_plan
from lib.hecras_unsteady_parser import parse_unsteady_flow

PROJECT = ORACLE / "projects" / "beaver"
SCENARIO_PATH = ORACLE / "scenarios" / "beaver_unsteady_linked.json"
CHECKPOINTS = [5.99, 5.875, 5.76, 5.685, 5.61, 5.41, 5.39, 5.13, 5.065, 5.0]

BRIDGE_FIELD_KEYS = (
    "bridge_stations",
    "bridge_deck_stations",
    "bridge_pier_stations",
    "bridge_num_piers",
    "bridge_low_flow_methods",
    "bridge_lengths",
    "bridge_friction_weighting",
    "bridge_upstream_cross_sections",
    "bridge_downstream_cross_sections",
    "bridge_approach_cross_sections",
    "bridge_departure_cross_sections",
    "bridge_internal_cross_sections",
    "bridge_ineffective_left_stations_upstream",
    "bridge_ineffective_right_stations_upstream",
    "bridge_high_flow_methods",
    "bridge_pressure_flow_coeffs_inlet",
    "bridge_roadway_embankments",
)


def _steady_wsel(payload: dict[str, Any], flow_cfs: float) -> list[float]:
    kwargs: dict[str, Any] = {
        "cross_sections": payload["cross_sections"],
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
    elif payload.get("downstream_bc_type") == 3:
        kwargs["downstream_bc_rating_q"] = payload.get("downstream_bc_rating_q") or []
        kwargs["downstream_bc_rating_wsel"] = payload.get("downstream_bc_rating_wsel") or []
    else:
        ds = payload.get("downstream_wsel_hydrograph") or []
        kwargs["downstream_wsel"] = ds[0] if ds else 0.0
    for key in BRIDGE_FIELD_KEYS:
        val = payload.get(key)
        if val is not None:
            kwargs[key] = val
    for key in payload:
        if key.startswith("bridge_") and key not in kwargs:
            kwargs[key] = payload[key]
    return list(st.solve_steady(kwargs)["wsel"])


def _compare_steady_layer(
    label: str,
    flow_cfs: float,
    payload: dict[str, Any],
    geom_xs,
    ref_hwm: dict[float, float],
) -> dict[str, Any]:
    wsel = _steady_wsel(payload, flow_cfs)
    rows: list[dict[str, Any]] = []
    worst = 0.0
    worst_rm = 0.0
    print(f"\n=== Layer 2 — steady @ {label} ({flow_cfs:.0f} cfs) vs Observed HWM ===")
    print(f"{'RM':>8}  {'RAS HWM':>10}  {'steady':>10}  {'Δ':>8}")
    print("-" * 44)
    for rm in CHECKPOINTS:
        ras = ref_hwm.get(rm)
        idx = rm_to_payload_index(rm, geom_xs)
        if ras is None or idx is None:
            continue
        s1d = wsel[idx]
        delta = s1d - ras
        print(f"{rm:8.3f}  {ras:10.3f}  {s1d:10.3f}  {delta:+8.3f}")
        rows.append({"river_mile": rm, "ras_hwm_ft": ras, "steady_wsel_ft": s1d, "delta_ft": delta})
        if abs(delta) > abs(worst):
            worst = delta
            worst_rm = rm
    print(f"max |Δ| = {abs(worst):.3f} ft at RM {worst_rm:.3f}")
    return {
        "flow_cfs": flow_cfs,
        "label": label,
        "max_abs_delta_ft": abs(worst),
        "worst_rm": worst_rm,
        "checkpoints": rows,
    }


def _mapping_layer(geom, flow, plan, payload: dict[str, Any]) -> dict[str, Any]:
    bridge = geom.bridge
    bu = cross_section_by_description(geom.cross_sections, "upstream of bridge")
    bd = cross_section_by_description(geom.cross_sections, "downstream of bridge")
    fields = build_bridge_fields(geom)

    print("=== Layer 1 — mapping audit ===")
    print(f"XS: {len(geom.cross_sections)}  bridge RM: {bridge.rm if bridge else '?'}")
    if plan:
        print(
            f"plan {plan.plan_number}: theta={plan.unsteady_theta}  "
            f"dt={plan.computation_interval_seconds}s  "
            f"friction_slope_method={plan.unsteady_friction_slope_method}"
        )
    if bridge:
        print(
            f"deck pts={len(bridge.deck_low_stations)}  piers={len(bridge.pier_stations)}  "
            f"length={bridge.bridge_length} ft  WSPRO={bridge.low_flow_method}"
        )
    print(f"BU RM={bu.rm if bu else '?'}  BD RM={bd.rm if bd else '?'}")
    print(f"Q steps={len(flow.upstream_q_cfs)}  peak Q={max(flow.upstream_q_cfs):.0f} cfs")
    print(f"mapped steps={payload['num_steps']}  dt={payload['dt']}s  coupling={payload.get('unsteady_structure_coupling_mode')}")
    missing = [k for k in BRIDGE_FIELD_KEYS if k not in payload and k not in fields]
    if missing:
        print(f"WARN missing bridge fields: {', '.join(missing)}")
    else:
        print("bridge field checklist: OK")
    if payload.get("bridge_roadway_embankments"):
        emb = payload["bridge_roadway_embankments"][0]
        print(
            f"roadway embankment: opening {emb['left']['embankment_profile']['stations'][-1]:.0f}"
            f"–{emb['right']['embankment_profile']['stations'][0]:.0f} ft"
        )
    return {
        "xs_count": len(geom.cross_sections),
        "bridge_rm": bridge.rm if bridge else None,
        "bu_rm": bu.rm if bu else None,
        "bd_rm": bd.rm if bd else None,
        "peak_q_cfs": max(flow.upstream_q_cfs),
        "mapped_steps": payload["num_steps"],
        "dt_seconds": payload["dt"],
        "coupling_mode": payload.get("unsteady_structure_coupling_mode"),
        "friction_slope_method": payload.get("unsteady_friction_slope_method"),
        "downstream_bc_type": payload.get("downstream_bc_type"),
    }


def _unsteady_layer(payload: dict[str, Any], geom_xs, ref_hwm: dict[float, float]) -> dict[str, Any]:
    print("\n=== Layer 3 — unsteady max WSEL (diagnostic) ===")
    result = st.solve_unsteady(payload)
    rows: list[dict[str, Any]] = []
    worst = 0.0
    worst_rm = 0.0
    print(f"{'RM':>8}  {'RAS HWM':>10}  {'max WSEL':>10}  {'Δ':>8}")
    print("-" * 44)
    for rm in CHECKPOINTS:
        ras = ref_hwm.get(rm)
        idx = rm_to_payload_index(rm, geom_xs)
        if ras is None or idx is None:
            continue
        peak = max(step[idx] for step in result["wsel"] if idx < len(step))
        delta = peak - ras
        print(f"{rm:8.3f}  {ras:10.3f}  {peak:10.3f}  {delta:+8.3f}")
        rows.append({"river_mile": rm, "ras_hwm_ft": ras, "max_wsel_ft": peak, "delta_ft": delta})
        if abs(delta) > abs(worst):
            worst = delta
            worst_rm = rm
    regimes = result.get("bridge_flow_regimes") or []
    regime_counts: dict[str, int] = {}
    for step_regs in regimes:
        for reg in step_regs:
            regime_counts[reg] = regime_counts.get(reg, 0) + 1
    print(f"max |Δ| = {abs(worst):.3f} ft at RM {worst_rm:.3f}")
    print(f"bridge regimes: {regime_counts}")
    return {
        "max_abs_delta_ft": abs(worst),
        "worst_rm": worst_rm,
        "checkpoints": rows,
        "regime_counts": regime_counts,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Beaver bridge layered restart diagnostic")
    parser.add_argument("--skip-unsteady", action="store_true", help="Skip Layer 3 (faster)")
    parser.add_argument("--out", type=Path, help="Write JSON report path")
    args = parser.parse_args()

    geom = parse_g01(PROJECT / "beaver.g01")
    flow = parse_unsteady_flow(PROJECT / "beaver.u02")
    plan_path = find_plan_file(PROJECT)
    plan = parse_plan(plan_path) if plan_path else None
    payload, _ = build_beaver_unsteady_inputs(PROJECT, coupling_mode=2)

    report: dict[str, Any] = {
        "schema_version": 2,
        "scenario_id": "beaver_unsteady_linked",
        "restart_phase": "layered_diagnostic",
        "reference_source": "linked_u02_observed_hwm",
        "reference_note": "Observed HWM is a dev proxy only — certification needs HDF WSEL(t)",
        "certification_status": "development",
    }

    report["mapping"] = _mapping_layer(geom, flow, plan, payload)
    report["steady_initial_q"] = _compare_steady_layer(
        "initial Q", flow.initial_flow_cfs, payload, geom.cross_sections, flow.observed_hwm
    )
    peak_q = max(flow.upstream_q_cfs)
    report["steady_peak_q"] = _compare_steady_layer(
        "peak Q", peak_q, payload, geom.cross_sections, flow.observed_hwm
    )

    if not args.skip_unsteady:
        report["unsteady"] = _unsteady_layer(payload, geom.cross_sections, flow.observed_hwm)

    out_path = args.out or (PROJECT / "restart_report_beaver.json")
    out_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"\nWrote report: {out_path}")
    print("\nRESULT: diagnostic complete (no pass/fail — use steady layers to isolate bridge vs unsteady)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
