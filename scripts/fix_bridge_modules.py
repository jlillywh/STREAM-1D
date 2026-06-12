from pathlib import Path
import re

ROOT = Path("src/solvers/bridge")

def read(name):
    return (ROOT / name).read_text(encoding="utf-8")

def write(name, text):
    (ROOT / name).write_text(text, encoding="utf-8")

opening = read("opening.rs")
geom = read("geometry.rs")
low = read("low_flow.rs")
high = read("high_flow.rs")
head = read("headwater.rs")

# Extract blocks to move from opening -> geometry
def extract_fn(text, name):
    pat = rf"(///[^\n]*\n)*pub\(crate\) fn {name}\("
    m = re.search(pat, text)
    if not m:
        pat = rf"pub\(crate\) fn {name}\("
        m = re.search(pat, text)
    if not m:
        raise SystemExit(f"missing fn {name}")
    start = m.start()
    # include leading const comment block if const
    rest = text[m.start():]
    if rest.startswith("pub"):
        pass
    depth = 0
    i = m.start()
    while i < len(text):
        c = text[i]
        if c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0:
                end = i + 1
                while end < len(text) and text[end] in "\r\n":
                    end += 1
                return text[start:end], text[:start] + text[end:]
        i += 1
    raise SystemExit(f"unclosed fn {name}")

# const APPROACH_DEPARTURE_TABLE_SLICES
m = re.search(r"const APPROACH_DEPARTURE_TABLE_SLICES: usize = 50;\n\n", opening)
if m:
    const_block = m.group(0)
    opening = opening[:m.start()] + opening[m.end():]
else:
    const_block = "pub(crate) const APPROACH_DEPARTURE_TABLE_SLICES: usize = 50;\n\n"

bounds_block, opening = extract_fn(opening, "opening_station_bounds_from_deck")
internal_block, opening = extract_fn(opening, "internal_opening_friction_segments")

# Move reconcile + solve_low_flow_tailwater from low -> headwater
reconcile_block, low = extract_fn(low, "reconcile_low_flow_with_high_flow")
tail_block, low = extract_fn(low, "solve_low_flow_tailwater")

# Insert into geometry before pier_width (first fn in second chunk) or at end of first part
insert_at = geom.find("pub(crate) fn pier_width_user_to_metric")
geom = geom[:insert_at] + const_block + bounds_block + "\n" + internal_block + "\n" + geom[insert_at:]

# Fix opening_station_bounds_m to use local geometry fn - already calls opening_station_bounds_from_deck

head_extra = reconcile_block + "\n" + tail_block + "\n"
head_body = head.split("\n\n", 1)[1] if head.startswith("use ") else head
head = GEOMETRY_HEADWATER_IMPORTS = None

HEAD_IMPORTS = """use crate::geometry::GeometryTable;

use super::geometry::BridgeGeometry;
use super::high_flow::solve_high_flow;
use super::low_flow::{solve_low_flow, upstream_energy_grade};
use super::types::{BridgeFlowRegimeKind, BridgeHeadwaterSolve, LowFlowClass};

"""

head = HEAD_IMPORTS + head_body.strip() + "\n\n" + head_extra

GEOMETRY_IMPORTS = """use crate::geometry::{
    flow_area_for_row, row_at_elevation, CrossSection, GeometryTable, GuideBanks,
    IneffectiveFlowAreas,
};
use crate::solvers::bridge_abutment::{resolve_abutments, BridgeAbutments};
use crate::solvers::deck_vent_geometry::{resolve_deck_vents, ResolvedDeckVent};
use crate::solvers::pier_geometry::{
    evenly_spaced_pier_stations, resolve_pier_width_specs, PierAttachmentsUserInput,
    PierWidthUserInput, ResolvedPier,
};
use crate::utils::{UnitSystem, FT_TO_M};

use super::ice_debris::{clamp_opening_blockage_factor, BridgeIceDebrisParams};
use super::section::{
    apply_bridge_skew, BridgeFrictionLengths, BridgeFrictionWeighting, BridgeSectionContext,
};
use super::types::{BridgeCouplingParams, HighFlowMethod, LowFlowMethod, PierShape};

"""

# Replace geometry imports
if geom.startswith("use "):
    geom = GEOMETRY_IMPORTS + geom.split("\n\n", 1)[1]

OPENING_IMPORTS = """use crate::geometry::{
    flow_area_for_row, row_at_elevation, CrossSection, GeometryRow, GeometryTable, GuideBanks,
    IneffectiveFlowAreas,
};
use crate::solvers::pier_geometry::{
    evenly_spaced_pier_stations, resolve_pier_width_specs, ResolvedPier,
};
use crate::utils::G_METRIC;

use super::geometry::{
    apply_opening_blockage, capped_ice_thickness_m, effective_z_bed_m, interpolate_profile,
    opening_station_bounds_from_deck, scale_base_area_for_ice, BridgeDeckProfile, BridgeGeometry,
};

"""

if opening.startswith("use "):
    opening = OPENING_IMPORTS + opening.split("\n\n", 1)[1]

LOW_IMPORTS = """use crate::geometry::GeometryTable;
use crate::utils::G_METRIC;

use super::geometry::{
    effective_z_bed_m, gross_opening_area_at_low_chord, profile_opening_area_factor, BridgeGeometry,
};
use super::high_flow::solve_high_flow;
use super::opening::{
    active_resolved_piers, approach_departure_cut_modifiers_active, bridge_energy_friction_loss,
    gross_projected_opening_width_m, ineffective_for_side, lookup_row, obstructed_hydraulics,
    obstructed_opening_at_wsel, pier_floating_debris_obstruction_m2, pier_submerged_area_at_wsel,
    reach_cut_flow_area, section_xs, specific_force, velocity_head, wspro_contraction_loss,
    yarnell_downstream_flow_area_m2, ObstructedHydraulics,
};
use super::types::{BridgeFlowRegimeKind, BridgeHeadwaterSolve, LowFlowClass, LowFlowMethod, PierShape};

"""

if low.startswith("use "):
    low = LOW_IMPORTS + low.split("\n\n", 1)[1]

HIGH_IMPORTS = """use crate::geometry::GeometryTable;
use crate::solvers::deck_vent_geometry::total_deck_vent_discharge_m3s;
use crate::utils::G_METRIC;

use super::geometry::{
    effective_deck_crest_m, effective_scalar_high_chord_m, effective_weir_length_m, interpolate_profile,
    profile_opening_area_factor, BridgeGeometry,
};
use super::low_flow::{
    net_opening_area_at_low_chord, solve_high_flow_energy, solve_high_flow_energy_fallback,
    upstream_energy_grade,
};
use super::opening::{
    obstructed_hydraulics, opening_height_below_deck_m, velocity_head,
};
use super::types::{BridgeFlowRegimeKind, BridgeHeadwaterSolve, HighFlowMethod};

"""

if high.startswith("use "):
    high = HIGH_IMPORTS + high.split("\n\n", 1)[1]

# Remove bad import block lines from high if duplicated
high = re.sub(r"use super::opening::\{[^}]+\};\n", "", high, count=1)

COUPLING_IMPORTS = """use crate::geometry::GeometryTable;
use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS};

use super::geometry::{build_bridge_geometry, BridgeDeckProfile};
use super::headwater::{solve_bridge_headwater_metric, solve_low_flow_tailwater};
use super::high_flow::solve_high_flow_tailwater;
use super::low_flow::classify_low_flow;
use super::section::{
    bridge_q_to_metric_magnitude, mirror_bridge_section_context, BridgeFlowDirection,
    BridgeSectionContext,
};
use super::types::{BridgeCouplingParams, LowFlowClass};

"""

coupling = read("coupling.rs")
if coupling.startswith("use "):
    coupling = COUPLING_IMPORTS + coupling.split("\n\n", 1)[1]

RATING_IMPORTS = """use crate::geometry::{
    flow_area_for_row, row_at_elevation, CrossSection, GeometryTable, IneffectiveFlowAreas,
};
use crate::solvers::bridge_abutment::{resolve_abutments, BridgeAbutmentUserInput, BridgeAbutments};
use crate::utils::{UnitSystem, FT_TO_M};

use super::coupling::{solve_bridge_coupled, BridgeSolveResult};
use super::geometry::{build_bridge_deck_profile, build_bridge_geometry, BridgeDeckProfile};
use super::section::{apply_bridge_skew, hydraulic_hw_tw_reach, BridgeFlowDirection, BridgeSectionContext};
use super::types::{
    BridgeCouplingParams, BridgeRatingCurveInputs, BridgeRatingCurveResult, BridgeSolveParams,
    BridgeFrictionWeighting,
};

"""

rating = read("rating.rs")
if rating.startswith("use "):
    rating = RATING_IMPORTS + rating.split("\n\n", 1)[1]

# Fix rating - BridgeFrictionWeighting is in section not types
rating = rating.replace("BridgeFrictionWeighting,\n};", "")
rating = rating.replace(
    "BridgeSolveParams,\n    BridgeFrictionWeighting,\n};",
    "BridgeSolveParams,\n};",
)
rating = rating.replace(
    "use super::types::{\n    BridgeCouplingParams, BridgeRatingCurveInputs, BridgeRatingCurveResult, BridgeSolveParams,\n    BridgeFrictionWeighting,\n};",
    "use super::section::BridgeFrictionWeighting;\nuse super::types::{\n    BridgeCouplingParams, BridgeRatingCurveInputs, BridgeRatingCurveResult, BridgeSolveParams,\n};",
)

write("geometry.rs", geom)
write("opening.rs", opening)
write("low_flow.rs", low)
write("high_flow.rs", high)
write("headwater.rs", head)
write("coupling.rs", coupling)
write("rating.rs", rating)
print("patched modules")
