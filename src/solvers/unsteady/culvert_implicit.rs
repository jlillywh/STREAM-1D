//! Culvert inlet-control residual for implicit Preissmann coupling (Phase 3).

use crate::geometry::{CrossSection, GeometryTable};
use crate::solvers::bridge::unsteady_coupling::wsel_metric_to_user;
use crate::solvers::culvert::{culvert_headwater_residual, culvert_implicit_inlet_eligible, solve_culvert};
use crate::utils::UnitSystem;

use super::structure_coupling::build_unsteady_culvert_params;
use super::UnsteadyInputs;

/// Momentum-row replacement coefficients at node `i + 1` (first equation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ImplicitMomentumRow {
    pub m1: f64,
    pub m2: f64,
    pub m3: f64,
    pub m4: f64,
    pub rhs: f64,
}

pub(crate) fn try_culvert_implicit_momentum_row(
    inputs: &UnsteadyInputs,
    raw_units: UnitSystem,
    c_idx: usize,
    interval_i: usize,
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
) -> Option<ImplicitMomentumRow> {
    let tw_user = wsel_metric_to_user(y_metric[interval_i + 1], raw_units);
    let hw_user = wsel_metric_to_user(y_metric[interval_i], raw_units);
    let params = build_unsteady_culvert_params(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
        tw_user,
        hw_user,
    );
    if !culvert_implicit_inlet_eligible(&params) {
        return None;
    }

    let residual = culvert_headwater_residual(
        y_metric[interval_i],
        y_metric[interval_i + 1],
        q_metric[interval_i],
        &params,
    )?;

    Some(ImplicitMomentumRow {
        m1: residual.dr_dy_us,
        m2: 0.0,
        m3: residual.dr_dy_ds,
        m4: 0.0,
        rhs: -residual.r,
    })
}

/// Skip explicit face overwrite only when mode `2` already satisfied the inlet residual.
pub(crate) fn culvert_implicit_post_step_satisfied(
    inputs: &UnsteadyInputs,
    c_idx: usize,
    interval_i: usize,
    raw_units: UnitSystem,
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
) -> bool {
    if inputs.unsteady_structure_coupling_mode != Some(2) {
        return false;
    }
    let tw_user = wsel_metric_to_user(y_metric[interval_i + 1], raw_units);
    let hw_user = wsel_metric_to_user(y_metric[interval_i], raw_units);
    let params = build_unsteady_culvert_params(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
        tw_user,
        hw_user,
    );
    let Some(residual) = culvert_headwater_residual(
        y_metric[interval_i],
        y_metric[interval_i + 1],
        q_metric[interval_i],
        &params,
    ) else {
        return false;
    };
    let tol = if raw_units == UnitSystem::USCustomary {
        0.01 * crate::utils::FT_TO_M
    } else {
        0.003
    };
    residual.r.abs() <= tol
}

pub(crate) fn culvert_implicit_diagnostics(
    inputs: &UnsteadyInputs,
    c_idx: usize,
    interval_i: usize,
    raw_units: UnitSystem,
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
) -> crate::solvers::culvert::CulvertSolveResult {
    let tw_user = wsel_metric_to_user(y_metric[interval_i + 1], raw_units);
    let hw_user = wsel_metric_to_user(y_metric[interval_i], raw_units);
    let params = build_unsteady_culvert_params(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
        tw_user,
        hw_user,
    );
    solve_culvert(&params)
}
