//! Preissmann θ-scheme Saint-Venant step with optional structure-interval hooks.

use crate::geometry::{flow_area_for_row, geometry_row_at_elevation, CrossSection, GeometryTable};
use crate::utils::{solve_block_tridiagonal, Mat2, UnitSystem, Vec2, G_METRIC};

use super::culvert_implicit::{ImplicitMomentumRow, try_culvert_implicit_momentum_row};
use super::structure_coupling;
use super::UnsteadyInputs;

/// How inline structures participate in the Preissmann step (API v33).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsteadyStructureCouplingMode {
    /// One reach solve per step; structures corrected in post-step passes (default).
    PostStepOnly = 0,
    /// Reserved: reach–structure–reach outer iteration (Phase 5 optional).
    ReachStructureReach = 1,
    /// Structure residual rows in the Preissmann Jacobian (opt-in; Phase 3+ physics).
    Implicit = 2,
}

impl UnsteadyStructureCouplingMode {
    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => Self::ReachStructureReach,
            2 => Self::Implicit,
            _ => Self::PostStepOnly,
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
    if params.structure_coupling_mode != UnsteadyStructureCouplingMode::Implicit {
        return None;
    }
    let tag = tag?;
    #[cfg(test)]
    if let Some(probe) = implicit_hook_probe.and_then(|p| unsafe { p.as_mut() }) {
        probe.push(interval_i);
    }
    let inputs = params.unsteady_inputs?;
    match tag {
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
        StructureIntervalTag::Bridge(b_idx) => super::bridge_implicit::try_bridge_implicit_momentum_row(
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
        ),
    }
}

/// Solves a single Preissmann time step.
pub fn solve_preissmann_step(params: &PreissmannStepParams<'_>) -> Option<(Vec<f64>, Vec<f64>)> {
    let n = params.y_current.len();
    if n < 2 {
        return None;
    }

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
        let dx = params.xs_list[i].station - params.xs_list[i + 1].station;
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
            + G_METRIC * a_avg * params.theta * d_sf_dy_i
            + params.theta * dfce_dyi;
        let m2 = (1.0 / (2.0 * params.dt)) - params.theta / dx * (2.0 * v_i) * factor_i
            + 0.5 * G_METRIC * a_avg * params.theta * d_sf_d_q
            + params.theta * dfce_dqi;
        let m3 = -params.theta / dx * (v_ip * v_ip * t_ip) * factor_ip
            + G_METRIC * a_avg * params.theta / dx
            + G_METRIC * a_avg * params.theta * d_sf_dy_ip
            + params.theta * dfce_dyip;
        let m4 = (1.0 / (2.0 * params.dt)) + params.theta / dx * (2.0 * v_ip) * factor_ip
            + 0.5 * G_METRIC * a_avg * params.theta * d_sf_d_q
            + params.theta * dfce_dqip;

        let flux_i = (params.q_current[i] * params.q_current[i] / a_i) * factor_i;
        let flux_ip = (params.q_current[i + 1] * params.q_current[i + 1] / a_ip) * factor_ip;
        let me = (flux_i - flux_ip) / dx
            + G_METRIC * a_avg * (params.y_current[i] - params.y_current[i + 1]) / dx
            - G_METRIC * a_avg * sf
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

    let delta = solve_block_tridiagonal(&a, &b, &c, &d)?;

    let mut y_next = vec![0.0; n];
    let mut q_next = vec![0.0; n];
    for i in 0..n {
        let dy = delta[i].v1.clamp(-1.0, 1.0);
        let dq = delta[i].v2.clamp(-25.0, 25.0);

        y_next[i] = params.y_current[i] + dy;
        q_next[i] = params.q_current[i] + dq;
    }

    q_next[0] = params.q_up_next;
    y_next[n - 1] = params.y_down_next;

    Some((y_next, q_next))
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
        solve_preissmann_step(&params).expect("two-node step")
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
