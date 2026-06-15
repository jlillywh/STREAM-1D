//! Quasi-steady particular + perturbation decomposition for unsteady routing (mode 4).
//!
//! Each step decomposes the stage as `y = y_qs(Q, TW) + η` where `y_qs` is the steady
//! gradually-varied profile (structures included). Before the Preissmann step the baseline
//! is re-anchored to the new quasi-steady profile while preserving the perturbation `η`;
//! at constant discharge the profile snaps to `y_qs` so operator-splitting drift cannot accumulate.

use crate::geometry::CrossSection;
use crate::solvers::steady::SteadyInputs;
use crate::utils::{UnitSystem, CFS_TO_CMS, FT_TO_M};

use super::culvert_implicit::culvert_approach_transient_active;
use super::UnsteadyInputs;

/// Build steady-model inputs matching the unsteady reach at the given discharge and tailwater.
pub(crate) fn steady_inputs_for_quasi_steady(
    inputs: &UnsteadyInputs,
    flow_rate_user: f64,
    downstream_wsel_user: Option<f64>,
) -> SteadyInputs {
    SteadyInputs {
        cross_sections: inputs.cross_sections.clone(),
        flow_rate: flow_rate_user,
        num_slices: inputs.num_slices,
        coeff_contraction: inputs.coeff_contraction,
        coeff_expansion: inputs.coeff_expansion,
        regime: 0,
        downstream_wsel: downstream_wsel_user,
        downstream_bc_type: inputs.downstream_bc_type,
        downstream_bc_slope: inputs.downstream_bc_slope,
        downstream_bc_rating_q: inputs.downstream_bc_rating_q.clone(),
        downstream_bc_rating_wsel: inputs.downstream_bc_rating_wsel.clone(),
        upstream_wsel: None,
        max_spacing: inputs.max_spacing,
        densify_reach_modifier_policy: inputs.densify_reach_modifier_policy,
        culvert_stations: inputs.culvert.culvert_stations.clone(),
        culvert_shape_types: inputs.culvert.culvert_shape_types.clone(),
        culvert_spans: inputs.culvert.culvert_spans.clone(),
        culvert_rises: inputs.culvert.culvert_rises.clone(),
        culvert_roughness_ns: inputs.culvert.culvert_roughness_ns.clone(),
        culvert_lengths: inputs.culvert.culvert_lengths.clone(),
        culvert_entrance_loss_coeffs: inputs.culvert.culvert_entrance_loss_coeffs.clone(),
        culvert_exit_loss_coeffs: inputs.culvert.culvert_exit_loss_coeffs.clone(),
        culvert_barrels: inputs.culvert.culvert_barrels.clone(),
        culvert_roughness_n_bottoms: inputs.culvert.culvert_roughness_n_bottoms.clone(),
        culvert_depth_bottom_ns: inputs.culvert.culvert_depth_bottom_ns.clone(),
        culvert_depth_blockeds: inputs.culvert.culvert_depth_blockeds.clone(),
        culvert_inlet_types: inputs.culvert.culvert_inlet_types.clone(),
        culvert_z_ups: inputs.culvert.culvert_z_ups.clone(),
        culvert_z_downs: inputs.culvert.culvert_z_downs.clone(),
        culvert_crest_elevs: inputs.culvert.culvert_crest_elevs.clone(),
        culvert_weir_coeffs: inputs.culvert.culvert_weir_coeffs.clone(),
        culvert_weir_lengths: inputs.culvert.culvert_weir_lengths.clone(),
        culvert_skew_angles: inputs.culvert.culvert_skew_angles.clone(),
        culvert_active_barrels: inputs.culvert.culvert_active_barrels.clone(),
        culvert_barrel_spans: inputs.culvert.culvert_barrel_spans.clone(),
        culvert_barrel_rises: inputs.culvert.culvert_barrel_rises.clone(),
        bridge_stations: inputs.bridge.bridge_stations.clone(),
        bridge_low_chords: inputs.bridge.bridge_low_chords.clone(),
        bridge_high_chords: inputs.bridge.bridge_high_chords.clone(),
        bridge_pier_widths: inputs.bridge.bridge_pier_widths.clone(),
        bridge_num_piers: inputs.bridge.bridge_num_piers.clone(),
        bridge_pier_shapes: inputs.bridge.bridge_pier_shapes.clone(),
        bridge_weir_coeffs: inputs.bridge.bridge_weir_coeffs.clone(),
        bridge_orifice_coeffs: inputs.bridge.bridge_orifice_coeffs.clone(),
        bridge_abutment_block_widths: inputs.bridge.bridge_abutment_block_widths.clone(),
        bridge_abutment_left_widths: inputs.bridge.bridge_abutment_left_widths.clone(),
        bridge_abutment_right_widths: inputs.bridge.bridge_abutment_right_widths.clone(),
        bridge_abutment_left_stations: inputs.bridge.bridge_abutment_left_stations.clone(),
        bridge_abutment_right_stations: inputs.bridge.bridge_abutment_right_stations.clone(),
        bridge_abutment_left_top_elevations: inputs
            .bridge
            .bridge_abutment_left_top_elevations
            .clone(),
        bridge_abutment_right_top_elevations: inputs
            .bridge
            .bridge_abutment_right_top_elevations
            .clone(),
        bridge_abutment_left_top_profile_stations: inputs
            .bridge
            .bridge_abutment_left_top_profile_stations
            .clone(),
        bridge_abutment_left_top_profile_elevations: inputs
            .bridge
            .bridge_abutment_left_top_profile_elevations
            .clone(),
        bridge_abutment_right_top_profile_stations: inputs
            .bridge
            .bridge_abutment_right_top_profile_stations
            .clone(),
        bridge_abutment_right_top_profile_elevations: inputs
            .bridge
            .bridge_abutment_right_top_profile_elevations
            .clone(),
        bridge_low_flow_methods: inputs.bridge.bridge_low_flow_methods.clone(),
        bridge_high_flow_methods: inputs.bridge.bridge_high_flow_methods.clone(),
        bridge_lengths: inputs.bridge.bridge_lengths.clone(),
        bridge_friction_weighting: inputs.bridge.bridge_friction_weighting.clone(),
        bridge_approach_friction_lengths: inputs.bridge.bridge_approach_friction_lengths.clone(),
        bridge_departure_friction_lengths: inputs.bridge.bridge_departure_friction_lengths.clone(),
        bridge_opening_blockage_factors: inputs.bridge.bridge_opening_blockage_factors.clone(),
        bridge_pier_debris_widths: inputs.bridge.bridge_pier_debris_widths.clone(),
        bridge_pier_debris_heights: inputs.bridge.bridge_pier_debris_heights.clone(),
        bridge_ice_thicknesses: inputs.bridge.bridge_ice_thicknesses.clone(),
        bridge_ice_modes: inputs.bridge.bridge_ice_modes.clone(),
        bridge_deck_ice_thicknesses: inputs.bridge.bridge_deck_ice_thicknesses.clone(),
        bridge_wspro_coeffs: inputs.bridge.bridge_wspro_coeffs.clone(),
        bridge_pressure_flow_coeffs_inlet: inputs
            .bridge
            .bridge_pressure_flow_coeffs_inlet
            .clone(),
        bridge_max_weir_submergence: inputs.bridge.bridge_max_weir_submergence.clone(),
        bridge_deck_stations: inputs.bridge.bridge_deck_stations.clone(),
        bridge_deck_low_elevations: inputs.bridge.bridge_deck_low_elevations.clone(),
        bridge_deck_high_elevations: inputs.bridge.bridge_deck_high_elevations.clone(),
        bridge_ineffective_left_stations: inputs.bridge.bridge_ineffective_left_stations.clone(),
        bridge_ineffective_left_elevations: inputs
            .bridge
            .bridge_ineffective_left_elevations
            .clone(),
        bridge_ineffective_right_stations: inputs.bridge.bridge_ineffective_right_stations.clone(),
        bridge_ineffective_right_elevations: inputs
            .bridge
            .bridge_ineffective_right_elevations
            .clone(),
        bridge_ineffective_left_stations_upstream: inputs
            .bridge
            .bridge_ineffective_left_stations_upstream
            .clone(),
        bridge_ineffective_left_elevations_upstream: inputs
            .bridge
            .bridge_ineffective_left_elevations_upstream
            .clone(),
        bridge_ineffective_right_stations_upstream: inputs
            .bridge
            .bridge_ineffective_right_stations_upstream
            .clone(),
        bridge_ineffective_right_elevations_upstream: inputs
            .bridge
            .bridge_ineffective_right_elevations_upstream
            .clone(),
        bridge_ineffective_left_stations_downstream: inputs
            .bridge
            .bridge_ineffective_left_stations_downstream
            .clone(),
        bridge_ineffective_left_elevations_downstream: inputs
            .bridge
            .bridge_ineffective_left_elevations_downstream
            .clone(),
        bridge_ineffective_right_stations_downstream: inputs
            .bridge
            .bridge_ineffective_right_stations_downstream
            .clone(),
        bridge_ineffective_right_elevations_downstream: inputs
            .bridge
            .bridge_ineffective_right_elevations_downstream
            .clone(),
        bridge_skew_angles: inputs.bridge.bridge_skew_angles.clone(),
        bridge_pier_stations: inputs.bridge.bridge_pier_stations.clone(),
        bridge_upstream_cross_sections: inputs.bridge.bridge_upstream_cross_sections.clone(),
        bridge_downstream_cross_sections: inputs.bridge.bridge_downstream_cross_sections.clone(),
        bridge_internal_cross_sections: inputs.bridge.bridge_internal_cross_sections.clone(),
        bridge_opening_reach_station_origins: inputs
            .bridge
            .bridge_opening_reach_station_origins
            .clone(),
        bridge_opening_anchor_modes: inputs.bridge.bridge_opening_anchor_modes.clone(),
        bridge_opening_anchor_reach_stations: inputs
            .bridge
            .bridge_opening_anchor_reach_stations
            .clone(),
        bridge_composed_embankment_blocked: inputs
            .bridge
            .bridge_composed_embankment_blocked
            .clone(),
        ..Default::default()
    }
}

fn downstream_wsel_user_for_quasi_steady(
    inputs: &UnsteadyInputs,
    downstream_wsel_metric: f64,
    raw_units: UnitSystem,
) -> Option<f64> {
    match inputs.downstream_bc_type.unwrap_or(0) {
        0 => {
            let w = if raw_units == UnitSystem::USCustomary {
                downstream_wsel_metric / FT_TO_M
            } else {
                downstream_wsel_metric
            };
            Some(w)
        }
        _ => None,
    }
}

fn flow_rate_user(q_metric: f64, raw_units: UnitSystem) -> f64 {
    if raw_units == UnitSystem::USCustomary {
        q_metric / CFS_TO_CMS
    } else {
        q_metric
    }
}

/// Linear interpolation of stage on sorted reach stations (descending upstream → downstream).
fn interpolate_wsel_at_station(stations: &[f64], wsel: &[f64], s: f64) -> f64 {
    let n = stations.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 || s >= stations[0] {
        return wsel[0];
    }
    if s <= stations[n - 1] {
        return wsel[n - 1];
    }
    for i in 0..n - 1 {
        let s0 = stations[i];
        let s1 = stations[i + 1];
        if s <= s0 && s >= s1 {
            let t = if (s0 - s1).abs() > 1e-9 {
                (s0 - s) / (s0 - s1)
            } else {
                0.0
            };
            return (1.0 - t) * wsel[i] + t * wsel[i + 1];
        }
    }
    wsel[n - 1]
}

/// Steady WSEL profile mapped onto the densified Preissmann grid (metric stages).
pub(crate) fn densified_quasi_steady_profile(
    inputs: &UnsteadyInputs,
    raw_units: UnitSystem,
    q_up_metric: f64,
    downstream_wsel_metric: f64,
    xs_stations_sorted: &[f64],
    original_mapping: &[usize],
    num_xs: usize,
    densified_stations: &[f64],
) -> Vec<f64> {
    let ds_wsel_user =
        downstream_wsel_user_for_quasi_steady(inputs, downstream_wsel_metric, raw_units);
    let steady_inputs = steady_inputs_for_quasi_steady(
        inputs,
        flow_rate_user(q_up_metric, raw_units),
        ds_wsel_user,
    );
    let steady = crate::solvers::steady::solve_steady(&steady_inputs);

    let mut sorted_wsel = vec![0.0; num_xs];
    for orig_idx in 0..num_xs {
        let w = steady.wsel[orig_idx];
        sorted_wsel[original_mapping[orig_idx]] = if raw_units == UnitSystem::USCustomary {
            w * FT_TO_M
        } else {
            w
        };
    }

    densified_stations
        .iter()
        .map(|&s| interpolate_wsel_at_station(xs_stations_sorted, &sorted_wsel, s))
        .collect()
}

/// Re-anchor `y := y_qs_new + (y - y_qs_prev)` preserving perturbation η across baseline updates.
pub(crate) fn reanchor_quasi_steady_baseline(
    y_metric: &mut [f64],
    y_qs_new: &[f64],
    y_qs_prev: &[f64],
) {
    debug_assert_eq!(y_metric.len(), y_qs_new.len());
    debug_assert_eq!(y_metric.len(), y_qs_prev.len());
    for k in 0..y_metric.len() {
        y_metric[k] = y_qs_new[k] + (y_metric[k] - y_qs_prev[k]);
    }
}

/// Scale for |dQ/dt| (m³/s²) in η-blend: small → track quasi-steady; large → keep Preissmann η.
pub(crate) const QUASI_STEADY_ETA_BLEND_DQDT_SCALE: f64 = 0.08;

/// Blend factor on perturbation η = y − y_qs (0 = snap to quasi-steady, 1 = keep solver η).
pub(crate) fn quasi_steady_eta_weight(
    q_up_next: f64,
    q_up_at_step_start: f64,
    dt: f64,
) -> f64 {
    let dqdt = (q_up_next - q_up_at_step_start).abs() / dt.max(1e-6);
    (dqdt / QUASI_STEADY_ETA_BLEND_DQDT_SCALE).min(1.0)
}

/// After structure coupling, relax η toward zero (track y_qs) with weight increasing as |dQ/dt| grows.
pub(crate) fn reconcile_quasi_steady_perturbation(
    y_metric: &mut [f64],
    y_qs: &[f64],
    q_up_next: f64,
    q_up_at_step_start: f64,
    dt: f64,
) {
    let eta_weight = quasi_steady_eta_weight(q_up_next, q_up_at_step_start, dt);
    if eta_weight >= 1.0 - 1e-12 {
        return;
    }
    for k in 0..y_metric.len() {
        y_metric[k] = y_qs[k] + eta_weight * (y_metric[k] - y_qs[k]);
    }
}

/// After a converged step at constant Q, snap the profile to the quasi-steady particular solution.
pub(crate) fn snap_to_quasi_steady_if_steady_flow(
    y_metric: &mut [f64],
    y_qs: &[f64],
    q_up_next: f64,
    q_up_at_step_start: f64,
    dt: f64,
) {
    if !culvert_approach_transient_active(q_up_next, q_up_at_step_start, dt) {
        y_metric.copy_from_slice(y_qs);
    } else {
        reconcile_quasi_steady_perturbation(y_metric, y_qs, q_up_next, q_up_at_step_start, dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_wsel_respects_descending_stations() {
        let stations = [1000.0, 500.0, 0.0];
        let wsel = [10.0, 8.0, 5.0];
        assert!((interpolate_wsel_at_station(&stations, &wsel, 750.0) - 9.0).abs() < 1e-9);
        assert!((interpolate_wsel_at_station(&stations, &wsel, 250.0) - 6.5).abs() < 1e-9);
    }

    #[test]
    fn reanchor_preserves_perturbation() {
        let mut y = vec![12.0, 10.0, 8.0];
        let y_new = vec![11.0, 9.5, 7.5];
        let y_prev = vec![10.0, 9.0, 7.0];
        reanchor_quasi_steady_baseline(&mut y, &y_new, &y_prev);
        assert!((y[0] - 13.0).abs() < 1e-9);
        assert!((y[1] - 10.5).abs() < 1e-9);
        assert!((y[2] - 8.5).abs() < 1e-9);
    }
}
