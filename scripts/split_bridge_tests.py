#!/usr/bin/env python3
"""Split bridge_tests.rs into bridge/tests/ modules (R5)."""
import re
from pathlib import Path
from collections import defaultdict

ROOT = Path(__file__).resolve().parents[1]
src = (ROOT / "src/solvers/bridge_tests.rs").read_text()
lines = src.splitlines(keepends=True)

items = []
i = 0
while i < len(lines):
    line = lines[i]
    if line.startswith("#[test]"):
        block = [line]
        i += 1
        m = re.match(r"fn (\w+)", lines[i])
        name = m.group(1) if m else f"anon_{i}"
        depth = 0
        started = False
        while i < len(lines):
            block.append(lines[i])
            for ch in lines[i]:
                if ch == "{":
                    depth += 1
                    started = True
                elif ch == "}":
                    depth -= 1
            i += 1
            if started and depth == 0:
                break
        items.append(("test", name, "".join(block)))
        continue
    if line.startswith("fn ") and not re.match(r"fn test_", line):
        line = line.replace("fn ", "pub(crate) fn ", 1)
        m = re.match(r"fn (\w+)", line)
        name = m.group(1) if m else None
        block = []
        j = i - 1
        while j >= 0 and (lines[j].startswith("///") or lines[j].strip() == ""):
            j -= 1
        block.extend(lines[j + 1 : i])
        block.append(line)
        depth = 0
        started = False
        for ch in line:
            if ch == "{":
                depth += 1
                started = True
            elif ch == "}":
                depth -= 1
        i += 1
        while i < len(lines):
            if lines[i].startswith("#[test]") and depth == 0:
                break
            block.append(lines[i])
            for ch in lines[i]:
                if ch == "{":
                    depth += 1
                    started = True
                elif ch == "}":
                    depth -= 1
            i += 1
            if started and depth == 0:
                break
        items.append(("helper", name, "".join(block)))
        continue
    if not items or items[-1][0] != "header":
        items.append(("header", None, line))
    else:
        items[-1] = ("header", None, items[-1][2] + line)
    i += 1

merged = []
for kind, name, text in items:
    if merged and kind == "header" and merged[-1][0] == "header":
        merged[-1] = ("header", None, merged[-1][2] + text)
    else:
        merged.append((kind, name, text))
items = merged

helpers = []
tests = []
for kind, name, text in items:
    if kind == "header":
        continue
    elif kind == "helper":
        helpers.append(text)
    else:
        tests.append((name, text))


def category(name: str) -> str:
    n = name.lower()
    if "rating_curve" in n:
        return "rating"
    if any(
        k in n
        for k in (
            "sluice",
            "bradley",
            "weir",
            "pressure",
            "combined_high",
            "submergence",
            "high_flow",
            "tailwater",
            "deck_ice",
            "deck_vent",
            "segment_weir",
            "reconcile",
            "submerged_orifice",
            "partially_submerged",
            "scalar_weir",
            "explicit_high_flow",
        )
    ):
        return "high_flow"
    if any(
        k in n
        for k in (
            "obstructed",
            "abutment_reduces",
            "asymmetric_abutment",
            "footing",
            "nosing",
            "opening_blockage",
            "pier_debris",
            "ice_thickness",
            "hand_calc",
            "reach_cut_flow",
            "approach_overbank",
            "per_side_abutment_tops",
            "legacy_constant_pier_area",
            "profile_pier_obstructed",
            "tapered_pier_obstructed",
            "tapered_pier_skew",
            "submerged_footing",
        )
    ):
        return "opening"
    if any(
        k in n
        for k in (
            "solve_bridge_wsel",
            "friction_weighting",
            "ineffective",
            "guide_bank",
            "approach_narrowing",
            "deck_profile",
            "skew",
            "pier_stations",
            "bu_section",
            "bu_bd",
            "longer_bu_bd",
            "solve_bridge_headwater",
            "per_side_abutments_affect",
            "footing_nosing_exceed",
            "tapered_pier_exceed",
            "tapered_vs_mean_constant_solve",
            "tapered_pier_solve",
            "profile_pier_solve",
            "deck_profile_stations",
            "apply_bridge_skew",
            "approach_ineffective_raises",
        )
    ):
        return "coupling"
    return "low_flow"


by_cat = defaultdict(list)
for name, text in tests:
    by_cat[category(name)].append(text)

out_dir = ROOT / "src/solvers/bridge/tests"
out_dir.mkdir(parents=True, exist_ok=True)

common_header = """//! Shared bridge test fixtures and helpers.

use super::*;
use crate::geometry::{CrossSection, GuideBankToe, GuideBanks, IneffectiveFlowAreas};
use crate::solvers::pier_geometry::{
    resolve_pier_width_specs, PierAttachmentsUserInput, PierWidthSpec, PierWidthUserInput,
    ResolvedPier,
};

"""

(out_dir / "common.rs").write_text(common_header + "".join(helpers))

(out_dir / "mod.rs").write_text(
    """//! Bridge hydraulics unit tests (split by physics area).

use super::*;
pub(crate) use crate::geometry::{row_at_elevation, CrossSection, IneffectiveFlowAreas};
pub(crate) use crate::solvers::deck_vent_geometry::DeckVentUserInput;
pub(crate) use crate::solvers::pier_geometry::{resolve_pier_width_specs, PierWidthUserInput};

mod common;
mod coupling;
mod high_flow;
mod low_flow;
mod opening;
mod rating;

pub(crate) use common::*;
"""
)

for cat in ["opening", "low_flow", "high_flow", "coupling", "rating"]:
    body = "use super::*;\n\n" + "".join(by_cat[cat])
    (out_dir / f"{cat}.rs").write_text(body)

print(f"helpers: {len(helpers)}")
for cat in sorted(by_cat):
    print(f"  {cat}: {len(by_cat[cat])} tests")
print(f"total tests: {sum(len(v) for v in by_cat.values())}")
