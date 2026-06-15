//! Preissmann θ-scheme Saint-Venant step with optional structure-interval hooks.

use crate::geometry::{flow_area_for_row, geometry_row_at_elevation, CrossSection, GeometryTable};
use crate::utils::{solve_block_tridiagonal, Mat2, UnitSystem, Vec2, G_METRIC};

use super::culvert_implicit::{
    culvert_approach_jacobian_omega_for, culvert_approach_jacobian_scope,
    culvert_approach_jacobian_temporally_active, culvert_approach_monolithic_scope,
    CULVERT_APPROACH_MONOLITHIC_OMEGA, culvert_departure_tailwater_at_interval,
    culvert_swell_head_at_interval, try_culvert_approach_implicit_momentum_row,
    ImplicitMomentumRow, try_culvert_implicit_momentum_row,
};
use super::structure_coupling;
use super::UnsteadyInputs;

/// How inline structures participate in the Preissmann step (API v33+).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsteadyStructureCouplingMode {
    /// One reach solve per step; structures corrected in post-step passes (default).
    PostStepOnly = 0,
    /// Reserved: reach–structure–reach outer iteration (not implemented; defer).
    ReachStructureReach = 1,
    /// Hybrid coupling: implicit Jacobian rows where eligible, explicit post-step fallback elsewhere.
    Implicit = 2,
    /// Convergent monolithic Newton: culvert HW + full approach backwater in Preissmann each step.
    MonolithicNewton = 3,
    /// Quasi-steady particular + perturbation: re-anchor to steady profile each step (mode 2 physics).
    QuasiSteadyParticular = 4,
}

/// Max outer Newton iterations per time step (mode 3).
pub(crate) const MONOLITHIC_NEWTON_MAX_ITER: usize = 20;
/// Convergence tolerance on max |residual| (metric, ~0.01 ft).
pub(crate) const MONOLITHIC_NEWTON_TOL_M: f64 = 0.003;
/// Stage update clamp per Newton iteration (mode 3).
pub(crate) const MONOLITHIC_NEWTON_DY_CLAMP_M: f64 = 0.5;
/// Discharge update clamp per Newton iteration (mode 3).
pub(crate) const MONOLITHIC_NEWTON_DQ_CLAMP: f64 = 15.0;

/// Per-step Preissmann diagnostics when structures are present.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PreissmannStepStats {
    /// Structure intervals that replaced the reach momentum row with an implicit residual.
    pub implicit_interval_count: u32,
    /// Outer Newton iterations (mode 3 only).
    pub newton_iterations: u32,
    /// Outer Newton converged within tolerance (mode 3).
    pub newton_converged: bool,
    /// Max |residual| at last Newton iteration (mode 3).
    pub newton_max_residual: f64,
    /// Max |residual| before the first Newton update (mode 3).
    pub newton_initial_residual: f64,
    /// Max |momentum-row residual| at last Newton iteration (mode 3).
    pub newton_momentum_residual: f64,
    /// Max |continuity-row residual| at last Newton iteration (mode 3).
    pub newton_continuity_residual: f64,
}

impl UnsteadyStructureCouplingMode {
    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => Self::ReachStructureReach,
            2 => Self::Implicit,
            3 => Self::MonolithicNewton,
            4 => Self::QuasiSteadyParticular,
            _ => Self::PostStepOnly,
        }
    }

    pub(crate) fn uses_implicit_rows(self) -> bool {
        matches!(
            self,
            Self::Implicit | Self::MonolithicNewton | Self::QuasiSteadyParticular
        )
    }

    pub(crate) fn uses_hybrid_friction_patches(self) -> bool {
        matches!(self, Self::Implicit | Self::QuasiSteadyParticular)
    }

    pub(crate) fn is_monolithic_newton(self) -> bool {
        self == Self::MonolithicNewton
    }

    pub(crate) fn is_quasi_steady_particular(self) -> bool {
        self == Self::QuasiSteadyParticular
    }

    /// Preissmann / post-step structure coupling uses hybrid implicit physics (mode 2).
    pub(crate) fn preissmann_coupling_mode(self) -> Self {
        if self.is_quasi_steady_particular() {
            Self::Implicit
        } else {
            self
        }
    }
}

/// Culvert or bridge occupying reach interval `i` (between nodes `i` and `i+1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureIntervalTag {
    Culvert(usize),
    Bridge(usize),
}

/// Inputs for one Preissmann time step.
pub struct PreissmannStepParams<'a> {
    pub tables: &'a [GeometryTable],
    pub xs_list: &'a [CrossSection],
    pub densified_stations: &'a [f64],
    pub z_mins: &'a [f64],
    pub y_current: &'a [f64],
    pub q_current: &'a [f64],
    pub dt: f64,
    pub q_up_next: f64,
    pub y_down_next: f64,
    pub theta: f64,
    pub c_contraction: f64,
    pub c_expansion: f64,
    pub structure_coupling_mode: UnsteadyStructureCouplingMode,
    pub culvert_intervals: &'a [(usize, usize)],
    pub bridge_intervals: &'a [(usize, usize)],
    pub unsteady_inputs: Option<&'a UnsteadyInputs>,
    pub raw_units: UnitSystem,
    /// When `structure_coupling_mode == Implicit`, records interval indices where the hook ran.
    #[cfg(test)]
    pub implicit_hook_probe: Option<*mut Vec<usize>>,
}

pub(crate) fn tag_at_interval(
    interval_i: usize,
    culvert_intervals: &[(usize, usize)],
    bridge_intervals: &[(usize, usize)],
) -> Option<StructureIntervalTag> {
    if let Some((_, c_idx)) = culvert_intervals.iter().find(|(i, _)| *i == interval_i) {
        return Some(StructureIntervalTag::Culvert(*c_idx));
    }
    if let Some((_, b_idx)) = bridge_intervals.iter().find(|(i, _)| *i == interval_i) {
        return Some(StructureIntervalTag::Bridge(*b_idx));
    }
    None
}

fn try_implicit_structure_momentum_row(
    params: &PreissmannStepParams<'_>,
    interval_i: usize,
    tag: Option<StructureIntervalTag>,
    #[cfg(test)] implicit_hook_probe: Option<*mut Vec<usize>>,
) -> Option<ImplicitMomentumRow> {
    if !params.structure_coupling_mode.uses_implicit_rows() {
        return None;
    }
    if let Some(tag) = tag {
        #[cfg(test)]
        if let Some(probe) = implicit_hook_probe.and_then(|p| unsafe { p.as_mut() }) {
            probe.push(interval_i);
        }
        let inputs = params.unsteady_inputs?;
        return match tag {
            StructureIntervalTag::Culvert(c_idx) => try_culvert_implicit_momentum_row(
                inputs,
                params.raw_units,
                c_idx,
                interval_i,
                params.tables,
                params.xs_list,
                params.z_mins,
                params.y_current,
                params.q_current,
            ),
            StructureIntervalTag::Bridge(b_idx) => {
                super::bridge_implicit::try_bridge_implicit_momentum_row(
                    inputs,
                    params.raw_units,
                    b_idx,
                    interval_i,
                    params.densified_stations,
                    params.tables,
                    params.xs_list,
                    params.z_mins,
                    params.y_current,
                    params.q_current,
                )
            }
        };
    }
    if params.structure_coupling_mode.is_monolithic_newton() {
        if culvert_approach_monolithic_scope(interval_i, params.culvert_intervals).is_some() {
            #[cfg(test)]
            if let Some(probe) = implicit_hook_probe.and_then(|p| unsafe { p.as_mut() }) {
                probe.push(interval_i);
            }
            return try_culvert_approach_implicit_momentum_row(
                interval_i,
                params.raw_units,
                params.densified_stations,
                params.tables,
                params.xs_list,
                params.z_mins,
                params.y_current,
                params.q_current,
                params.c_contraction,
                params.c_expansion,
                CULVERT_APPROACH_MONOLITHIC_OMEGA,
            );
        }
        return None;
    }
    if let Some(upstream_cells) =
        culvert_approach_jacobian_scope(interval_i, params.culvert_intervals)
    {
        let temporally_active = culvert_approach_jacobian_temporally_active(
            params.q_up_next,
            params.q_current[0],
            params.dt,
        );
        if temporally_active {
            let omega = culvert_approach_jacobian_omega_for(upstream_cells);
            if omega > 0.0 {
                #[cfg(test)]
                if let Some(probe) = implicit_hook_probe.and_then(|p| unsafe { p.as_mut() }) {
                    probe.push(interval_i);
                }
                return try_culvert_approach_implicit_momentum_row(
                    interval_i,
                    params.raw_units,
                    params.densified_stations,
                    params.tables,
                    params.xs_list,
                    params.z_mins,
                    params.y_current,
                    params.q_current,
                    params.c_contraction,
                    params.c_expansion,
                    omega,
                );
            }
        }
    }
    None
}

struct PreissmannLinearSystem {
    a: Vec<Mat2>,
    b: Vec<Mat2>,
    c: Vec<Mat2>,
    d: Vec<Vec2>,
    stats: PreissmannStepStats,
    max_rhs: f64,
}

fn max_abs_preissmann_rhs(d: &[Vec2]) -> f64 {
    max_abs_preissmann_rhs_split(d).0
}

fn max_abs_preissmann_rhs_split(d: &[Vec2]) -> (f64, f64, f64) {
    let mut max_m = 0.0_f64;
    let mut max_c = 0.0_f64;
    for v in d {
        max_m = max_m.max(v.v1.abs());
        max_c = max_c.max(v.v2.abs());
    }
    (max_m.max(max_c), max_m, max_c)
}

fn assemble_preissmann_linear_system(params: &PreissmannStepParams<'_>) -> Option<PreissmannLinearSystem> {
    let n = params.y_current.len();
    if n < 2 {
        return None;
    }

    let mut stats = PreissmannStepStats::default();
    let mut a = vec![Mat2::zero(); n];
    let mut b = vec![Mat2::zero(); n];
    let mut c = vec![Mat2::zero(); n];
    let mut d = vec![Vec2::zero(); n];

    let b0_11 = 0.0;
    let b0_12 = 1.0;
    let d0_1 = params.q_up_next - params.q_current[0];

    let bn_21 = 1.0;
    let bn_22 = 0.0;
    let dn_2 = params.y_down_next - params.y_current[n - 1];

    for i in 0..n - 1 {
        let dx = if i + 1 < params.densified_stations.len() {
            (params.densified_stations[i] - params.densified_stations[i + 1]).abs()
        } else {
            (params.xs_list[i].station - params.xs_list[i + 1].station).abs()
        };
        if dx <= 0.0 {
            return None;
        }

        let row_i = geometry_row_at_elevation(
            &params.tables[i],
            Some(&params.xs_list[i]),
            params.y_current[i],
            None,
            None,
        );
        let row_ip = geometry_row_at_elevation(
            &params.tables[i + 1],
            Some(&params.xs_list[i + 1]),
            params.y_current[i + 1],
            None,
            None,
        );

        let a_i = row_i.area.max(1e-6);
        let a_ip = row_ip.area.max(1e-6);
        let flow_a_i = flow_area_for_row(&row_i).max(1e-6);
        let flow_a_ip = flow_area_for_row(&row_ip).max(1e-6);
        let t_i = row_i.top_width.max(1e-6);
        let t_ip = row_ip.top_width.max(1e-6);

        let v_i = params.q_current[i] / flow_a_i;
        let v_ip = params.q_current[i + 1] / flow_a_ip;

        let k_i = row_i.conveyance.max(1e-6);
        let k_ip = row_ip.conveyance.max(1e-6);

        let dk_dy_i =
            structure_coupling::compute_dk_dy(&params.tables[i], &params.xs_list[i], params.y_current[i]);
        let dk_dy_ip = structure_coupling::compute_dk_dy(
            &params.tables[i + 1],
            &params.xs_list[i + 1],
            params.y_current[i + 1],
        );

        let q_avg = 0.5 * (params.q_current[i] + params.q_current[i + 1]);
        let k_avg = 0.5 * (k_i + k_ip);
        let k_avg_clamp = k_avg.max(0.01);
        let sf = (q_avg * q_avg.abs()) / (k_avg_clamp * k_avg_clamp);

        let d_sf_d_q = 2.0 * q_avg.abs() / (k_avg_clamp * k_avg_clamp);

        let z_min_i = params.z_mins[i];
        let z_min_ip = params.z_mins[i + 1];
        let depth_i = (params.y_current[i] - z_min_i).max(0.0);
        let depth_ip = (params.y_current[i + 1] - z_min_ip).max(0.0);

        let d_sf_dy_i = if depth_i < 0.1 {
            0.0
        } else {
            -q_avg * q_avg.abs() / (k_avg_clamp * k_avg_clamp * k_avg_clamp) * dk_dy_i
        };
        let d_sf_dy_ip = if depth_ip < 0.1 {
            0.0
        } else {
            -q_avg * q_avg.abs() / (k_avg_clamp * k_avg_clamp * k_avg_clamp) * dk_dy_ip
        };

        let mut sf_eff = sf;
        let mut d_sf_dy_i_eff = d_sf_dy_i;
        let mut d_sf_dy_ip_eff = d_sf_dy_ip;
        if params.structure_coupling_mode.uses_hybrid_friction_patches() {
            if let Some(inputs) = params.unsteady_inputs {
                if let Some(sh) = culvert_swell_head_at_interval(
                    inputs,
                    i,
                    params.culvert_intervals,
                    params.raw_units,
                    params.densified_stations,
                    params.tables,
                    params.xs_list,
                    params.z_mins,
                    params.y_current,
                    params.q_current,
                    dx,
                ) {
                    sf_eff += sh.sh;
                    d_sf_dy_i_eff += sh.d_sh_dy_i;
                    d_sf_dy_ip_eff += sh.d_sh_dy_ip;
                }
                if let Some(tw) = culvert_departure_tailwater_at_interval(
                    i,
                    params.culvert_intervals,
                    params.raw_units,
                    params.densified_stations,
                    params.tables,
                    params.xs_list,
                    params.z_mins,
                    params.y_current,
                    params.q_current,
                    params.c_contraction,
                    params.c_expansion,
                    dx,
                ) {
                    sf_eff += tw.sh;
                    d_sf_dy_i_eff += tw.d_sh_dy_i;
                    d_sf_dy_ip_eff += tw.d_sh_dy_ip;
                }
            }
        }

        let a_avg = (a_i * a_ip).sqrt();

        let c1 = t_i / (2.0 * params.dt);
        let c2 = -params.theta / dx;
        let c3 = t_ip / (2.0 * params.dt);
        let c4 = params.theta / dx;
        let ce = (params.q_current[i] - params.q_current[i + 1]) / dx;

        let d_hyd_i = a_i / t_i;
        let celerity_i = (G_METRIC * d_hyd_i).sqrt();
        let fr_i = if celerity_i > 1e-6 {
            v_i.abs() / celerity_i
        } else {
            0.0
        };
        let factor_i = if fr_i < 1.0 {
            (1.0 - fr_i * fr_i).max(0.0)
        } else {
            0.0
        };

        let d_hyd_ip = a_ip / t_ip;
        let celerity_ip = (G_METRIC * d_hyd_ip).sqrt();
        let fr_ip = if celerity_ip > 1e-6 {
            v_ip.abs() / celerity_ip
        } else {
            0.0
        };
        let factor_ip = if fr_ip < 1.0 {
            (1.0 - fr_ip * fr_ip).max(0.0)
        } else {
            0.0
        };

        let c_ec = if v_ip.abs() > v_i.abs() {
            params.c_contraction
        } else {
            params.c_expansion
        };
        let sign_v = (v_ip * v_ip - v_i * v_i).signum();
        let s_ce_force = a_avg * (c_ec / (2.0 * dx)) * (v_ip * v_ip - v_i * v_i).abs();

        let dfce_dyi = a_avg * (c_ec / dx) * sign_v * (v_i * v_i * t_i / flow_a_i);
        let dfce_dqi = -a_avg * (c_ec / dx) * sign_v * (v_i / flow_a_i);
        let dfce_dyip = -a_avg * (c_ec / dx) * sign_v * (v_ip * v_ip * t_ip / flow_a_ip);
        let dfce_dqip = a_avg * (c_ec / dx) * sign_v * (v_ip / flow_a_ip);

        let m1 = params.theta / dx * (v_i * v_i * t_i) * factor_i - G_METRIC * a_avg * params.theta / dx
            + G_METRIC * a_avg * params.theta * d_sf_dy_i_eff
            + params.theta * dfce_dyi;
        let m2 = (1.0 / (2.0 * params.dt)) - params.theta / dx * (2.0 * v_i) * factor_i
            + 0.5 * G_METRIC * a_avg * params.theta * d_sf_d_q
            + params.theta * dfce_dqi;
        let m3 = -params.theta / dx * (v_ip * v_ip * t_ip) * factor_ip
            + G_METRIC * a_avg * params.theta / dx
            + G_METRIC * a_avg * params.theta * d_sf_dy_ip_eff
            + params.theta * dfce_dyip;
        let m4 = (1.0 / (2.0 * params.dt)) + params.theta / dx * (2.0 * v_ip) * factor_ip
            + 0.5 * G_METRIC * a_avg * params.theta * d_sf_d_q
            + params.theta * dfce_dqip;

        let flux_i = (params.q_current[i] * params.q_current[i] / a_i) * factor_i;
        let flux_ip = (params.q_current[i + 1] * params.q_current[i + 1] / a_ip) * factor_ip;
        let me = (flux_i - flux_ip) / dx
            + G_METRIC * a_avg * (params.y_current[i] - params.y_current[i + 1]) / dx
            - G_METRIC * a_avg * sf_eff
            - s_ce_force;

        let tag = tag_at_interval(i, params.culvert_intervals, params.bridge_intervals);
        let implicit_row = try_implicit_structure_momentum_row(
            params,
            i,
            tag,
            #[cfg(test)]
            params.implicit_hook_probe,
        );

        if i == 0 {
            b[0] = Mat2 {
                m11: b0_11,
                m12: b0_12,
                m21: c1,
                m22: c2,
            };
            c[0] = Mat2 {
                m11: 0.0,
                m12: 0.0,
                m21: c3,
                m22: c4,
            };
            d[0] = Vec2 {
                v1: d0_1,
                v2: ce,
            };
        } else {
            b[i].m21 = c1;
            b[i].m22 = c2;
            c[i].m21 = c3;
            c[i].m22 = c4;
            d[i].v2 = ce;
        }

        if let Some(im) = implicit_row {
            stats.implicit_interval_count += 1;
            a[i + 1] = Mat2 {
                m11: im.m1,
                m12: im.m2,
                m21: 0.0,
                m22: 0.0,
            };
            b[i + 1].m11 = im.m3;
            b[i + 1].m12 = im.m4;
            d[i + 1].v1 = im.rhs;
        } else {
            a[i + 1] = Mat2 {
                m11: m1,
                m12: m2,
                m21: 0.0,
                m22: 0.0,
            };
            b[i + 1].m11 = m3;
            b[i + 1].m12 = m4;
            d[i + 1].v1 = me;
        }

        if i == n - 2 {
            b[n - 1].m21 = bn_21;
            b[n - 1].m22 = bn_22;
            d[n - 1].v2 = dn_2;
        }
    }

    let max_rhs = max_abs_preissmann_rhs(&d);
    let (_, max_m, max_c) = max_abs_preissmann_rhs_split(&d);
    stats.newton_momentum_residual = max_m;
    stats.newton_continuity_residual = max_c;
    Some(PreissmannLinearSystem {
        a,
        b,
        c,
        d,
        stats,
        max_rhs,
    })
}

fn apply_preissmann_delta(
    y: &mut [f64],
    q: &mut [f64],
    delta: &[Vec2],
    dy_clamp: f64,
    dq_clamp: f64,
) {
    for i in 0..y.len() {
        let dy = delta[i].v1.clamp(-dy_clamp, dy_clamp);
        let dq = delta[i].v2.clamp(-dq_clamp, dq_clamp);
        y[i] += dy;
        q[i] += dq;
    }
}

fn solve_preissmann_once(params: &PreissmannStepParams<'_>) -> Option<(Vec<f64>, Vec<f64>, PreissmannStepStats)> {
    let n = params.y_current.len();
    let system = assemble_preissmann_linear_system(params)?;
    let delta = solve_block_tridiagonal(&system.a, &system.b, &system.c, &system.d)?;

    let mut y_next = params.y_current.to_vec();
    let mut q_next = params.q_current.to_vec();
    let (dy_clamp, dq_clamp) = if params.structure_coupling_mode.is_monolithic_newton() {
        (MONOLITHIC_NEWTON_DY_CLAMP_M, MONOLITHIC_NEWTON_DQ_CLAMP)
    } else {
        (1.0, 25.0)
    };
    apply_preissmann_delta(&mut y_next, &mut q_next, &delta, dy_clamp, dq_clamp);

    q_next[0] = params.q_up_next;
    y_next[n - 1] = params.y_down_next;

    Some((y_next, q_next, system.stats))
}

fn solve_preissmann_monolithic_newton(
    params: &PreissmannStepParams<'_>,
) -> Option<(Vec<f64>, Vec<f64>, PreissmannStepStats)> {
    let n = params.y_current.len();
    let mut y = params.y_current.to_vec();
    let mut q = params.q_current.to_vec();
    let mut stats = PreissmannStepStats::default();

    for iter in 0..MONOLITHIC_NEWTON_MAX_ITER {
        let trial = PreissmannStepParams {
            tables: params.tables,
            xs_list: params.xs_list,
            densified_stations: params.densified_stations,
            z_mins: params.z_mins,
            y_current: &y,
            q_current: &q,
            dt: params.dt,
            q_up_next: params.q_up_next,
            y_down_next: params.y_down_next,
            theta: params.theta,
            c_contraction: params.c_contraction,
            c_expansion: params.c_expansion,
            structure_coupling_mode: params.structure_coupling_mode,
            culvert_intervals: params.culvert_intervals,
            bridge_intervals: params.bridge_intervals,
            unsteady_inputs: params.unsteady_inputs,
            raw_units: params.raw_units,
            #[cfg(test)]
            implicit_hook_probe: params.implicit_hook_probe,
        };
        let system = assemble_preissmann_linear_system(&trial)?;
        stats.implicit_interval_count = stats
            .implicit_interval_count
            .max(system.stats.implicit_interval_count);
        if iter == 0 {
            stats.newton_initial_residual = system.max_rhs;
        }
        stats.newton_max_residual = system.max_rhs;
        stats.newton_momentum_residual = system.stats.newton_momentum_residual;
        stats.newton_continuity_residual = system.stats.newton_continuity_residual;
        stats.newton_iterations = (iter + 1) as u32;

        if system.max_rhs <= MONOLITHIC_NEWTON_TOL_M {
            stats.newton_converged = true;
            break;
        }

        let delta = solve_block_tridiagonal(&system.a, &system.b, &system.c, &system.d)?;
        apply_preissmann_delta(
            &mut y,
            &mut q,
            &delta,
            MONOLITHIC_NEWTON_DY_CLAMP_M,
            MONOLITHIC_NEWTON_DQ_CLAMP,
        );
        q[0] = params.q_up_next;
        y[n - 1] = params.y_down_next;
    }

    Some((y, q, stats))
}

/// Solves a single Preissmann time step.
pub fn solve_preissmann_step(
    params: &PreissmannStepParams<'_>,
) -> Option<(Vec<f64>, Vec<f64>, PreissmannStepStats)> {
    if params.y_current.len() < 2 {
        return None;
    }
    if params.structure_coupling_mode.is_monolithic_newton() {
        solve_preissmann_monolithic_newton(params)
    } else {
        solve_preissmann_once(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::CrossSection;
    use crate::utils::UnitSystem;

    fn two_node_step(
        mode: UnsteadyStructureCouplingMode,
        culvert_intervals: &[(usize, usize)],
        bridge_intervals: &[(usize, usize)],
        probe: Option<*mut Vec<usize>>,
    ) -> (Vec<f64>, Vec<f64>) {
        let xs_us = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
            coeff_contraction: None,
            coeff_expansion: None,
        };
        let xs_ds = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            coeff_contraction: None,
            coeff_expansion: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let tables = vec![xs_us.generate_lookup_table(50), xs_ds.generate_lookup_table(50)];
        let z_mins = vec![0.0, 0.0];
        let y0 = vec![2.0, 2.0];
        let q0 = vec![15.0, 15.0];
        let densified_stations = vec![100.0, 0.0];
        let params = PreissmannStepParams {
            tables: &tables,
            xs_list: &[xs_us, xs_ds],
            densified_stations: &densified_stations,
            z_mins: &z_mins,
            y_current: &y0,
            q_current: &q0,
            dt: 60.0,
            q_up_next: 15.0,
            y_down_next: 2.0,
            theta: 0.6,
            c_contraction: 0.1,
            c_expansion: 0.3,
            structure_coupling_mode: mode,
            culvert_intervals,
            bridge_intervals,
            unsteady_inputs: None,
            raw_units: UnitSystem::Metric,
            implicit_hook_probe: probe,
        };
        let (y, q, _) = solve_preissmann_step(&params).expect("two-node step");
        (y, q)
    }

    #[test]
    fn implicit_hook_runs_only_on_tagged_intervals() {
        let mut probe = Vec::new();
        two_node_step(
            UnsteadyStructureCouplingMode::Implicit,
            &[(0, 0)],
            &[],
            Some(&mut probe as *mut Vec<usize>),
        );
        assert_eq!(probe, vec![0usize]);

        probe.clear();
        two_node_step(
            UnsteadyStructureCouplingMode::Implicit,
            &[],
            &[(0, 0)],
            Some(&mut probe as *mut Vec<usize>),
        );
        assert_eq!(probe, vec![0usize]);

        probe.clear();
        two_node_step(
            UnsteadyStructureCouplingMode::PostStepOnly,
            &[(0, 0)],
            &[],
            Some(&mut probe as *mut Vec<usize>),
        );
        assert!(probe.is_empty());
    }

    #[test]
    fn implicit_mode_stub_matches_post_step_mode() {
        let post = two_node_step(UnsteadyStructureCouplingMode::PostStepOnly, &[(0, 0)], &[], None);
        let implicit = two_node_step(UnsteadyStructureCouplingMode::Implicit, &[(0, 0)], &[], None);
        assert_eq!(post.0, implicit.0);
        assert_eq!(post.1, implicit.1);
    }

    #[test]
    fn tag_at_interval_prefers_culvert_when_both_present() {
        assert_eq!(
            tag_at_interval(1, &[(1, 0)], &[(1, 0)]),
            Some(StructureIntervalTag::Culvert(0))
        );
        assert_eq!(
            tag_at_interval(2, &[(1, 0)], &[(2, 0)]),
            Some(StructureIntervalTag::Bridge(0))
        );
        assert_eq!(tag_at_interval(3, &[(1, 0)], &[(2, 0)]), None);
    }
}
