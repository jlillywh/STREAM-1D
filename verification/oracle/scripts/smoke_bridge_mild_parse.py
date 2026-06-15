#!/usr/bin/env python3
"""Smoke-test bridge mild mapper payload."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

from lib.bridge_mild_mapper import build_bridge_mild_unsteady_inputs

project = ORACLE / "projects" / "bridge_mild"
for case in ("yarnell", "wspro"):
    for mode in (0, 2):
        payload, flow = build_bridge_mild_unsteady_inputs(project, case=case, coupling_mode=mode)
        assert payload["unsteady_structure_coupling_mode"] == mode
        assert payload["bridge_friction_weighting"] == [1]
        assert len(payload["cross_sections"]) == 5
        assert len(flow.upstream_q_cfs) == payload["num_steps"]
        print(f"OK {case} mode {mode}: {len(payload['cross_sections'])} XS, steps={payload['num_steps']}")

print("smoke OK")
