//! Post-step culvert + bridge internal boundary coupling for unsteady routing.

use crate::geometry::{conveyance_derivative_at_elevation, geometry_row_at_elevation, CrossSection, GeometryTable};
use crate::utils::{UnitSystem, FT_TO_M, structure_in_reach_interval};

use super::UnsteadyInputs;

pub(crate) const CULVERT_HW_MAX_ITER: usize = 12;
const CULVERT_HW_TOL_FT: f64 = 0.001;
const CULVERT_HW_TOL_M: f64 = 0.0003;
pub(crate) const CULVERT_STEP_MAX_PASSES: usize = 5;
const CULVERT_STEP_TOL_FT: f64 = 0.01;
const CULVERT_STEP_TOL_M: f64 = 0.003;

/// Post-step coupling order when both culverts and bridges are present.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum StructureCouplingOrder {
    /// Merge culverts and bridges; process by reach interval downstream-first.
    CombinedDownstream = 0,
    /// Legacy: all culverts (downstream-first), then all bridges (downstream-first).
    CulvertsFirst = 1,
    /// All bridges (downstream-first), then all culverts (downstream-first).
    BridgesFirst = 2,
}

impl StructureCouplingOrder {
    pub(crate) fn from_i32(val: i32) -> Self {
        match val {
            1 => StructureCouplingOrder::CulvertsFirst,
            2 => StructureCouplingOrder::BridgesFirst,
            _ => StructureCouplingOrder::CombinedDownstream,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum StructureKind {
    Culvert,
    Bridge,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct CoupledStructure {
    pub interval_i: usize,
    pub kind: StructureKind,
    pub idx: usize,
}

/// Post-step structure coupling diagnostics for one time step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StructureCouplingStepDiagnostics {
    /// `true` when the post-step face-update loop converged within tolerance.
    pub converged: bool,
    /// Structure intervals that ran explicit face overwrite (not satisfied by implicit residual).
    pub explicit_fallback_count: u32,
}

pub(crate) struct StructureCouplingStepResults {
    pub culvert: Option<Vec<crate::solvers::culvert::CulvertSolveResult>>,
    pub bridge: Option<Vec<crate::solvers::bridge::BridgeSolveResult>>,
    pub diagnostics: Option<StructureCouplingStepDiagnostics>,
}

pub(crate) fn build_coupled_structure_order(
    culvert_intervals: &[(usize, usize)],
    bridge_intervals: &[(usize, usize)],
    order: StructureCouplingOrder,
) -> Vec<CoupledStructure> {
    let mut culverts: Vec<CoupledStructure> = culvert_intervals
        .iter()
        .map(|&(i, idx)| CoupledStructure {
            interval_i: i,
            kind: StructureKind::Culvert,
            idx,
        })
        .collect();
    let mut bridges: Vec<CoupledStructure> = bridge_intervals
        .iter()
        .map(|&(i, idx)| CoupledStructure {
            interval_i: i,
            kind: StructureKind::Bridge,
            idx,
        })
        .collect();

    match order {
        StructureCouplingOrder::CombinedDownstream => {
            culverts.append(&mut bridges);
            culverts.sort_by(|a, b| {
                b.interval_i
                    .cmp(&a.interval_i)
                    .then_with(|| match (a.kind, b.kind) {
                        (StructureKind::Culvert, StructureKind::Bridge) => std::cmp::Ordering::Less,
                        (StructureKind::Bridge, StructureKind::Culvert) => std::cmp::Ordering::Greater,
                        _ => std::cmp::Ordering::Equal,
                    })
            });
            culverts
        }
        StructureCouplingOrder::CulvertsFirst => {
            culverts.sort_by(|a, b| b.interval_i.cmp(&a.interval_i));
            bridges.sort_by(|a, b| b.interval_i.cmp(&a.interval_i));
            culverts.append(&mut bridges);
            culverts
        }
        StructureCouplingOrder::BridgesFirst => {
            bridges.sort_by(|a, b| b.interval_i.cmp(&a.interval_i));
            culverts.sort_by(|a, b| b.interval_i.cmp(&a.interval_i));
            bridges.append(&mut culverts);
            bridges
        }
    }
}

pub(crate) fn find_structure_intervals(
    structure_stations: &[f64],
    densified_stations: &[f64],
    raw_units: UnitSystem,
) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (s_idx, &s_st) in structure_stations.iter().enumerate() {
        let s_st_metric = if raw_units == UnitSystem::USCustomary {
            s_st * FT_TO_M
        } else {
            s_st
        };
        for i in 0..densified_stations.len().saturating_sub(1) {
            if structure_in_reach_interval(s_st_metric, densified_stations, i) {
                out.push((i, s_idx));
                break;
            }
        }
    }
    out
}

fn culvert_hw_tolerance(raw_units: UnitSystem) -> f64 {
    if raw_units == UnitSystem::USCustomary {
        CULVERT_HW_TOL_FT
    } else {
        CULVERT_HW_TOL_M
    }
}

fn culvert_step_tolerance(raw_units: UnitSystem) -> f64 {
    if raw_units == UnitSystem::USCustomary {
        CULVERT_STEP_TOL_FT
    } else {
        CULVERT_STEP_TOL_M
    }
}

pub(crate) fn build_unsteady_culvert_params(
    inputs: &UnsteadyInputs,
    c_idx: usize,
    i: usize,
    raw_units: UnitSystem,
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
    tw_wsel_user: f64,
    upstream_wsel_user: f64,
) -> crate::solvers::culvert::CulvertSolveParams {
    let c = &inputs.culvert;
    let shape_type = c.culvert_shape_types.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0);
    let span = c.culvert_spans.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(4.0);
    let rise = c.culvert_rises.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(4.0);
    let roughness_n = c.culvert_roughness_ns.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.013);
    let culv_len = c.culvert_lengths.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(100.0);
    let entrance_loss_coeff = c.culvert_entrance_loss_coeffs.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.5);
    let exit_loss_coeff = c.culvert_exit_loss_coeffs.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(1.0);
    let manning_n_bottom = c.culvert_roughness_n_bottoms.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(roughness_n);
    let depth_bottom_n = c.culvert_depth_bottom_ns.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
    let depth_blocked = c.culvert_depth_blockeds.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
    let inlet_type = c.culvert_inlet_types.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0);
    let crest_elev = c.culvert_crest_elevs.as_ref().and_then(|v| v.get(c_idx)).copied();
    let weir_coeff = c.culvert_weir_coeffs.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
    let weir_length = c.culvert_weir_lengths.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
    let num_barrels = c.culvert_barrels.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(1).max(1);
    let active_barrels = c.culvert_active_barrels.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0);
    let skew_deg = c.culvert_skew_angles.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.0);
    let barrel_spans = c.culvert_barrel_spans.as_ref().and_then(|v| v.get(c_idx)).cloned();
    let barrel_rises = c.culvert_barrel_rises.as_ref().and_then(|v| v.get(c_idx)).cloned();

    let bed_z_down = if raw_units == UnitSystem::USCustomary {
        densified_z_mins[i + 1] / FT_TO_M
    } else {
        densified_z_mins[i + 1]
    };
    let bed_z_up = if raw_units == UnitSystem::USCustomary {
        densified_z_mins[i] / FT_TO_M
    } else {
        densified_z_mins[i]
    };
    let z_down_user = c.culvert_z_downs.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(bed_z_down);
    let z_up_user = c.culvert_z_ups.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(bed_z_up);

    let q_user = if raw_units == UnitSystem::USCustomary {
        q_metric[i] / crate::utils::CFS_TO_CMS
    } else {
        q_metric[i]
    };

    let ds_row = geometry_row_at_elevation(
        &densified_tables[i + 1],
        Some(&densified_xs[i + 1]),
        y_metric[i + 1],
        None,
        None,
    );
    let ds_area_user = if raw_units == UnitSystem::USCustomary {
        ds_row.channel_area / (FT_TO_M * FT_TO_M)
    } else {
        ds_row.channel_area
    };
    let ds_velocity_user = q_user / ds_area_user.max(1e-9);

    let wsel_up_metric = if raw_units == UnitSystem::USCustomary {
        upstream_wsel_user * FT_TO_M
    } else {
        upstream_wsel_user
    };
    let us_row = geometry_row_at_elevation(
        &densified_tables[i],
        Some(&densified_xs[i]),
        wsel_up_metric,
        None,
        None,
    );
    let us_area_user = if raw_units == UnitSystem::USCustomary {
        us_row.channel_area / (FT_TO_M * FT_TO_M)
    } else {
        us_row.channel_area
    };
    let us_velocity_user = q_user / us_area_user.max(1e-9);

    crate::solvers::culvert::CulvertSolveParams {
        q: q_user,
        shape_type,
        inlet_type,
        span,
        rise,
        roughness_n,
        length: culv_len,
        entrance_loss_coeff,
        exit_loss_coeff,
        z_down: z_down_user,
        z_up: z_up_user,
        tw_wsel: tw_wsel_user,
        units: raw_units,
        manning_n_bottom,
        depth_bottom_n,
        depth_blocked,
        ds_velocity: ds_velocity_user,
        us_velocity: us_velocity_user,
        crest_elev,
        weir_coeff,
        weir_length,
        num_barrels,
        active_barrels,
        skew_deg,
        barrel_spans,
        barrel_rises,
    }
}

fn converge_culvert_headwater(
    inputs: &UnsteadyInputs,
    c_idx: usize,
    i: usize,
    raw_units: UnitSystem,
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    y_metric: &[f64],
    q_metric: &[f64],
    tw_wsel_user: f64,
    initial_hw: f64,
) -> crate::solvers::culvert::CulvertSolveResult {
    let tol = culvert_hw_tolerance(raw_units);
    let mut wsel_up_user = initial_hw;
    let mut result = crate::solvers::culvert::CulvertSolveResult {
        wsel: initial_hw,
        control_type: "outlet".to_string(),
        wsel_inlet: initial_hw,
        wsel_outlet: initial_hw,
        q_barrel: 0.0,
        q_weir: 0.0,
        barrel_depth: 0.0,
        barrel_velocity: 0.0,
        barrel_froude: 0.0,
    };

    for _ in 0..CULVERT_HW_MAX_ITER {
        let params = build_unsteady_culvert_params(
            inputs,
            c_idx,
            i,
            raw_units,
            densified_tables,
            densified_xs,
            densified_z_mins,
            y_metric,
            q_metric,
            tw_wsel_user,
            wsel_up_user,
        );
        result = crate::solvers::culvert::solve_culvert(&params);
        if (result.wsel - wsel_up_user).abs() <= tol {
            break;
        }
        wsel_up_user = result.wsel;
    }
    result
}

fn empty_culvert_step_results(num_culverts: usize) -> Vec<crate::solvers::culvert::CulvertSolveResult> {
    vec![
        crate::solvers::culvert::CulvertSolveResult {
            wsel: 0.0,
            control_type: String::new(),
            wsel_inlet: 0.0,
            wsel_outlet: 0.0,
            q_barrel: 0.0,
            q_weir: 0.0,
            barrel_depth: 0.0,
            barrel_velocity: 0.0,
            barrel_froude: 0.0,
        };
        num_culverts
    ]
}

fn empty_bridge_step_results(num_bridges: usize) -> Vec<crate::solvers::bridge::BridgeSolveResult> {
    vec![
        crate::solvers::bridge::BridgeSolveResult {
            wsel_up: 0.0,
            wsel_down: 0.0,
            head_loss: 0.0,
            flow_regime: String::new(),
        };
        num_bridges
    ]
}

pub(crate) fn apply_structure_internal_boundaries(
    inputs: &UnsteadyInputs,
    raw_units: UnitSystem,
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    densified_stations: &[f64],
    y_metric: &mut [f64],
    q_metric: &[f64],
    culvert_intervals: &[(usize, usize)],
    bridge_intervals: &[(usize, usize)],
) -> StructureCouplingStepResults {
    use crate::solvers::bridge::unsteady_coupling::{
        couple_bridge_interval, q_metric_to_user, wsel_metric_to_user, wsel_user_to_metric,
    };

    let num_culverts = inputs.culvert.culvert_stations.as_ref().map(|s| s.len()).unwrap_or(0);
    let num_bridges = inputs.bridge.bridge_stations.as_ref().map(|s| s.len()).unwrap_or(0);
    let order = StructureCouplingOrder::from_i32(inputs.structure_coupling_order.unwrap_or(0));
    let coupled = build_coupled_structure_order(culvert_intervals, bridge_intervals, order);

    if coupled.is_empty() {
        return StructureCouplingStepResults {
            culvert: None,
            bridge: None,
            diagnostics: None,
        };
    }

    let step_tol = culvert_step_tolerance(raw_units);
    let implicit_mode = inputs.unsteady_structure_coupling_mode == Some(2);
    let mut explicit_fallback_count = 0_u32;
    let mut converged = false;
    let dm = densified_tables.len();
    let mut culvert_results = if num_culverts > 0 {
        empty_culvert_step_results(num_culverts)
    } else {
        Vec::new()
    };
    let mut bridge_results = if num_bridges > 0 {
        empty_bridge_step_results(num_bridges)
    } else {
        Vec::new()
    };

    for pass in 0..CULVERT_STEP_MAX_PASSES {
        let mut max_delta = 0.0_f64;

        for structure in &coupled {
            let i = structure.interval_i;
            let tw_wsel_user = wsel_metric_to_user(y_metric[i + 1], raw_units);
            let prev_hw_user = wsel_metric_to_user(y_metric[i], raw_units);

            match structure.kind {
                StructureKind::Culvert => {
                    let implicit_satisfied = implicit_mode
                        && super::culvert_implicit::culvert_implicit_post_step_satisfied(
                            inputs,
                            structure.idx,
                            i,
                            raw_units,
                            densified_tables,
                            densified_xs,
                            densified_z_mins,
                            y_metric,
                            q_metric,
                        );
                    if implicit_satisfied {
                        culvert_results[structure.idx] =
                            super::culvert_implicit::culvert_implicit_diagnostics(
                                inputs,
                                structure.idx,
                                i,
                                raw_units,
                                densified_tables,
                                densified_xs,
                                densified_z_mins,
                                y_metric,
                                q_metric,
                            );
                        continue;
                    }
                    if pass == 0 {
                        explicit_fallback_count += 1;
                    }
                    let result = converge_culvert_headwater(
                        inputs,
                        structure.idx,
                        i,
                        raw_units,
                        densified_tables,
                        densified_xs,
                        densified_z_mins,
                        y_metric,
                        q_metric,
                        tw_wsel_user,
                        prev_hw_user,
                    );
                    max_delta = max_delta.max((result.wsel - prev_hw_user).abs());
                    y_metric[i] = if raw_units == UnitSystem::USCustomary {
                        result.wsel * FT_TO_M
                    } else {
                        result.wsel
                    };
                    culvert_results[structure.idx] = result;
                }
                StructureKind::Bridge => {
                    let b_idx = structure.idx;
                    let implicit_satisfied = implicit_mode
                        && super::bridge_implicit::bridge_implicit_post_step_satisfied(
                            inputs,
                            b_idx,
                            i,
                            raw_units,
                            densified_stations,
                            densified_tables,
                            densified_xs,
                            densified_z_mins,
                            y_metric,
                            q_metric,
                        );
                    if implicit_satisfied {
                        bridge_results[structure.idx] =
                            super::bridge_implicit::bridge_implicit_diagnostics(
                                inputs,
                                b_idx,
                                i,
                                raw_units,
                                densified_stations,
                                densified_tables,
                                densified_xs,
                                densified_z_mins,
                                y_metric,
                                q_metric,
                            );
                        continue;
                    }
                    if pass == 0 {
                        explicit_fallback_count += 1;
                    }
                    let b = &inputs.bridge;
                    let interval_length_m =
                        (densified_stations[i] - densified_stations[i + 1]).abs();
                    let coupling = super::bridge_coupling_for(inputs, b_idx);
                    let deck = super::bridge_deck_profile_for(inputs, b_idx, raw_units);
                    let num_slices = inputs.num_slices.unwrap_or(100);
                    let face_geo = super::bridge_face_geometry_for(
                        inputs,
                        b_idx,
                        i,
                        raw_units,
                        num_slices,
                        densified_stations,
                        densified_tables,
                        densified_xs,
                        densified_z_mins,
                        interval_length_m,
                    );
                    let weir_default = if raw_units == UnitSystem::USCustomary {
                        2.6
                    } else {
                        1.44
                    };
                    let ctx = crate::solvers::bridge::unsteady_coupling::BridgeUnsteadyIntervalContext {
                        scalars: crate::solvers::bridge::unsteady_coupling::BridgeUnsteadyScalars {
                            low_chord: b.bridge_low_chords.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0),
                            high_chord: b.bridge_high_chords.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0),
                            pier_width: b.bridge_pier_widths.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0),
                            num_piers: b.bridge_num_piers.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0),
                            pier_shape: b.bridge_pier_shapes.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0),
                            weir_coeff: b.bridge_weir_coeffs.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(weir_default),
                            orifice_coeff: b.bridge_orifice_coeffs.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.5),
                        },
                        coupling: &coupling,
                        deck: deck.as_ref(),
                        face_geo: &face_geo,
                        interval_length_m,
                    };
                    let q_user = q_metric_to_user(q_metric[i], raw_units);
                    let (result, update_face, delta, updated_user) = couple_bridge_interval(
                        &ctx,
                        i,
                        dm,
                        raw_units,
                        y_metric,
                        q_user,
                    );
                    max_delta = max_delta.max(delta);
                    y_metric[update_face] = wsel_user_to_metric(updated_user, raw_units);
                    bridge_results[structure.idx] = result;
                }
            }
        }

        if max_delta <= step_tol {
            converged = true;
            break;
        }
    }

    StructureCouplingStepResults {
        culvert: if num_culverts > 0 && !culvert_intervals.is_empty() {
            Some(culvert_results)
        } else {
            None
        },
        bridge: if num_bridges > 0 && !bridge_intervals.is_empty() {
            Some(bridge_results)
        } else {
            None
        },
        diagnostics: Some(StructureCouplingStepDiagnostics {
            converged,
            explicit_fallback_count,
        }),
    }
}

/// Helper to compute numerical derivative of conveyance K with respect to elevation y.
pub(crate) fn compute_dk_dy(table: &GeometryTable, xs: &CrossSection, elev: f64) -> f64 {
    conveyance_derivative_at_elevation(table, Some(xs), elev, None, None, 0.01)
}
