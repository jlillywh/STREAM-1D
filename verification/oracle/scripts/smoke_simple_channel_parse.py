#!/usr/bin/env python3
"""Smoke test: simple_channel g01/u02 parse (friction-slope DS)."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
sys.path.insert(0, str(ORACLE))
sys.path.insert(0, str(ROOT / "python"))

from lib.hecras_geom_parser import parse_g01  # noqa: E402
from lib.hecras_unsteady_parser import parse_unsteady_flow  # noqa: E402
from lib.simple_channel_mapper import build_simple_channel_unsteady_inputs  # noqa: E402

PROJECT = ORACLE / "projects" / "simple_channel"


def main() -> int:
    g = parse_g01(PROJECT / "simple_channel.g01")
    flow = parse_unsteady_flow(PROJECT / "simple_channel.u02")
    payload, _ = build_simple_channel_unsteady_inputs(PROJECT)
    rms = [xs.rm for xs in g.cross_sections]
    print(f"XS count: {len(g.cross_sections)}  RMs: {rms}")
    print(
        f"Q steps: {len(flow.upstream_q_cfs)}  "
        f"DS friction slope: {flow.downstream_friction_slope}  "
        f"DS stage steps: {len(flow.downstream_stage_hydrograph)}"
    )
    print(
        f"Payload: downstream_bc_type={payload.get('downstream_bc_type')}  "
        f"downstream_bc_slope={payload.get('downstream_bc_slope')}  "
        f"num_steps={payload.get('num_steps')}"
    )
    assert flow.downstream_friction_slope == 0.001, flow.downstream_friction_slope
    assert not flow.downstream_stage_hydrograph
    assert payload["downstream_bc_type"] == 2
    assert payload["downstream_bc_slope"] == 0.001
    assert len(g.cross_sections) == 4
    assert len(flow.upstream_q_cfs) == 49
    text = (PROJECT / "simple_channel.u02").read_text(encoding="utf-8")
    assert "Friction Slope=0.001,0" in text or "Friction Slope=0.001, 0" in text
    assert "Stage Hydrograph=" not in text
    flow_idx = text.index("Flow Hydrograph=")
    ds_idx = text.index("Boundary Location=", text.index("0.0"))
    assert flow_idx < ds_idx, "downstream boundary must follow upstream hydrograph"
    print("OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
