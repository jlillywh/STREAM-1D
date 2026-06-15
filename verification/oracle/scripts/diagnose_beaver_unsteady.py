#!/usr/bin/env python3
"""
Beaver Creek Chunk 8 diagnostic — published gap table vs Observed HWM (plan 03).

Compares coupling modes 0 and 2 at checkpoint RMs. Does not fudge reference:
Observed HWM from beaver.u02 is the dev reference until HDF WSEL(t) lands.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

import stream1d as st
from lib.beaver_mapper import build_beaver_unsteady_inputs, rm_to_payload_index
from lib.hecras_geom_parser import parse_g01
from lib.hecras_plan_parser import find_plan_file, parse_plan
from lib.hecras_unsteady_parser import parse_unsteady_flow

PROJECT = ORACLE / "projects" / "beaver"
SCENARIO_PATH = ORACLE / "scenarios" / "beaver_unsteady_linked.json"
TOLERANCE_FT = 0.5
CHECKPOINTS = [5.99, 5.875, 5.76, 5.685, 5.61, 5.41, 5.39, 5.13, 5.065, 5.0]


def max_wsel_at_rm(wsel_series: list[list[float]], idx: int) -> float:
    return max(step[idx] for step in wsel_series if idx < len(step))


def run_mode(coupling_mode: int, geom_xs) -> dict:
    payload, flow = build_beaver_unsteady_inputs(PROJECT)
    payload["unsteady_structure_coupling_mode"] = coupling_mode
    result = st.solve_unsteady(payload)
    peaks: dict[float, float] = {}
    for rm in CHECKPOINTS:
        idx = rm_to_payload_index(rm, geom_xs)
        if idx is not None:
            peaks[rm] = max_wsel_at_rm(result["wsel"], idx)
    regimes = result.get("bridge_flow_regimes") or []
    regime_counts: dict[str, int] = {}
    for step_regs in regimes:
        for reg in step_regs:
            regime_counts[reg] = regime_counts.get(reg, 0) + 1
    implicit = result.get("structure_implicit_interval_count") or []
    fallback = result.get("structure_explicit_fallback_count") or []
    return {
        "payload": payload,
        "flow": flow,
        "peaks": peaks,
        "regime_counts": regime_counts,
        "implicit_steps": sum(1 for c in implicit if c > 0),
        "max_fallback": max(fallback) if fallback else 0,
        "converged_all": all(result.get("structure_coupling_converged") or [True]),
    }


def main() -> int:
    scenario = json.loads(SCENARIO_PATH.read_text(encoding="utf-8"))
    flow = parse_unsteady_flow(PROJECT / "beaver.u02")
    geom = parse_g01(PROJECT / "beaver.g01")
    plan_path = find_plan_file(PROJECT)
    plan = parse_plan(plan_path) if plan_path else None

    ref = flow.observed_hwm
    mode0 = run_mode(0, geom.cross_sections)
    mode2 = run_mode(2, geom.cross_sections)

    print("=== Beaver Creek — Chunk 8 diagnostic ===")
    print(f"scenario: {scenario['id']}  certification: {scenario['parity_program']['certification']}")
    print(f"XS count: {len(geom.cross_sections)}  bridge RM: {geom.bridge.rm if geom.bridge else '?'}")
    if plan:
        print(
            f"plan {plan.plan_number}: theta={plan.unsteady_theta}  "
            f"computation_interval={plan.computation_interval_seconds}s"
        )
    print(
        f"Q hydrograph: {len(flow.upstream_q_cfs)} hourly steps → "
        f"{mode0['payload']['num_steps']} steps @ dt={mode0['payload']['dt']}s"
    )
    print(f"peak Q = {max(flow.upstream_q_cfs):.0f} cfs")
    print(
        f"downstream_bc_type={mode0['payload'].get('downstream_bc_type')}  "
        f"slope={mode0['payload'].get('downstream_bc_slope')}  "
        f"(2 = friction slope, dynamic normal depth)"
    )
    print()

    rows = []
    worst_mode0 = 0.0
    worst_mode2 = 0.0
    worst_rm = 0.0
    print(f"{'RM':>8}  {'RAS HWM':>10}  {'mode0 max':>10}  {'mode2 max':>10}  {'Δ0':>8}  {'Δ2':>8}  Status")
    print("-" * 72)
    for rm in CHECKPOINTS:
        ras = ref.get(rm)
        if ras is None:
            continue
        p0 = mode0["peaks"].get(rm)
        p2 = mode2["peaks"].get(rm)
        if p0 is None or p2 is None:
            continue
        d0 = p0 - ras
        d2 = p2 - ras
        passed = abs(d0) <= TOLERANCE_FT and abs(d2) <= TOLERANCE_FT
        status = "PASS" if passed else "FAIL"
        print(f"{rm:8.3f}  {ras:10.3f}  {p0:10.3f}  {p2:10.3f}  {d0:+8.3f}  {d2:+8.3f}  [{status}]")
        rows.append(
            {
                "river_mile": rm,
                "ras_observed_hwm_ft": ras,
                "stream1d_mode0_max_wsel_ft": p0,
                "stream1d_mode2_max_wsel_ft": p2,
                "delta_mode0_ft": d0,
                "delta_mode2_ft": d2,
                "passed": passed,
            }
        )
        if abs(d0) > abs(worst_mode0):
            worst_mode0 = d0
            worst_rm = rm
        if abs(d2) > abs(worst_mode2):
            worst_mode2 = d2

    print("-" * 72)
    max_abs_0 = max(abs(r["delta_mode0_ft"]) for r in rows) if rows else 0.0
    max_abs_2 = max(abs(r["delta_mode2_ft"]) for r in rows) if rows else 0.0
    overall_pass = all(r["passed"] for r in rows)
    print(f"max |Δ| mode 0 = {max_abs_0:.3f} ft   mode 2 = {max_abs_2:.3f} ft   tol ±{TOLERANCE_FT} ft")
    print()

    print("=== high-flow bridge (100-yr) ===")
    print(f"mode 0 regime histogram: {mode0['regime_counts']}")
    print(f"mode 2 regime histogram: {mode2['regime_counts']}")
    print(f"mode 2 implicit steps: {mode0['implicit_steps']} vs {mode2['implicit_steps']}")
    print(f"mode 2 max explicit fallback/step: {mode2['max_fallback']}")
    high_flow = sum(
        v for k, v in mode2["regime_counts"].items()
        if k in ("pressure", "weir", "energy", "combined") or k.startswith("high")
    )
    if high_flow:
        print(
            "NOTE: high-flow intervals use explicit post-step fallback (B3) — "
            "implicit low-flow not claimed for 100-yr Beaver."
        )
    print()

    gap_doc = {
        "schema_version": 1,
        "scenario_id": "beaver_unsteady_linked",
        "reference_source": "linked_u02_observed_hwm",
        "tolerance_ft": TOLERANCE_FT,
        "overall_pass": overall_pass,
        "max_abs_delta_mode0_ft": max_abs_0,
        "max_abs_delta_mode2_ft": max_abs_2,
        "checkpoints": rows,
        "mapping_notes": scenario["stream1d"]["notes"],
        "certification_status": "development",
        "hdf_required": True,
    }
    out_path = PROJECT / "gap_table_beaver_unsteady.json"
    out_path.write_text(json.dumps(gap_doc, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote gap table: {out_path}")

    print()
    if overall_pass:
        print("RESULT: PASS (Observed HWM dev reference)")
    else:
        print("RESULT: FAIL — published gap table (no mapper fudge)")
        print(f"  worst mode 0 Δ at RM {worst_rm}: {worst_mode0:+.3f} ft")
        print(f"  worst mode 2 Δ at RM {worst_rm if worst_mode2 else worst_rm}: {worst_mode2:+.3f} ft")
        print("  Re-run after HDF reference + BC alignment for certification.")

    return 0 if overall_pass else 1


if __name__ == "__main__":
    raise SystemExit(main())
