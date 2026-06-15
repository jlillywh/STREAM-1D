#!/usr/bin/env python3
"""Diagnose reach_mild WSEL vs ConSpan reference."""
from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

import stream1d as st
from lib.hecras_geom_parser import parse_g01, parsed_xs_to_dict, rm_to_station
from lib.reach_mapper import build_reach_unsteady_inputs

project = ORACLE / "projects" / "reach_mild"
payload, _flow = build_reach_unsteady_inputs(project)
geom = parse_g01(project / "reach_mild.g01")
xs_list = [st.CrossSection(**parsed_xs_to_dict(x)) for x in geom.cross_sections]

for label, kwargs in [
    ("friction_slope", {"downstream_bc_type": 2, "downstream_bc_slope": 0.001}),
    ("known_ds_31.5", {"downstream_bc_type": 0, "downstream_wsel": 31.5}),
    ("known_ds_32.92", {"downstream_bc_type": 0, "downstream_wsel": 32.92}),
]:
    r = st.solve_steady(st.SteadyInputs(cross_sections=xs_list, flow_rate=1000, regime=0, max_spacing=200, **kwargs))
    print(f"\n=== steady {label} ===")
    for xs in geom.cross_sections:
        idx = next(i for i, x in enumerate(geom.cross_sections) if x.rm == xs.rm)
        print(f"  RM {xs.rm:.3f}  WSEL {r['wsel'][idx]:.3f}")

un = st.solve_unsteady(payload)
last = un["wsel"][-1]
print("\n=== unsteady (mapper payload) ===")
rms = [20.535, 20.422, 20.308, 20.095]
ref = [33.72, 33.41, 33.14, 31.0]
print("RM       ref    last    max")
for rm, refv in zip(rms, ref):
    stn = rm_to_station(geom.cross_sections, rm)
    idx = min(range(len(geom.cross_sections)), key=lambda i: abs(geom.cross_sections[i].station - stn))
    mx = max(s[idx] for s in un["wsel"])
    print(f"{rm:6.3f}  {refv:6.2f}  {last[idx]:6.2f}  {mx:6.2f}")

print("\ninitial_wsel in payload:", payload.get("initial_wsel"))
print("downstream_bc_type:", payload.get("downstream_bc_type"))
