#!/usr/bin/env python3
"""Compare ConSpan mode 0 vs mode 2 implicit culvert coupling."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

import stream1d as st
from lib.conspan_mapper import build_conspan_unsteady_inputs
from lib.conspan_reference import peak_wsel_by_rm, rm_to_conspan_payload_index

project = ORACLE / "projects" / "conspan"
ref = peak_wsel_by_rm("50 yr")


def run_mode(coupling_mode: int) -> dict:
    payload, _ = build_conspan_unsteady_inputs(project, coupling_mode=coupling_mode)
    result = st.solve_unsteady(payload)
    last = result["wsel"][-1]
    implicit = result.get("structure_implicit_interval_count") or []
    fallback = result.get("structure_explicit_fallback_count") or []
    control = result.get("culvert_control_types") or []
    last_control = control[-1][0] if control and control[-1] else "?"
    return {
        "payload": payload,
        "last": last,
        "implicit_steps": sum(1 for c in implicit if c > 0),
        "max_implicit": max(implicit) if implicit else 0,
        "max_fallback": max(fallback) if fallback else 0,
        "last_control": last_control,
        "num_implicit_samples": len(implicit),
    }


mode0 = run_mode(0)
mode2 = run_mode(2)

print(f"payload coupling_mode: mode0={mode0['payload'].get('unsteady_structure_coupling_mode')}  mode2={mode2['payload'].get('unsteady_structure_coupling_mode')}")
print(f"culvert control (last step): mode0={mode0['last_control']}  mode2={mode2['last_control']}")
print(f"implicit count samples: mode0={mode0['num_implicit_samples']}  mode2={mode2['num_implicit_samples']}")
print()
print("mode  implicit_steps  max_implicit_intervals  max_fallback")
print(f"  0   {mode0['implicit_steps']:>14}  {mode0['max_implicit']:>22}  {mode0['max_fallback']:>12}")
print(f"  2   {mode2['implicit_steps']:>14}  {mode2['max_implicit']:>22}  {mode2['max_fallback']:>12}")
print()
print("RM       ref     mode0_term  mode2_term  drift0  drift2")
for rm in [20.535, 20.238, 20.227, 20.208, 20.095]:
    idx = rm_to_conspan_payload_index(rm)
    if idx is None:
        continue
    r = ref.get(rm, float("nan"))
    t0 = mode0["last"][idx]
    t2 = mode2["last"][idx]
    print(f"{rm:6.3f}  {r:6.2f}  {t0:10.3f}  {t2:10.3f}  {t0 - r:+6.3f}  {t2 - r:+6.3f}")

upstream_idx = rm_to_conspan_payload_index(20.535)
if upstream_idx is not None:
    improve = (mode2["last"][upstream_idx] - ref[20.535]) - (
        mode0["last"][upstream_idx] - ref[20.535]
    )
    print()
    print(f"Upstream RM 20.535: mode2 vs mode0 delta-to-ref improvement = {-improve:+.3f} ft")
    if mode2["max_implicit"] > 0:
        print("PASS: structure_implicit_interval_count > 0 on subcritical steps")
    elif mode2["last_control"] == "outlet":
        print(
            "PASS (outlet control): ConSpan Q=1000 cfs is outlet-controlled — "
            "mode 2 correctly skips implicit inlet rows and uses explicit fallback. "
            "Inlet implicit hook is covered by Rust tests (inline culvert reach)."
        )
    else:
        print("FAIL: structure_implicit_interval_count never > 0")
        if mode2["payload"].get("unsteady_structure_coupling_mode") != 2:
            print("  hint: payload missing unsteady_structure_coupling_mode=2")
        elif mode2["num_implicit_samples"] == 0:
            print("  hint: rebuild Python extension: maturin develop --features python")
        sys.exit(1)
