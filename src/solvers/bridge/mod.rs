mod coupling;
mod geometry;
mod headwater;
mod high_flow;
mod ice_debris;
pub(crate) mod implicit;
mod low_flow;
mod opening;
mod rating;
pub mod reach_coupling;
mod section;
mod types;
pub(crate) mod unsteady_coupling;

pub use ice_debris::{ice_debris_params_for_bridge, BridgeIceDebrisParams};
pub(crate) use geometry::build_bridge_geometry;
pub(crate) use section::bridge_q_to_metric_magnitude;
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
mod tests;
