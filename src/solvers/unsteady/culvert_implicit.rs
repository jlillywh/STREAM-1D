//! Culvert internal-boundary residual for implicit Preissmann coupling (inlet + outlet control).

use crate::geometry::{CrossSection, GeometryTable};
use crate::solvers::bridge::unsteady_coupling::{wsel_metric_to_user, wsel_user_to_metric};
use crate::solvers::culvert::{
    culvert_implicit_eligible, solve_culvert, CulvertHeadwaterResidual,
};
use crate::solvers::steady::{solve_critical_depth_table, solve_step};
use crate::utils::UnitSystem;

use super::structure_coupling::build_unsteady_culvert_params;
use super::UnsteadyInputs;

/// HEC-RAS swell-head slope $S_h = h_l/(e\Delta x)$ on culvert / approach reach intervals (mode 2).
pub(crate) const CULVERT_SWELL_HEAD_ENABLED: bool = true;
/// Cap $|S_h|$ per interval (dimensionless) for Newton stability.
pub(crate) const CULVERT_SWELL_HEAD_MAX: f64 = 0.15;
/// Reach intervals upstream of the culvert that receive distributed $S_h$ (`0` = culvert cell only, per HEC Eq. 2-97).
pub(crate) const CULVERT_SWELL_HEAD_UPSTREAM_SPREAD: usize = 1;

/// Relaxation on approach-interval Jacobian rows (0, 1].
pub(crate) const CULVERT_APPROACH_JACOBIAN_OMEGA: f64 = 0.22;
/// Reach intervals upstream of the culvert that replace momentum with relaxed `solve_step` residuals.
pub(crate) const CULVERT_APPROACH_JACOBIAN_UPSTREAM_SPREAD: usize = 20;
/// First upstream cell uses post-step only; Jacobian starts at the second interval upstream.
pub(crate) const CULVERT_APPROACH_JACOBIAN_MIN_UPSTREAM_CELLS: usize = 2;
/// Extra friction on the departure reach when BD diverges from subcritical `solve_step`.
/// Not a documented HEC-RAS term (unlike swell-head Eqs 2-94–2-97); stability/parity knob only.
pub(crate) const CULVERT_DEPARTURE_TAILWATER_OMEGA: f64 = 0.30;
/// Relaxation for post-step approach backwater on intervals upstream of the culvert.
pub(crate) const CULVERT_APPROACH_POST_STEP_OMEGA: f64 = 0.55;
/// Max |ΔWSEL| per structure pass on each swept approach node (metric, ~0.5 ft).
pub(crate) const CULVERT_APPROACH_POST_STEP_MAX_DELTA_M: f64 = 0.15;
/// How many reach intervals upstream of the culvert receive chained `solve_step` post-step.
pub(crate) const CULVERT_APPROACH_UPSTREAM_SWEEP_CAP: usize = 64;
/// Post-step upstream sweeps after structure coupling (second pass uses 50% ω).
pub(crate) const CULVERT_APPROACH_UPSTREAM_SWEEP_PASSES: usize = 2;

/// Momentum-row replacement coefficients at node `i + 1` (first equation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ImplicitMomentumRow {
    pub m1: f64,
    pub m2: f64,
    pub m3: f64,
    pub m4: f64,
    pub rhs: f64,
}

fn face_perturbation_metric(raw_units: UnitSystem) -> f64 {
    if raw_units == UnitSystem::USCustomary {
        0.01 * crate::utils::FT_TO_M
    } else {
        0.003
    }
}

fn culvert_required_headwater_metric(
    inputs: &UnsteadyInputs,
    c_idx: usize,
    interval_i: usize,
    raw_units: UnitSystem,
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
    y_us_metric: f64,
    y_ds_metric: f64,
    require_implicit_eligible: bool,
) -> Option<f64> {
    let tw_user = wsel_metric_to_user(y_ds_metric, raw_units);
    let hw_user = wsel_metric_to_user(y_us_metric, raw_units);
    let mut y_local = y_metric.to_vec();
    y_local[interval_i] = y_us_metric;
    y_local[interval_i + 1] = y_ds_metric;
    let params = build_unsteady_culvert_params(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        &y_local,
        q_metric,
        tw_user,
        hw_user,
    );
    if require_implicit_eligible && !culvert_implicit_eligible(&params) {
        return None;
    }
    if params.q <= 1e-6 {
        return None;
    }
    let result = solve_culvert(&params);
    Some(wsel_user_to_metric(result.wsel, raw_units))
}

pub(crate) fn compute_culvert_structure_residual(
    inputs: &UnsteadyInputs,
    c_idx: usize,
    interval_i: usize,
    raw_units: UnitSystem,
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
) -> Option<CulvertHeadwaterResidual> {
    culvert_headwater_jacobian(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
        true,
    )
}

/// Rating-curve headwater Jacobian for swell-head forcing (allows roadway crest / overtopping splits).
fn culvert_headwater_jacobian(
    inputs: &UnsteadyInputs,
    c_idx: usize,
    interval_i: usize,
    raw_units: UnitSystem,
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
    require_implicit_eligible: bool,
) -> Option<CulvertHeadwaterResidual> {
    let y_us = y_metric[interval_i];
    let y_ds = y_metric[interval_i + 1];
    let q = q_metric[interval_i];
    let hw = culvert_required_headwater_metric(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
        y_us,
        y_ds,
        require_implicit_eligible,
    )?;
    let r = y_us - hw;
    let dy = face_perturbation_metric(raw_units);

    let hw_us_p = culvert_required_headwater_metric(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
        y_us + dy,
        y_ds,
        require_implicit_eligible,
    )
    .unwrap_or(hw);
    let hw_us_m = culvert_required_headwater_metric(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
        y_us - dy,
        y_ds,
        require_implicit_eligible,
    )
    .unwrap_or(hw);
    let dr_dy_us = 1.0 - (hw_us_p - hw_us_m) / (2.0 * dy);

    let hw_ds_p = culvert_required_headwater_metric(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
        y_us,
        y_ds + dy,
        require_implicit_eligible,
    )
    .unwrap_or(hw);
    let hw_ds_m = culvert_required_headwater_metric(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
        y_us,
        y_ds - dy,
        require_implicit_eligible,
    )
    .unwrap_or(hw);
    let dr_dy_ds = -(hw_ds_p - hw_ds_m) / (2.0 * dy);

    let dq = (q.abs() * 1e-4).max(1e-4);
    let mut q_local = q_metric.to_vec();
    q_local[interval_i] = q + dq;
    let hw_q_p = culvert_required_headwater_metric(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        &q_local,
        y_us,
        y_ds,
        require_implicit_eligible,
    )
    .unwrap_or(hw);
    q_local[interval_i] = (q - dq).max(1e-6);
    let hw_q_m = culvert_required_headwater_metric(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        &q_local,
        y_us,
        y_ds,
        require_implicit_eligible,
    )
    .unwrap_or(hw);
    let dr_dq = -(hw_q_p - hw_q_m) / (2.0 * dq);

    Some(CulvertHeadwaterResidual {
        r,
        dr_dy_us,
        dr_dy_ds,
        dr_dq,
    })
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
    let residual = compute_culvert_structure_residual(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
    )?;

    Some(ImplicitMomentumRow {
        m1: residual.dr_dy_us,
        m2: 0.0,
        m3: residual.dr_dy_ds,
        m4: 0.0,
        rhs: -residual.r,
    })
}

/// Monolithic Newton on reach + structure + approach backwater each time step (mode 3).
pub(crate) const CULVERT_APPROACH_MONOLITHIC_UPSTREAM_SPREAD: usize = 32;
/// Cell 1 (immediate approach) stays on standard Preissmann; monolithic rows start at cell 2.
pub(crate) const CULVERT_APPROACH_MONOLITHIC_MIN_UPSTREAM_CELLS: usize = 2;
/// Relaxation on monolithic approach rows (1.0 over-constrains vs unsteady continuity during ramps).
pub(crate) const CULVERT_APPROACH_MONOLITHIC_OMEGA: f64 = 0.75;

/// Reach intervals upstream of culvert for mode-3 monolithic `solve_step` rows (cells 1..=spread).
pub(crate) fn culvert_approach_monolithic_scope(
    interval_i: usize,
    culvert_intervals: &[(usize, usize)],
) -> Option<usize> {
    culvert_intervals.iter().find_map(|&(c_i, _)| {
        if interval_i >= c_i || c_i == 0 {
            return None;
        }
        let upstream_cells = c_i - interval_i;
        if upstream_cells < CULVERT_APPROACH_MONOLITHIC_MIN_UPSTREAM_CELLS
            || upstream_cells > CULVERT_APPROACH_MONOLITHIC_UPSTREAM_SPREAD
        {
            return None;
        }
        Some(upstream_cells)
    })
}

pub(crate) fn unsteady_coupling_is_implicit(mode: Option<i32>) -> bool {
    matches!(mode, Some(2) | Some(3) | Some(4))
}

pub(crate) fn unsteady_coupling_is_monolithic(mode: Option<i32>) -> bool {
    mode == Some(3)
}

/// When `|dQ/dt|` at the upstream BC is below this (m³/s²), approach Jacobian rows are suppressed (mode 2).
const CULVERT_APPROACH_JACOBIAN_DQDT_THRESHOLD: f64 = 1e-5;

/// True when upstream Q is changing (ramp / transient); false at constant Q.
pub(crate) fn culvert_approach_transient_active(
    q_up_next: f64,
    q_up_current: f64,
    dt: f64,
) -> bool {
    (q_up_next - q_up_current).abs() / dt.max(1e-6) > CULVERT_APPROACH_JACOBIAN_DQDT_THRESHOLD
}

/// True during Q transients (ramp); false at constant upstream Q so steady profiles are not over-constrained.
pub(crate) fn culvert_approach_jacobian_temporally_active(
    q_up_next: f64,
    q_up_current: f64,
    dt: f64,
) -> bool {
    CULVERT_APPROACH_JACOBIAN_OMEGA > 0.0
        && culvert_approach_transient_active(q_up_next, q_up_current, dt)
}

/// When `interval_i` lies in the culvert approach pool, returns 1-based upstream cell count
/// (`1` = immediate approach interval, `2` = next upstream, …).
pub(crate) fn culvert_approach_jacobian_scope(
    interval_i: usize,
    culvert_intervals: &[(usize, usize)],
) -> Option<usize> {
    if CULVERT_APPROACH_JACOBIAN_OMEGA <= 0.0 {
        return None;
    }
    culvert_intervals.iter().find_map(|&(c_i, _)| {
        if interval_i >= c_i || c_i == 0 {
            return None;
        }
        let upstream_cells = c_i - interval_i;
        if upstream_cells < CULVERT_APPROACH_JACOBIAN_MIN_UPSTREAM_CELLS
            || upstream_cells > CULVERT_APPROACH_JACOBIAN_UPSTREAM_SPREAD
        {
            return None;
        }
        Some(upstream_cells)
    })
}

/// Distance-weighted ω for an approach Jacobian row (`upstream_cells` from [`culvert_approach_jacobian_scope`]).
pub(crate) fn culvert_approach_jacobian_omega_for(upstream_cells: usize) -> f64 {
    let base = CULVERT_APPROACH_JACOBIAN_OMEGA;
    if base <= 0.0 {
        return 0.0;
    }
    let dist = upstream_cells.saturating_sub(CULVERT_APPROACH_JACOBIAN_MIN_UPSTREAM_CELLS) as f64;
    let cap = (CULVERT_APPROACH_JACOBIAN_UPSTREAM_SPREAD
        .saturating_sub(CULVERT_APPROACH_JACOBIAN_MIN_UPSTREAM_CELLS)
        .max(1)) as f64;
    let weight = (1.0 - 0.60 * dist / cap).max(0.25);
    base * weight
}

/// True when `interval_i` is the reach interval immediately downstream of a culvert (BD face upstream).
#[allow(dead_code)]
pub(crate) fn is_culvert_departure_interval(
    interval_i: usize,
    culvert_intervals: &[(usize, usize)],
    num_nodes: usize,
) -> bool {
    culvert_intervals.iter().any(|&(c_i, _)| {
        c_i + 1 == interval_i && interval_i + 1 < num_nodes
    })
}

/// Culvert cell and upstream reach intervals that receive swell-head forcing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CulvertSwellScope {
    pub culvert_interval_i: usize,
    pub c_idx: usize,
    /// `0` on the culvert interval, `1` on the immediate upstream cell, etc.
    pub upstream_cells: usize,
}

pub(crate) fn culvert_swell_head_scope(
    interval_i: usize,
    culvert_intervals: &[(usize, usize)],
) -> Option<CulvertSwellScope> {
    if !CULVERT_SWELL_HEAD_ENABLED {
        return None;
    }
    culvert_intervals
        .iter()
        .find_map(|&(c_i, c_idx)| {
            if interval_i > c_i {
                return None;
            }
            let upstream_cells = c_i - interval_i;
            if upstream_cells > CULVERT_SWELL_HEAD_UPSTREAM_SPREAD {
                return None;
            }
            Some(CulvertSwellScope {
                culvert_interval_i: c_i,
                c_idx,
                upstream_cells,
            })
        })
}

fn interval_dx(stations: &[f64], interval_i: usize) -> f64 {
    if interval_i + 1 >= stations.len() {
        return 1.0;
    }
    (stations[interval_i] - stations[interval_i + 1]).abs().max(1e-3)
}

fn cumulative_swell_length(
    stations: &[f64],
    interval_i: usize,
    culvert_interval_i: usize,
    fallback_dx: f64,
) -> f64 {
    if interval_i > culvert_interval_i {
        return fallback_dx.max(1e-3);
    }
    let mut total = 0.0;
    for j in interval_i..=culvert_interval_i {
        total += interval_dx(stations, j);
    }
    total.max(fallback_dx.max(1e-3))
}

/// HEC-RAS added force term: representative swell slope $S_h$ and partial derivatives.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CulvertSwellHeadDerivs {
    pub sh: f64,
    pub d_sh_dy_i: f64,
    pub d_sh_dy_ip: f64,
    pub d_sh_dq_i: f64,
    pub d_sh_dq_ip: f64,
}

fn culvert_loss_length_metric(
    inputs: &UnsteadyInputs,
    c_idx: usize,
    dx: f64,
    raw_units: UnitSystem,
) -> f64 {
    let user_len = inputs
        .culvert
        .culvert_lengths
        .as_ref()
        .and_then(|v| v.get(c_idx))
        .copied()
        .unwrap_or(0.0);
    let barrel_m = if user_len > 1e-6 {
        if raw_units == UnitSystem::USCustomary {
            user_len * crate::utils::FT_TO_M
        } else {
            user_len
        }
    } else {
        0.0
    };
    if barrel_m > 1e-3 {
        barrel_m.max(dx)
    } else {
        dx.max(1e-3)
    }
}

/// $S_h = h_l/L$ with $h_l = \mathrm{HW}(Q, TW) - TW$ from the culvert rating curve (HEC Eq. 2-94–2-97).
pub(crate) fn culvert_swell_head_at_interval(
    inputs: &UnsteadyInputs,
    interval_i: usize,
    culvert_intervals: &[(usize, usize)],
    raw_units: UnitSystem,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
    dx: f64,
) -> Option<CulvertSwellHeadDerivs> {
    if !CULVERT_SWELL_HEAD_ENABLED
        || !unsteady_coupling_is_implicit(inputs.unsteady_structure_coupling_mode)
        || unsteady_coupling_is_monolithic(inputs.unsteady_structure_coupling_mode)
    {
        return None;
    }

    let scope = culvert_swell_head_scope(interval_i, culvert_intervals)?;
    let culvert_interval_i = scope.culvert_interval_i;
    let c_idx = scope.c_idx;

    if culvert_interval_i + 1 >= y_metric.len() {
        return None;
    }

    let residual = culvert_headwater_jacobian(
        inputs,
        c_idx,
        culvert_interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
        false,
    )?;

    // When the HW residual is satisfied, swell on any approach/culvert cell double-counts head
    // loss and blows up far-upstream WSEL at constant Q. Suppress all swell cells in that case.
    if residual.r.abs() <= culvert_implicit_tolerance_metric(raw_units) {
        return None;
    }

    let y_us = y_metric[culvert_interval_i];
    let y_ds = y_metric[culvert_interval_i + 1];
    let hw = y_us - residual.r;
    let hl = (hw - y_ds).max(0.0);
    let loss_dx = if scope.upstream_cells == 0 {
        culvert_loss_length_metric(inputs, c_idx, dx, raw_units)
    } else {
        cumulative_swell_length(
            densified_stations,
            interval_i,
            culvert_interval_i,
            dx,
        )
    };
    let mut sh = hl / loss_dx;

    let dhw_dy_us = 1.0 - residual.dr_dy_us;
    let dhw_dy_ds = -residual.dr_dy_ds;
    let dhl_dy_us = dhw_dy_us;
    let dhl_dy_ds = dhw_dy_ds - 1.0;
    let dhl_dq = -residual.dr_dq;

    let inv_l = 1.0 / loss_dx;
    let (mut d_sh_dy_i, mut d_sh_dy_ip, mut d_sh_dq_i, mut d_sh_dq_ip) =
        if scope.upstream_cells == 0 {
            // Structure interval (HEC Eq. 2-97): both culvert faces participate.
            (
                inv_l * dhl_dy_us,
                inv_l * dhl_dy_ds,
                0.0,
                inv_l * dhl_dq,
            )
        } else if scope.upstream_cells == 1 {
            // Immediate upstream reach cell: downstream face is culvert US.
            (0.0, inv_l * dhl_dy_us, 0.0, inv_l * dhl_dq)
        } else {
            // Farther upstream: distributed $h_l$ with local RHS forcing only (stable Jacobian).
            (0.0, 0.0, 0.0, 0.0)
        };

    if sh > CULVERT_SWELL_HEAD_MAX {
        sh = CULVERT_SWELL_HEAD_MAX;
        d_sh_dy_i = 0.0;
        d_sh_dy_ip = 0.0;
        d_sh_dq_i = 0.0;
        d_sh_dq_ip = 0.0;
    } else if sh < -CULVERT_SWELL_HEAD_MAX {
        sh = -CULVERT_SWELL_HEAD_MAX;
        d_sh_dy_i = 0.0;
        d_sh_dy_ip = 0.0;
        d_sh_dq_i = 0.0;
        d_sh_dq_ip = 0.0;
    }

    Some(CulvertSwellHeadDerivs {
        sh,
        d_sh_dy_i,
        d_sh_dy_ip,
        d_sh_dq_i,
        d_sh_dq_ip,
    })
}

/// Departure-reach tailwater forcing: nudge BD toward subcritical `solve_step` without replacing momentum.
pub(crate) fn culvert_departure_tailwater_at_interval(
    interval_i: usize,
    culvert_intervals: &[(usize, usize)],
    raw_units: UnitSystem,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
    default_contraction: f64,
    default_expansion: f64,
    dx: f64,
) -> Option<CulvertSwellHeadDerivs> {
    let omega = CULVERT_DEPARTURE_TAILWATER_OMEGA;
    if omega <= 0.0 {
        return None;
    }
    if !culvert_intervals
        .iter()
        .any(|&(c_i, _)| c_i + 1 == interval_i && interval_i + 1 < y_metric.len())
    {
        return None;
    }
    let j = interval_i;
    let y_bd = y_metric[j];
    let y_ds = y_metric[j + 1];
    let q = q_metric[j];
    let implied = culvert_approach_wsel_metric(
        j,
        y_ds,
        q,
        densified_tables,
        densified_xs,
        densified_z_mins,
        densified_stations,
        default_contraction,
        default_expansion,
    )?;
    let mismatch = implied - y_bd;
    let inv_dx = 1.0 / dx.max(1e-3);
    let scale = omega * inv_dx;
    let mut sh = scale * mismatch;

    let dy = face_perturbation_metric(raw_units);
    let implied_ds_p = culvert_approach_wsel_metric(
        j,
        y_ds + dy,
        q,
        densified_tables,
        densified_xs,
        densified_z_mins,
        densified_stations,
        default_contraction,
        default_expansion,
    )
    .unwrap_or(implied);
    let implied_ds_m = culvert_approach_wsel_metric(
        j,
        y_ds - dy,
        q,
        densified_tables,
        densified_xs,
        densified_z_mins,
        densified_stations,
        default_contraction,
        default_expansion,
    )
    .unwrap_or(implied);
    let d_implied_dy_ds = (implied_ds_p - implied_ds_m) / (2.0 * dy);
    let mut d_sh_dy_i = -scale;
    let mut d_sh_dy_ip = scale * d_implied_dy_ds;

    if sh > CULVERT_SWELL_HEAD_MAX {
        sh = CULVERT_SWELL_HEAD_MAX;
        d_sh_dy_i = 0.0;
        d_sh_dy_ip = 0.0;
    } else if sh < -CULVERT_SWELL_HEAD_MAX {
        sh = -CULVERT_SWELL_HEAD_MAX;
        d_sh_dy_i = 0.0;
        d_sh_dy_ip = 0.0;
    }

    Some(CulvertSwellHeadDerivs {
        sh,
        d_sh_dy_i,
        d_sh_dy_ip,
        d_sh_dq_i: 0.0,
        d_sh_dq_ip: 0.0,
    })
}

fn culvert_approach_wsel_metric(
    approach_interval_i: usize,
    y_ds_metric: f64,
    q_metric: f64,
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    densified_stations: &[f64],
    default_contraction: f64,
    default_expansion: f64,
) -> Option<f64> {
    let i = approach_interval_i;
    if i + 1 >= densified_tables.len() {
        return None;
    }
    let length = (densified_stations[i] - densified_stations[i + 1]).abs();
    if length <= 0.0 {
        return None;
    }
    let yc_ds = solve_critical_depth_table(&densified_tables[i + 1], q_metric.abs());
    solve_step(
        &densified_tables[i + 1],
        Some(&densified_xs[i + 1]),
        None,
        y_ds_metric,
        &densified_tables[i],
        Some(&densified_xs[i]),
        None,
        densified_z_mins[i],
        yc_ds,
        q_metric.abs(),
        length,
        default_contraction,
        default_expansion,
        true,
        false,
        false,
    )
}

/// Relaxed standard-step residual on the approach interval: $R = \omega (y_\mathrm{us} - y_\mathrm{step}(y_\mathrm{ds}, Q))$.
pub(crate) fn try_culvert_approach_implicit_momentum_row(
    approach_interval_i: usize,
    raw_units: UnitSystem,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
    default_contraction: f64,
    default_expansion: f64,
    omega: f64,
) -> Option<ImplicitMomentumRow> {
    if approach_interval_i == 0 || omega <= 0.0 {
        return None;
    }
    let i = approach_interval_i;
    let y_us = y_metric[i];
    let y_ds = y_metric[i + 1];
    let q = q_metric[i];
    let implied = culvert_approach_wsel_metric(
        i,
        y_ds,
        q,
        densified_tables,
        densified_xs,
        densified_z_mins,
        densified_stations,
        default_contraction,
        default_expansion,
    )?;
    let r = omega * (y_us - implied);
    let dy = face_perturbation_metric(raw_units);

    let implied_ds_p = culvert_approach_wsel_metric(
        i,
        y_ds + dy,
        q,
        densified_tables,
        densified_xs,
        densified_z_mins,
        densified_stations,
        default_contraction,
        default_expansion,
    )
    .unwrap_or(implied);
    let implied_ds_m = culvert_approach_wsel_metric(
        i,
        y_ds - dy,
        q,
        densified_tables,
        densified_xs,
        densified_z_mins,
        densified_stations,
        default_contraction,
        default_expansion,
    )
    .unwrap_or(implied);
    let dr_dy_us = omega;
    let dr_dy_ds = -omega * (implied_ds_p - implied_ds_m) / (2.0 * dy);

    Some(ImplicitMomentumRow {
        m1: dr_dy_us,
        m2: 0.0,
        m3: dr_dy_ds,
        m4: 0.0,
        rhs: -r,
    })
}

/// Post-step: capped relaxed `solve_step` backwater on one approach interval.
fn apply_culvert_approach_interval_post_step(
    approach_interval_j: usize,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &mut [f64],
    q_metric: &[f64],
    default_contraction: f64,
    default_expansion: f64,
    omega: f64,
    max_delta_m: f64,
) {
    if approach_interval_j == 0 || omega <= 0.0 {
        return;
    }
    let j = approach_interval_j;
    let y_ds = y_metric[j + 1];
    let Some(implied) = culvert_approach_wsel_metric(
        j,
        y_ds,
        q_metric[j],
        densified_tables,
        densified_xs,
        densified_z_mins,
        densified_stations,
        default_contraction,
        default_expansion,
    ) else {
        return;
    };
    let y_old = y_metric[j];
    let blended = (1.0 - omega) * y_old + omega * implied;
    if blended > y_old {
        let raise = (blended - y_old).min(max_delta_m);
        y_metric[j] = y_old + raise;
    } else {
        let lower = (y_old - blended).min(max_delta_m);
        if lower > 0.0 {
            y_metric[j] = y_old - lower;
        }
    }
}

/// Post-step: capped relaxed `solve_step` on the interval immediately upstream of the culvert US face.
pub(crate) fn apply_culvert_approach_immediate_post_step(
    culvert_interval_i: usize,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &mut [f64],
    q_metric: &[f64],
    default_contraction: f64,
    default_expansion: f64,
    omega: f64,
) {
    if culvert_interval_i == 0 || omega <= 0.0 {
        return;
    }
    apply_culvert_approach_interval_post_step(
        culvert_interval_i - 1,
        densified_stations,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
        default_contraction,
        default_expansion,
        omega,
        CULVERT_APPROACH_POST_STEP_MAX_DELTA_M,
    );
}

/// Post-step: chained `solve_step` backwater from two intervals upstream of the culvert (capped per node).
pub(crate) fn apply_culvert_approach_post_step(
    culvert_interval_i: usize,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &mut [f64],
    q_metric: &[f64],
    default_contraction: f64,
    default_expansion: f64,
    omega: f64,
) {
    if culvert_interval_i <= 1 || omega <= 0.0 {
        return;
    }
    let last = culvert_interval_i
        .saturating_sub(CULVERT_APPROACH_UPSTREAM_SWEEP_CAP)
        .max(1);
    let mut first = culvert_interval_i - 2;
    if CULVERT_APPROACH_JACOBIAN_OMEGA > 0.0 {
        let above_jacobian = culvert_interval_i
            .saturating_sub(CULVERT_APPROACH_JACOBIAN_UPSTREAM_SPREAD)
            .saturating_sub(1);
        if first <= above_jacobian {
            return;
        }
        first = above_jacobian;
    }
    if last > first {
        return;
    }
    let cap = CULVERT_APPROACH_UPSTREAM_SWEEP_CAP.max(1) as f64;
    for pass in 0..CULVERT_APPROACH_UPSTREAM_SWEEP_PASSES {
        let pass_omega = omega * if pass == 0 { 1.0 } else { 0.50 };
        if pass_omega <= 0.0 {
            continue;
        }
        for j in (last..=first).rev() {
            let dist = culvert_interval_i - 2 - j;
            let weight = (1.0 - 0.70 * dist as f64 / cap).max(0.30);
            let omega_j = pass_omega * weight;
            let max_delta_m = if dist == 0 {
                CULVERT_APPROACH_POST_STEP_MAX_DELTA_M
            } else {
                CULVERT_APPROACH_POST_STEP_MAX_DELTA_M * 1.33
            };
            apply_culvert_approach_interval_post_step(
                j,
                densified_stations,
                densified_tables,
                densified_xs,
                densified_z_mins,
                y_metric,
                q_metric,
                default_contraction,
                default_expansion,
                omega_j,
                max_delta_m,
            );
        }
    }
}

fn culvert_implicit_tolerance_metric(raw_units: UnitSystem) -> f64 {
    if raw_units == UnitSystem::USCustomary {
        0.01 * crate::utils::FT_TO_M
    } else {
        0.003
    }
}

/// Skip explicit face overwrite when mode `2` already satisfied the structure residual.
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
    if !unsteady_coupling_is_implicit(inputs.unsteady_structure_coupling_mode) {
        return false;
    }
    let Some(residual) = compute_culvert_structure_residual(
        inputs,
        c_idx,
        interval_i,
        raw_units,
        densified_tables,
        densified_xs,
        densified_z_mins,
        y_metric,
        q_metric,
    ) else {
        return false;
    };
    residual.r.abs() <= culvert_implicit_tolerance_metric(raw_units)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn culvert_approach_jacobian_scope_covers_upstream_pool() {
        let culverts = [(25_usize, 0_usize)];
        assert_eq!(culvert_approach_jacobian_scope(24, &culverts), None);
        assert_eq!(culvert_approach_jacobian_scope(23, &culverts), Some(2));
        assert_eq!(culvert_approach_jacobian_scope(5, &culverts), Some(20));
        assert!(culvert_approach_jacobian_scope(0, &culverts).is_none());
        assert!(culvert_approach_jacobian_scope(25, &culverts).is_none());
    }

    #[test]
    fn culvert_approach_jacobian_omega_decays_with_distance() {
        let w2 = culvert_approach_jacobian_omega_for(2);
        let w12 = culvert_approach_jacobian_omega_for(12);
        let w20 = culvert_approach_jacobian_omega_for(20);
        assert!(w2 > w12);
        assert!(w12 > w20);
        assert!(w20 >= CULVERT_APPROACH_JACOBIAN_OMEGA * 0.25);
    }

    #[test]
    fn culvert_approach_jacobian_temporal_gate() {
        assert!(!culvert_approach_transient_active(10.0, 10.0, 60.0));
        assert!(culvert_approach_transient_active(11.0, 10.0, 60.0));
        assert!(!culvert_approach_jacobian_temporally_active(10.0, 10.0, 60.0));
        assert!(culvert_approach_jacobian_temporally_active(11.0, 10.0, 60.0));
    }
}
