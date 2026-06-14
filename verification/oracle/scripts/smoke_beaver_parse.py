"""Smoke test for Beaver linked parsers (run from repo root)."""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from lib.beaver_mapper import build_beaver_unsteady_inputs  # noqa: E402
from lib.hecras_geom_parser import cross_section_by_description, parse_g01  # noqa: E402
from lib.hecras_plan_parser import find_plan_file, parse_plan  # noqa: E402
from lib.hecras_unsteady_parser import parse_unsteady_flow  # noqa: E402

p = ROOT / "projects" / "beaver"
g = parse_g01(p / "beaver.g01")
f = parse_unsteady_flow(p / "beaver.u02")
plan_path = find_plan_file(p)
if plan_path:
    plan = parse_plan(plan_path)
    print("plan", plan.plan_number, "theta", plan.unsteady_theta, "dt", plan.computation_interval_seconds)
print("XS", len(g.cross_sections))
if g.bridge:
    print(
        "bridge_rm",
        g.bridge.rm,
        "deck_pts",
        len(g.bridge.deck_low_stations),
        "piers",
        len(g.bridge.pier_stations),
        "length",
        g.bridge.bridge_length,
    )
bu = cross_section_by_description(g.cross_sections, "upstream of bridge")
bd = cross_section_by_description(g.cross_sections, "downstream of bridge")
print("BU", bu.rm if bu else None, "ineff", len(bu.ineff_blocks) if bu else 0)
print("BD", bd.rm if bd else None)
print("Q_steps", len(f.upstream_q_cfs), "peak_Q", max(f.upstream_q_cfs))
print("HWM_count", len(f.observed_hwm))

payload, _ = build_beaver_unsteady_inputs(p)
print("mapped_steps", payload.get("num_steps"), "dt", payload.get("dt"))
print("coupling_default_in_payload", payload.get("unsteady_structure_coupling_mode"))
print("bridge_deck_pts", len(payload.get("bridge_deck_stations", [[]])[0]))
print("has_BU", bool(payload.get("bridge_upstream_cross_sections")))
print("has_BD", bool(payload.get("bridge_downstream_cross_sections")))
