#!/usr/bin/env python3
"""Smoke test: parse ramp u04/u05 and validate hydrograph shape."""

from __future__ import annotations

import sys
from pathlib import Path

ORACLE = Path(__file__).resolve().parents[1]
ROOT = ORACLE.parents[1]
_PYTHON = ROOT / "python"
if _PYTHON.is_dir() and str(_PYTHON) not in sys.path:
    sys.path.insert(0, str(_PYTHON))
sys.path.insert(0, str(ORACLE))

from lib.hecras_unsteady_parser import parse_unsteady_flow  # noqa: E402
from lib.simple_channel_mapper import build_simple_channel_unsteady_inputs  # noqa: E402

PROJECT = ORACLE / "projects" / "simple_channel"

EXPECTED_Q = {
    0: 100.0,
    6: 150.0,
    12: 200.0,
    24: 200.0,
    36: 100.0,
    48: 100.0,
}


def _check_flow(path: Path, *, ds_type: int) -> None:
    flow = parse_unsteady_flow(path)
    assert len(flow.upstream_q_cfs) == 49, len(flow.upstream_q_cfs)
    assert abs(flow.initial_flow_cfs - 100.0) < 0.01
    for hour, expected in EXPECTED_Q.items():
        got = flow.upstream_q_cfs[hour]
        assert abs(got - expected) < 0.02, f"hour {hour}: got {got}, expected {expected}"

    payload, _ = build_simple_channel_unsteady_inputs(
        PROJECT,
        geometry_name="simple_channel.g01",
        flow_name=path.name,
    )
    assert payload["downstream_bc_type"] == ds_type
    assert payload["num_steps"] == 49
    assert len(payload["upstream_q_hydrograph"]) == 49
    print(f"OK {path.name}  ds_type={ds_type}  Q@12={flow.upstream_q_cfs[12]:.1f} cfs")


def main() -> int:
    write_script = ORACLE / "scripts" / "write_simple_channel_ramp.py"
    if not (PROJECT / "simple_channel.u04").is_file():
        import subprocess

        subprocess.run([sys.executable, str(write_script)], check=True, cwd=ORACLE.parents[1])

    _check_flow(PROJECT / "simple_channel.u04", ds_type=2)
    _check_flow(PROJECT / "simple_channel.u05", ds_type=3)
    print("simple_channel ramp parse smoke: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
