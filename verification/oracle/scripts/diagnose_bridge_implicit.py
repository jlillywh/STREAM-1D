#!/usr/bin/env python3
"""Compare bridge mild mode 0 vs mode 2 implicit coupling."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

import stream1d as st
from lib.bridge_mild_mapper import build_bridge_mild_unsteady_inputs, station_to_payload_index

project = ORACLE / "projects" / "bridge_mild"
ref_path = project / "reference_wsel_bridge_mild_unsteady.json"
ref = {float(c["rm"]): float(c["max_wsel_ft"]) for c in json.loads(ref_path.read_text())["checkpoints"]}
M_TO_FT = 3.280839895


def run_mode(coupling_mode: int) -> dict:
    payload, _ = build_bridge_mild_unsteady_inputs(project, case="yarnell", coupling_mode=coupling_mode)
    result = st.solve_unsteady(payload)
    last = result["wsel"][-1]
    implicit = result.get("structure_implicit_interval_count") or []
    fallback = result.get("structure_explicit_fallback_count") or []
    regimes = result.get("bridge_flow_regimes") or []
    last_regime = regimes[-1][0] if regimes and regimes[-1] else "?"
    return {
        "payload": payload,
        "last": last,
        "implicit_steps": sum(1 for c in implicit if c > 0),
        "max_implicit": max(implicit) if implicit else 0,
        "max_fallback": max(fallback) if fallback else 0,
        "last_regime": last_regime,
    }


mode0 = run_mode(0)
mode2 = run_mode(2)

print(
    f"coupling_mode: mode0={mode0['payload'].get('unsteady_structure_coupling_mode')}  "
    f"mode2={mode2['payload'].get('unsteady_structure_coupling_mode')}"
)
print(f"bridge regime (last step): mode0={mode0['last_regime']}  mode2={mode2['last_regime']}")
print()
print("mode  implicit_steps  max_implicit_intervals  max_fallback")
print(f"  0   {mode0['implicit_steps']:>14}  {mode0['max_implicit']:>22}  {mode0['max_fallback']:>12}")
print(f"  2   {mode2['implicit_steps']:>14}  {mode2['max_implicit']:>22}  {mode2['max_fallback']:>12}")
print()
print("sta(m)   ref     mode0_term  mode2_term  drift0  drift2")
for sta in [52.0, 48.0, 25.0, 0.0]:
    idx = station_to_payload_index(sta, mode0["payload"])
    if idx is None:
        continue
    r = ref.get(sta, float("nan"))
    t0 = mode0["last"][idx] * M_TO_FT
    t2 = mode2["last"][idx] * M_TO_FT
    print(f"{sta:6.1f}  {r:6.3f}  {t0:10.3f}  {t2:10.3f}  {t0 - r:+6.3f}  {t2 - r:+6.3f}")

if mode2["max_implicit"] > 0:
    print()
    print("PASS: structure_implicit_interval_count > 0 on subcritical bridge steps")
elif not all(w == w and abs(w) < 1e6 for w in mode2["last"]):
    print("FAIL: NaN/Inf in mode 2 terminal WSEL")
    sys.exit(1)
else:
    print()
    print("FAIL: structure_implicit_interval_count never > 0 for subcritical bridge")
    sys.exit(1)

# Regime-crossing sanity (TW ramp uses explicit fallback — Rust test covers engine path)
print()
print("Regime crossing: see Rust test_unsteady_implicit_bridge_tw_ramp_uses_explicit_fallback")
