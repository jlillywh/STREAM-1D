#!/usr/bin/env python3
"""Smoke test for ConSpan mild unsteady mapper (requires stream1d)."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ORACLE.parent.parent / "python"))

from lib.conspan_mapper import build_conspan_unsteady_inputs  # noqa: E402
from lib.conspan_reference import conspan_geometry_rms_upstream_first  # noqa: E402
from lib.hecras_geom_parser import parse_g01  # noqa: E402
from lib.hecras_unsteady_parser import parse_unsteady_flow  # noqa: E402

PROJECT = ORACLE / "projects" / "conspan"

geom = parse_g01(PROJECT / "ConSpan.g01")
flow = parse_unsteady_flow(PROJECT / "conspan.u02")
payload, _ = build_conspan_unsteady_inputs(PROJECT)

print("g01_xs", len(geom.cross_sections))
print("payload_xs", len(payload["cross_sections"]))
print("culvert_stations", payload.get("culvert_stations"))
print("num_steps", payload.get("num_steps"))
print("Q0", flow.upstream_q_cfs[0], "Q_last", flow.upstream_q_cfs[-1])
print("DS_stage", flow.downstream_stage_hydrograph[0] if flow.downstream_stage_hydrograph else None)
print("coupling_mode", payload.get("unsteady_structure_coupling_mode", 0))
print("rms", conspan_geometry_rms_upstream_first())
print("OK")
