#!/usr/bin/env python3
"""Chunk 5 gate: steady warm-start initial WSEL vs ConSpan 50 yr steady (±0.04 ft)."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

from lib.conspan_mapper import build_conspan_unsteady_inputs
from lib.conspan_reference import peak_wsel_by_rm, rm_to_conspan_payload_index

TOLERANCE_FT = 0.04

project = ORACLE / "projects" / "conspan"
ref = peak_wsel_by_rm("50 yr")

for coupling_mode in (0, 2):
    payload, _ = build_conspan_unsteady_inputs(project, coupling_mode=coupling_mode)
    worst = 0.0
    worst_rm = 0.0
    for rm, idx in (
        (rm, rm_to_conspan_payload_index(rm))
        for rm in ref
        if rm_to_conspan_payload_index(rm) is not None
    ):
        init = payload["initial_wsel"][idx]
        expected = ref.get(rm)
        if expected is None:
            continue
        delta = abs(init - expected)
        if delta > worst:
            worst = delta
            worst_rm = rm
    passed = worst <= TOLERANCE_FT
    status = "PASS" if passed else "FAIL"
    print(
        f"mode {coupling_mode}: max |initial - steady| = {worst:.4f} ft at RM {worst_rm:.3f} "
        f"[{status}] (tol ±{TOLERANCE_FT} ft)"
    )
    if not passed:
        sys.exit(1)

print("warm-start OK")
