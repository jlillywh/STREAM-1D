#!/usr/bin/env python3
"""Split solver.rs into focused bridge modules."""
from pathlib import Path

ROOT = Path("src/solvers/bridge")
solver_path = ROOT / "solver.rs"
lines = solver_path.read_text(encoding="utf-8").splitlines(keepends=True)

def extract(ranges):
    out = []
    for start, end in ranges:
        out.extend(lines[start - 1 : end])
    return "".join(out)

MODULE_RANGES = {
    "geometry": [(20, 378), (2669, 3088)],
    "opening": [(380, 419), (427, 927)],
    "low_flow": [(929, 1295), (1585, 1817), (3090, 3115)],
    "high_flow": [(421, 425), (1296, 1570), (1819, 1965), (3117, 3224)],
    "headwater": [(1571, 1583)],
    "coupling": [(1967, 2124), (2555, 2667)],
    "rating": [(2126, 2553)],
}

IMPORTS = {
    "geometry": """use crate::geometry::{
    CrossSection, GeometryTable, GuideBanks, IneffectiveFlowAreas,
};
use crate::solvers::bridge_abutment::{resolve_abutments, BridgeAbutments};
use crate::solvers::deck_vent_geometry::resolve_deck_vents;
use crate::solvers::pier_geometry::{
    evenly_spaced_pier_stations, resolve_pier_width_specs, PierAttachmentsUserInput,
    PierWidthUserInput, ResolvedPier,
};
use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS};

use super::ice_debris::{clamp_opening_blockage_factor, BridgeIceDebrisParams};
use super::section::{apply_bridge_skew, BridgeSectionContext};
use super::types::{BridgeCouplingParams, LowFlowMethod, PierShape};

""",
    "opening": """use crate::geometry::{
    flow_area_for_row, row_at_elevation, CrossSection, GeometryRow, GeometryTable, GuideBanks,
    IneffectiveFlowAreas,
};
use crate::solvers::pier_geometry::{
    evenly_spaced_pier_stations, resolve_pier_width_specs, ResolvedPier,
};
use crate::utils::G_METRIC;

use super::geometry::{
    apply_opening_blockage, capped_ice_thickness_m, effective_z_bed_m, interpolate_profile,
    scale_base_area_for_ice, BridgeDeckProfile, BridgeGeometry,
};

""",
    "low_flow": """use crate::geometry::GeometryTable;
use crate::utils::G_METRIC;

use super::geometry::BridgeGeometry;
use super::opening::{
    bridge_energy_friction_loss, bridge_opening_friction_loss, obstructed_conveyance,
    obstructed_hydraulics, obstructed_opening_at_wsel, pier_drag_momentum_with_table,
    pier_submerged_area_at_wsel, specific_force, velocity_head, wspro_contraction_loss,
    yarnell_downstream_flow_area_m2, ObstructedHydraulics,
};
use super::types::{BridgeFlowRegimeKind, BridgeHeadwaterSolve, LowFlowClass, LowFlowMethod};

""",
    "high_flow": """use crate::geometry::GeometryTable;
use crate::solvers::deck_vent_geometry::total_deck_vent_discharge_m3s;
use crate::utils::G_METRIC;

use super::geometry::{
    effective_scalar_high_chord_m, effective_weir_length_m, net_opening_area_at_low_chord,
    profile_opening_area_factor, BridgeGeometry,
};
use super::low_flow::{
    solve_high_flow_energy, solve_high_flow_energy_fallback, upstream_energy_grade,
};
use super::types::{BridgeFlowRegimeKind, BridgeHeadwaterSolve, HighFlowMethod};

""",
    "headwater": """use crate::geometry::GeometryTable;

use super::geometry::BridgeGeometry;
use super::high_flow::solve_high_flow;
use super::low_flow::solve_low_flow;
use super::types::BridgeHeadwaterSolve;

""",
    "coupling": """use crate::geometry::GeometryTable;
use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS};

use super::geometry::{build_bridge_geometry, BridgeDeckProfile, BridgeGeometry};
use super::headwater::solve_bridge_headwater_metric;
use super::high_flow::{solve_high_flow_tailwater};
use super::low_flow::{classify_low_flow, solve_low_flow_tailwater};
use super::section::{
    bridge_q_to_metric_magnitude, mirror_bridge_section_context, BridgeFlowDirection,
    BridgeSectionContext,
};
use super::types::{BridgeCouplingParams, LowFlowClass};

""",
    "rating": """use crate::geometry::{
    CrossSection, GeometryTable, IneffectiveFlowAreas,
};
use crate::solvers::bridge_abutment::BridgeAbutmentUserInput;
use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS};

use super::coupling::{solve_bridge_coupled, BridgeSolveResult};
use super::geometry::{
    abutment_input_from_params, build_bridge_deck_profile, build_bridge_geometry,
    coupling_from_params, default_weir_coeff_for_units, geometry_tables_from_params,
    ineffective_downstream_from_params, ineffective_upstream_from_params, interval_length_metric,
    pier_attachments_user_to_metric, pier_width_user_to_metric, resolve_ice_debris_geometry_fields,
    BridgeDeckProfile, BridgeGeometry,
};
use super::section::BridgeSectionContext;
use super::types::{
    BridgeCouplingParams, BridgeRatingCurveInputs, BridgeRatingCurveResult, BridgeSolveParams,
};

""",
}

# Fix rating imports - abutment_input etc are IN rating module, not geometry
IMPORTS["rating"] = """use crate::geometry::{
    CrossSection, GeometryTable, IneffectiveFlowAreas,
};
use crate::solvers::bridge_abutment::{resolve_abutments, BridgeAbutmentUserInput, BridgeAbutments};
use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS};

use super::coupling::solve_bridge_coupled;
use super::geometry::{
    build_bridge_deck_profile, build_bridge_geometry, BridgeDeckProfile, BridgeGeometry,
};
use super::section::BridgeSectionContext;
use super::types::{
    BridgeCouplingParams, BridgeRatingCurveInputs, BridgeRatingCurveResult, BridgeSolveParams,
};

"""

# Fix low_flow - pier_drag is in low_flow module itself
IMPORTS["low_flow"] = """use crate::geometry::GeometryTable;
use crate::utils::G_METRIC;

use super::geometry::BridgeGeometry;
use super::opening::{
    bridge_energy_friction_loss, bridge_opening_friction_loss, obstructed_conveyance,
    obstructed_hydraulics, obstructed_opening_at_wsel, pier_submerged_area_at_wsel,
    specific_force, velocity_head, wspro_contraction_loss, yarnell_downstream_flow_area_m2,
    ObstructedHydraulics,
};
use super::types::{BridgeFlowRegimeKind, BridgeHeadwaterSolve, LowFlowClass, LowFlowMethod};

"""

# Fix high_flow - needs more from opening and low_flow
IMPORTS["high_flow"] = """use crate::geometry::GeometryTable;
use crate::solvers::deck_vent_geometry::total_deck_vent_discharge_m3s;
use crate::utils::G_METRIC;

use super::geometry::{
    effective_scalar_high_chord_m, effective_weir_length_m, net_opening_area_at_low_chord,
    profile_opening_area_factor, BridgeGeometry,
};
use super::low_flow::{
    solve_high_flow_energy, solve_high_flow_energy_fallback, upstream_energy_grade,
};
use super::opening::{
    combined_high_flow_discharge, deck_vent_pressure_discharge_m3s, pressure_flow_discharge,
    weir_flow_discharge,
};
use super::types::{BridgeFlowRegimeKind, BridgeHeadwaterSolve, HighFlowMethod};

"""

for name, ranges in MODULE_RANGES.items():
    body = extract(ranges)
    path = ROOT / f"{name}.rs"
    path.write_text(IMPORTS[name] + body, encoding="utf-8")
    print(f"wrote {path.name}: {len((IMPORTS[name]+body).splitlines())} lines")

MOD_RS = '''mod coupling;
mod geometry;
mod headwater;
mod high_flow;
mod ice_debris;
mod low_flow;
mod opening;
mod rating;
mod section;
mod types;

pub use ice_debris::{ice_debris_params_for_bridge, BridgeIceDebrisParams};
pub use section::{
    apply_bridge_skew, mirror_bridge_section_context, BridgeFlowDirection, BridgeFrictionLengths,
    BridgeFrictionWeighting, BridgeSectionContext,
};
pub use geometry::{
    build_bridge_deck_profile, BridgeDeckProfile, BridgeGeometry,
};
pub use low_flow::{classify_low_flow, yarnell_pier_head_loss};
pub use rating::{compute_bridge_rating_curve, solve_bridge_from_params};
pub use coupling::{
    solve_bridge_coupled, solve_bridge_tailwater, solve_bridge_wsel, BridgeSolveResult,
};

pub use types::{
    BridgeCouplingParams, BridgeRatingCurveInputs, BridgeRatingCurveResult, BridgeSolveParams,
    HighFlowMethod, LowFlowClass, LowFlowMethod, PierShape,
};

#[cfg(test)]
pub use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS, G_METRIC};
#[cfg(test)]
pub use crate::geometry::{flow_area_for_row, row_at_elevation, GeometryRow, GeometryTable};
#[cfg(test)]
pub use crate::solvers::bridge_abutment::{resolve_abutments, BridgeAbutmentUserInput, BridgeAbutments};
#[cfg(test)]
pub use crate::solvers::deck_vent_geometry::{resolve_deck_vents, total_deck_vent_discharge_m3s, ResolvedDeckVent};
#[cfg(test)]
pub use crate::solvers::pier_geometry::{evenly_spaced_pier_stations, ResolvedPier};
#[cfg(test)]
pub(crate) use types::*;
#[cfg(test)]
pub(crate) use geometry::*;
#[cfg(test)]
pub(crate) use opening::*;
#[cfg(test)]
pub(crate) use low_flow::*;
#[cfg(test)]
pub(crate) use high_flow::*;
#[cfg(test)]
pub(crate) use headwater::*;
#[cfg(test)]
pub(crate) use coupling::*;
#[cfg(test)]
pub(crate) use rating::*;

#[cfg(test)]
#[path = "../bridge_tests.rs"]
mod tests;
'''

(ROOT / "mod.rs").write_text(MOD_RS, encoding="utf-8")
print("updated mod.rs")
