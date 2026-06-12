//! Unsteady post-step bridge interval coupling (face overwrite + HW iteration).

use crate::solvers::bridge_interior::BridgeFaceSolveGeometry;
use crate::utils::{UnitSystem, FT_TO_M};

use super::geometry::BridgeDeckProfile;
use super::types::BridgeCouplingParams;
use super::{solve_bridge_coupled, solve_bridge_tailwater, BridgeSolveResult};

pub(crate) const BRIDGE_HW_MAX_ITER: usize = 8;

pub(crate) fn bridge_hw_tolerance(raw_units: UnitSystem) -> f64 {
    if raw_units == UnitSystem::USCustomary {
        0.001
    } else {
        0.0003
    }
}

pub(crate) fn wsel_metric_to_user(y_metric: f64, raw_units: UnitSystem) -> f64 {
    if raw_units == UnitSystem::USCustomary {
        y_metric / FT_TO_M
    } else {
        y_metric
    }
}

pub(crate) fn wsel_user_to_metric(y_user: f64, raw_units: UnitSystem) -> f64 {
    if raw_units == UnitSystem::USCustomary {
        y_user * FT_TO_M
    } else {
        y_user
    }
}

pub(crate) fn q_metric_to_user(q_metric: f64, raw_units: UnitSystem) -> f64 {
    if raw_units == UnitSystem::USCustomary {
        q_metric / crate::utils::CFS_TO_CMS
    } else {
        q_metric
    }
}

/// Scalar bridge opening parameters for one unsteady interval solve.
pub(crate) struct BridgeUnsteadyScalars {
    pub low_chord: f64,
    pub high_chord: f64,
    pub pier_width: f64,
    pub num_piers: i32,
    pub pier_shape: i32,
    pub weir_coeff: f64,
    pub orifice_coeff: f64,
}

/// Resolved geometry + coupling for one bridge interval post-step pass.
pub(crate) struct BridgeUnsteadyIntervalContext<'a> {
    pub scalars: BridgeUnsteadyScalars,
    pub coupling: &'a BridgeCouplingParams,
    pub deck: Option<&'a BridgeDeckProfile>,
    pub face_geo: &'a BridgeFaceSolveGeometry,
    pub interval_length_m: f64,
}

/// Which reach faces supply tailwater vs receive solved headwater for post-step bridge coupling.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct BridgeIntervalCoupling {
    pub tw_face: usize,
    pub hw_face: usize,
    /// `Q < 0` and headwater sits on the reach downstream BC node — solve TW at BU, hold BD fixed.
    pub invert_tailwater: bool,
}

pub(crate) fn bridge_interval_coupling(i: usize, q_user: f64, dm: usize) -> BridgeIntervalCoupling {
    let reverses = q_user < -1e-12;
    if reverses {
        if i + 1 == dm - 1 {
            BridgeIntervalCoupling {
                tw_face: i,
                hw_face: i + 1,
                invert_tailwater: true,
            }
        } else {
            BridgeIntervalCoupling {
                tw_face: i,
                hw_face: i + 1,
                invert_tailwater: false,
            }
        }
    } else {
        BridgeIntervalCoupling {
            tw_face: i + 1,
            hw_face: i,
            invert_tailwater: false,
        }
    }
}

pub(crate) fn bridge_headwater_user(result: &BridgeSolveResult, q_user: f64) -> f64 {
    if q_user < -1e-12 {
        result.wsel_down
    } else {
        result.wsel_up
    }
}

fn solve_bridge_coupled_interval(
    q_user: f64,
    ctx: &BridgeUnsteadyIntervalContext<'_>,
    tw_wsel_user: f64,
    raw_units: UnitSystem,
) -> BridgeSolveResult {
    let s = &ctx.scalars;
    solve_bridge_coupled(
        q_user,
        s.low_chord,
        s.high_chord,
        s.pier_width,
        s.num_piers,
        s.pier_shape,
        s.weir_coeff,
        s.orifice_coeff,
        ctx.face_geo.z_down_user,
        ctx.face_geo.z_up_user,
        tw_wsel_user,
        raw_units,
        &ctx.face_geo.table_up,
        &ctx.face_geo.table_down,
        ctx.coupling,
        ctx.interval_length_m,
        ctx.deck,
        Some(&ctx.face_geo.sections),
    )
}

pub(crate) fn converge_bridge_headwater(
    ctx: &BridgeUnsteadyIntervalContext<'_>,
    q_user: f64,
    raw_units: UnitSystem,
    tw_wsel_user: f64,
    initial_hw: f64,
) -> BridgeSolveResult {
    let tol = bridge_hw_tolerance(raw_units);
    let mut hw_user = initial_hw;
    let mut result = solve_bridge_coupled_interval(q_user, ctx, tw_wsel_user, raw_units);

    for _ in 0..BRIDGE_HW_MAX_ITER {
        result = solve_bridge_coupled_interval(q_user, ctx, tw_wsel_user, raw_units);
        let hw_new = bridge_headwater_user(&result, q_user);
        if (hw_new - hw_user).abs() <= tol {
            break;
        }
        hw_user = hw_new;
    }
    result
}

/// Couple one bridge reach interval; returns (result, face index to update, delta, updated WSEL user).
pub(crate) fn couple_bridge_interval(
    ctx: &BridgeUnsteadyIntervalContext<'_>,
    i: usize,
    dm: usize,
    raw_units: UnitSystem,
    y_metric: &[f64],
    q_user: f64,
) -> (BridgeSolveResult, usize, f64, f64) {
    let coupling = bridge_interval_coupling(i, q_user, dm);
    let prev_tw_user = wsel_metric_to_user(y_metric[coupling.tw_face], raw_units);
    let prev_hw_user = wsel_metric_to_user(y_metric[coupling.hw_face], raw_units);

    if coupling.invert_tailwater {
        let s = &ctx.scalars;
        let tw_user = solve_bridge_tailwater(
            q_user,
            s.low_chord,
            s.high_chord,
            s.pier_width,
            s.num_piers,
            s.pier_shape,
            s.weir_coeff,
            s.orifice_coeff,
            ctx.face_geo.z_down_user,
            ctx.face_geo.z_up_user,
            prev_hw_user,
            raw_units,
            &ctx.face_geo.table_up,
            &ctx.face_geo.table_down,
            ctx.coupling,
            ctx.interval_length_m,
            ctx.deck,
            Some(&ctx.face_geo.sections),
        );
        let result = solve_bridge_coupled_interval(q_user, ctx, tw_user, raw_units);
        (
            result,
            coupling.tw_face,
            (tw_user - prev_tw_user).abs(),
            tw_user,
        )
    } else {
        let result = converge_bridge_headwater(ctx, q_user, raw_units, prev_tw_user, prev_hw_user);
        let hw_user = bridge_headwater_user(&result, q_user);
        (
            result,
            coupling.hw_face,
            (hw_user - prev_hw_user).abs(),
            hw_user,
        )
    }
}
