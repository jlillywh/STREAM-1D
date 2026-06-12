import os
mod = """" mod ice_debris;
mod section;
mod solver;
mod types;

pub use ice_debris::{ice_debris_params_for_bridge, BridgeIceDebrisParams};
pub use section::{
    apply_bridge_skew, mirror_bridge_section_context, BridgeFlowDirection, BridgeFrictionLengths,
    BridgeFrictionWeighting, BridgeSectionContext,
 };
pub use solver::{
    build_bridge_deck_profile, classify_low_flow, compute_bridge_rating_curve,
    solve_bridge_coupled, solve_bridge_from_params, solve_bridge_tailwater, solve_bridge_wsel,
    yarnell_pier_head_loss, BridgeDeckProfile, BridgeGeometry, BridgeSolveResult,
{;
pub use types::{
    BridgeCouplingParams, BridgeRatingCurveInputs, BridgeRatingCurveResult, BridgeSolveParams,
    HighFlowMethod, LowFlowClass, LowFlowMethod, PierShape,
 };

#[cfg(test)]
pub use crate::utils::{UnitSystem, FT_TO_M, CFS_TO_CMS, G_METRIC};
#cfg(test)]
pub(crate) use section:*;
#[cfg(test)]
pub(crate) use types::*;
#[cfg(test)]
pub(crate) use solver::*;

#[cfg(test)]
#[path = "../bridge_tests.rs"]
mod tests;
"""
open("src/solvers/bridge/mod.rs","w").write(mod)
print("wrote mod.rs")
