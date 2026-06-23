#!/usr/bin/env python3
"""
Summarize mode-3 (MonolithicNewton) outer-loop convergence per time step.

Usage:
  maturin develop --features python --release
  PYTHONPATH=python python3 verification/oracle/scripts/conspan_monolithic_newton_diagnostic.py

Optional:
  .../conspan_monolithic_newton_diagnostic.py --scenario path/to/scenario.json
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ORACLE_ROOT = Path(__file__).resolve().parents[1]
REPO_ROOT = ORACLE_ROOT.parents[1]
_PYTHON_PKG = REPO_ROOT / "python"
if str(_PYTHON_PKG) not in sys.path:
    sys.path.insert(0, str(_PYTHON_PKG))
if str(ORACLE_ROOT) not in sys.path:
    sys.path.insert(0, str(ORACLE_ROOT))

import stream1d as st  # noqa: E402

from lib.conspan_mapper import build_conspan_unsteady_inputs  # noqa: E402
from lib.scenario import load_scenario  # noqa: E402

# Matches MONOLITHIC_NEWTON_TOL_M in preissmann.rs (~0.01 ft stage scale).
NEWTON_TOL_M = 0.003
M_TO_FT = 3.28084
DEFAULT_SCENARIO = ORACLE_ROOT / "scenarios" / "conspan_unsteady_ramp_matrix_mode3.json"


def _run(scenario_path: Path) -> dict:
    scenario = load_scenario(scenario_path)
    linked = scenario.raw["linked_project"]
    stream1d_cfg = scenario.raw.get("stream1d", {})
    friction_override = stream1d_cfg.get("unsteady_friction_slope_method")
    payload, _flow = build_conspan_unsteady_inputs(
        scenario.linked_project_dir(),
        geometry_name=linked["geometry"],
        flow_name=linked["unsteady_flow"],
        plan_name=linked.get("plan"),
        coupling_mode=int(stream1d_cfg.get("coupling_mode", 3)),
        unsteady_friction_slope_method=(
            int(friction_override) if friction_override is not None else None
        ),
    )
    return st.solve_unsteady(payload)


def _summarize(result: dict) -> None:
    converged = result.get("monolithic_newton_converged")
    iterations = result.get("monolithic_newton_iterations")
    initial = result.get("monolithic_newton_initial_residual")
    final = result.get("monolithic_newton_max_residual")
    mom = result.get("monolithic_newton_momentum_residual")
    cont = result.get("monolithic_newton_continuity_residual")

    if not converged:
        print("No monolithic_newton_* fields in result (is unsteady_structure_coupling_mode=3?)")
        sys.exit(1)

    n = len(converged)
    n_ok = sum(1 for c in converged if c)
    print(f"Steps: {n}  |  Newton converged: {n_ok}/{n} ({100.0 * n_ok / n:.1f}%)")
    print(f"Tolerance: {NEWTON_TOL_M:.4f} m (~{NEWTON_TOL_M * M_TO_FT:.3f} ft) max |RHS|")
    print()

    if final:
        worst_i = max(range(n), key=lambda i: final[i])
        print(
            f"Worst final |RHS|: step {worst_i}  "
            f"max={final[worst_i]:.4f} m ({final[worst_i] * M_TO_FT:.3f} ft)  "
            f"iters={iterations[worst_i]}  converged={converged[worst_i]}"
        )
    if initial and final:
        print(
            f"Initial |RHS| at step 0: {initial[0]:.4f} m  →  "
            f"final: {final[0]:.4f} m  (iters={iterations[0]})"
        )
    print()

    failed = [i for i, c in enumerate(converged) if not c]
    if failed:
        print(f"Non-converged steps ({len(failed)}): {failed[:20]}{'...' if len(failed) > 20 else ''}")
        print("  step   iters   init_R(m)   final_R(m)   mom_R     cont_R")
        for i in failed[:15]:
            line = (
                f"  {i:4d}  {iterations[i]:5d}  "
                f"{initial[i]:10.4f}  {final[i]:10.4f}"
            )
            if mom and cont:
                line += f"  {mom[i]:8.4f}  {cont[i]:8.4f}"
            print(line)
    else:
        print("All steps converged within tolerance.")

    if iterations:
        it = iterations
        print()
        print(f"Iterations: min={min(it)}  max={max(it)}  mean={sum(it) / len(it):.1f}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--scenario",
        type=Path,
        default=DEFAULT_SCENARIO,
        help=f"Oracle scenario JSON (default: {DEFAULT_SCENARIO.name})",
    )
    args = parser.parse_args()
    print(f"Scenario: {args.scenario}")
    print("Running unsteady (mode 3)...")
    result = _run(args.scenario)
    print()
    _summarize(result)


if __name__ == "__main__":
    main()
