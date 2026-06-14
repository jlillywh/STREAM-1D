#!/usr/bin/env python3
"""Diagnose ConSpan mild unsteady WSEL vs steady reference."""

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
payload, _ = build_conspan_unsteady_inputs(project)
ref = peak_wsel_by_rm("50 yr")

un = st.solve_unsteady(payload)
last = un["wsel"][-1]

print("RM       ref    initial  terminal  max")
for rm in [20.535, 20.422, 20.308, 20.251, 20.238, 20.227, 20.208, 20.095]:
    idx = rm_to_conspan_payload_index(rm)
    if idx is None:
        continue
    init = payload["initial_wsel"][idx]
    term = last[idx]
    mx = max(s[idx] for s in un["wsel"])
    print(f"{rm:6.3f}  {ref.get(rm, float('nan')):6.2f}  {init:8.3f}  {term:9.3f}  {mx:6.3f}")
