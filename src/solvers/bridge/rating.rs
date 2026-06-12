use crate::geometry::{
    CrossSection, GeometryTable, IneffectiveFlowAreas,
};
use crate::solvers::bridge_abutment::BridgeAbutmentUserInput;
use crate::utils::{UnitSystem, FT_TO_M};

use super::ice_debris::{clamp_opening_blockage_factor, BridgeIceDebrisParams};

use super::coupling::{solve_bridge_coupled, BridgeSolveResult};
use super::geometry::build_bridge_deck_profile;
use super::section::{
    hydraulic_hw_tw_reach, BridgeFlowDirection, BridgeFrictionWeighting,
    BridgeSectionContext,
};
use super::types::{
    BridgeCouplingParams, BridgeRatingCurveInputs, BridgeRatingCurveResult, BridgeSolveParams,
};

pub(crate) fn default_weir_coeff_for_units(units: UnitSystem) -> f64 {
    if units == UnitSystem::USCustomary {
        2.6
    } else {
        1.44
    }
}

pub(crate) fn rectangular_channel_cross_section(
    width: f64,
    z_bed: f64,
    manning_n: f64,
    units: UnitSystem,
) -> CrossSection {
    CrossSection {
        station: 0.0,
        x: vec![0.0, 0.0, width, width],
        y: vec![z_bed + 10.0, z_bed, z_bed, z_bed + 10.0],
        n_stations: vec![0.0],
        n_values: vec![manning_n.max(0.01)],
        unit_system: units,
        is_overbank: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
    guide_banks: None,
    }
}

pub(crate) fn abutment_input_from_params(params: &BridgeSolveParams) -> BridgeAbutmentUserInput {
    BridgeAbutmentUserInput {
        legacy_total_width: params.abutment_block_width,
        left_width: params.abutment_left_width,
        right_width: params.abutment_right_width,
        left_station: params.abutment_left_station,
        right_station: params.abutment_right_station,
        left_top_elevation: params.abutment_left_top_elevation,
        right_top_elevation: params.abutment_right_top_elevation,
        left_top_profile_stations: params.abutment_left_top_profile_stations.clone(),
        left_top_profile_elevations: params.abutment_left_top_profile_elevations.clone(),
        right_top_profile_stations: params.abutment_right_top_profile_stations.clone(),
        right_top_profile_elevations: params.abutment_right_top_profile_elevations.clone(),
    }
}

pub(crate) fn coupling_from_params(params: &BridgeSolveParams) -> BridgeCouplingParams {
    BridgeCouplingParams {
        abutment: abutment_input_from_params(params),
        low_flow_method: params.low_flow_method,
        high_flow_method: params.high_flow_method,
        length: params.length,
        wspro_coeff: params.wspro_coeff,
        coeff_contraction: params.coeff_contraction,
        coeff_expansion: params.coeff_expansion,
        pressure_coeff_inlet: params.pressure_coeff_inlet,
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: params.max_weir_submergence,
        friction_weighting: BridgeFrictionWeighting::from_i32(params.friction_weighting),
        approach_friction_length: params.approach_friction_length,
        departure_friction_length: params.departure_friction_length,
        ice_debris: BridgeIceDebrisParams {
            opening_blockage_factor: clamp_opening_blockage_factor(params.opening_blockage_factor),
            ice_thickness: params.ice_thickness,
            ice_mode: params.ice_mode.clamp(0, 2) as u8,
            deck_ice_thickness: params.deck_ice_thickness,
            pier_debris_widths: params.pier_debris_widths.clone().unwrap_or_default(),
            pier_debris_heights: params.pier_debris_heights.clone().unwrap_or_default(),
        },
    }
}

pub(crate) fn vec_or_scalar(values: Option<&Vec<f64>>, scalar: Option<f64>) -> Vec<f64> {
    if let Some(v) = values {
        if !v.is_empty() {
            return v.clone();
        }
    }
    scalar.into_iter().collect()
}

pub(crate) fn ineffective_face_blocks(
    face_stations: Option<&Vec<f64>>,
    face_elevations: Option<&Vec<f64>>,
    face_station: Option<f64>,
    face_elevation: Option<f64>,
    legacy_stations: Option<&Vec<f64>>,
    legacy_elevations: Option<&Vec<f64>>,
    legacy_station: Option<f64>,
    legacy_elevation: Option<f64>,
) -> (Vec<f64>, Vec<f64>) {
    let stations = {
        let us = vec_or_scalar(face_stations, face_station);
        if !us.is_empty() {
            us
        } else {
            vec_or_scalar(legacy_stations, legacy_station)
        }
    };
    let elevations = {
        let ue = vec_or_scalar(face_elevations, face_elevation);
        if !ue.is_empty() {
            ue
        } else {
            vec_or_scalar(legacy_elevations, legacy_elevation)
        }
    };
    (stations, elevations)
}

pub(crate) fn ineffective_upstream_from_params(params: &BridgeSolveParams) -> Option<IneffectiveFlowAreas> {
    let (left_s, left_e) = ineffective_face_blocks(
        params.ineffective_left_stations_upstream.as_ref(),
        params.ineffective_left_elevations_upstream.as_ref(),
        params.ineffective_left_station_upstream,
        params.ineffective_left_elevation_upstream,
        params.ineffective_left_stations.as_ref(),
        params.ineffective_left_elevations.as_ref(),
        params.ineffective_left_station,
        params.ineffective_left_elevation,
    );
    let (right_s, right_e) = ineffective_face_blocks(
        params.ineffective_right_stations_upstream.as_ref(),
        params.ineffective_right_elevations_upstream.as_ref(),
        params.ineffective_right_station_upstream,
        params.ineffective_right_elevation_upstream,
        params.ineffective_right_stations.as_ref(),
        params.ineffective_right_elevations.as_ref(),
        params.ineffective_right_station,
        params.ineffective_right_elevation,
    );
    IneffectiveFlowAreas::from_block_pairs(&left_s, &left_e, &right_s, &right_e)
}

pub(crate) fn ineffective_downstream_from_params(params: &BridgeSolveParams) -> Option<IneffectiveFlowAreas> {
    let (left_s, left_e) = ineffective_face_blocks(
        params.ineffective_left_stations_downstream.as_ref(),
        params.ineffective_left_elevations_downstream.as_ref(),
        params.ineffective_left_station_downstream,
        params.ineffective_left_elevation_downstream,
        params.ineffective_left_stations.as_ref(),
        params.ineffective_left_elevations.as_ref(),
        params.ineffective_left_station,
        params.ineffective_left_elevation,
    );
    let (right_s, right_e) = ineffective_face_blocks(
        params.ineffective_right_stations_downstream.as_ref(),
        params.ineffective_right_elevations_downstream.as_ref(),
        params.ineffective_right_station_downstream,
        params.ineffective_right_elevation_downstream,
        params.ineffective_right_stations.as_ref(),
        params.ineffective_right_elevations.as_ref(),
        params.ineffective_right_station,
        params.ineffective_right_elevation,
    );
    IneffectiveFlowAreas::from_block_pairs(&left_s, &left_e, &right_s, &right_e)
}

pub(crate) fn interval_length_metric(params: &BridgeSolveParams) -> f64 {
    let interior = crate::solvers::bridge_interior::BridgeInteriorInput {
        bu: params.xs_up.clone(),
        bd: params.xs_down.clone(),
        internal: params.xs_internal.clone().unwrap_or_default(),
        opening_reach_station_origin: params.opening_reach_station_origin,
        ..Default::default()
    };
    let from_faces = crate::solvers::bridge_interior::resolve_bridge_friction_length_metric(
        &interior,
        0.0,
        params.length,
        params.units,
    );
    if from_faces > 1e-3 {
        return from_faces;
    }
    let len = if params.length > 1e-6 {
        params.length
    } else if params.units == UnitSystem::USCustomary {
        50.0
    } else {
        15.0
    };
    if params.units == UnitSystem::USCustomary {
        len * FT_TO_M
    } else {
        len
    }
}

pub(crate) fn geometry_tables_from_params(
    params: &BridgeSolveParams,
) -> (GeometryTable, GeometryTable, CrossSection, CrossSection) {
    let num_slices = params.num_slices.max(2);
    let manning_n = if params.manning_n > 1e-6 {
        params.manning_n
    } else {
        0.03
    };

    let (xs_up, xs_down) = if params.xs_up.is_some() || params.xs_down.is_some() {
        let up = params
            .xs_up
            .clone()
            .unwrap_or_else(|| {
                rectangular_channel_cross_section(
                    params.channel_width.max(1e-3),
                    params.z_up,
                    manning_n,
                    params.units,
                )
            });
        let down = params
            .xs_down
            .clone()
            .unwrap_or_else(|| {
                rectangular_channel_cross_section(
                    params.channel_width.max(1e-3),
                    params.z_down,
                    manning_n,
                    params.units,
                )
            });
        (up, down)
    } else {
        (
            rectangular_channel_cross_section(
                params.channel_width.max(1e-3),
                params.z_up,
                manning_n,
                params.units,
            ),
            rectangular_channel_cross_section(
                params.channel_width.max(1e-3),
                params.z_down,
                manning_n,
                params.units,
            ),
        )
    };

    let up_metric = xs_up.to_metric();
    let down_metric = xs_down.to_metric();
    (
        up_metric.generate_lookup_table(num_slices),
        down_metric.generate_lookup_table(num_slices),
        xs_up,
        xs_down,
    )
}

/// Solves upstream headwater from fixed tailwater using [`BridgeSolveParams`].
pub fn solve_bridge_from_params(params: &BridgeSolveParams) -> BridgeSolveResult {
    let mut params = params.clone();
    crate::solvers::bridge_roadway_compose::apply_roadway_embankment_compose_params(&mut params);
    let (_table_up, _table_down, mut xs_up, mut xs_down) = geometry_tables_from_params(&params);
    let opening_origin = params
        .opening_reach_station_origin
        .or_else(|| {
            Some(crate::solvers::bridge_interior::infer_opening_reach_station_origin(
                &xs_up,
            ))
        });
    if let Some(blocked) = params.composed_embankment_blocked.as_ref() {
        crate::solvers::bridge_roadway_compose::merge_embankment_blocked_into_section(
            &mut xs_up,
            blocked.left.as_ref(),
            blocked.right.as_ref(),
            opening_origin,
        );
        crate::solvers::bridge_roadway_compose::merge_embankment_blocked_into_section(
            &mut xs_down,
            blocked.left.as_ref(),
            blocked.right.as_ref(),
            opening_origin,
        );
    }
    let (table_up, table_down) = {
        let up_metric = xs_up.to_metric();
        let down_metric = xs_down.to_metric();
        (
            up_metric.generate_lookup_table(params.num_slices),
            down_metric.generate_lookup_table(params.num_slices),
        )
    };
    let coupling = coupling_from_params(&params);
    let deck = build_bridge_deck_profile(
        params.low_chord,
        params.high_chord,
        params.deck_stations.as_deref(),
        params.deck_low_elevations.as_deref(),
        params.deck_high_elevations.as_deref(),
        params.units,
    );
    let interior = crate::solvers::bridge_interior::BridgeInteriorInput {
        bu: Some(xs_up.clone()),
        bd: Some(xs_down.clone()),
        internal: params.xs_internal.clone().unwrap_or_default(),
        opening_reach_station_origin: opening_origin,
        ..Default::default()
    };
    let friction_lengths = crate::solvers::bridge_interior::resolve_bridge_friction_lengths_metric(
        &interior,
        0.0,
        params.length,
        None,
        None,
        Some(&xs_up),
        Some(&xs_down),
        BridgeFrictionWeighting::from_i32(params.friction_weighting),
        params.approach_friction_length,
        params.departure_friction_length,
        params.units,
    );
    let friction_length_m = friction_lengths.opening_m;
    let sections = BridgeSectionContext {
        ineffective_up: ineffective_upstream_from_params(&params),
        ineffective_down: ineffective_downstream_from_params(&params),
        xs_up: Some(xs_up),
        xs_down: Some(xs_down),
        internal_xs: interior.internal,
        opening_reach_station_origin: opening_origin,
        skew_deg: params.skew_deg,
        pier_stations: params.pier_stations.clone(),
        pier_widths: crate::solvers::pier_geometry::pier_width_user_from_rating_params(
            &params.pier_top_widths,
            &params.pier_bottom_widths,
            &params.pier_width_elevations,
            &params.pier_width_values,
            &params.pier_top_elevations,
            &params.pier_base_elevations,
        ),
        pier_attachments: crate::solvers::pier_geometry::pier_attachments_from_rating_params(
            &params.pier_footing_top_elevations,
            &params.pier_footing_widths,
            &params.pier_footing_bottom_elevations,
            &params.pier_nosing_lengths,
            &params.pier_nosing_widths,
        ),
        deck_vents: crate::solvers::deck_vent_geometry::deck_vents_from_rating_params(
            &params.deck_vent_left_stations,
            &params.deck_vent_right_stations,
            &params.deck_vent_stations,
            &params.deck_vent_widths,
            &params.deck_vent_invert_elevations,
            &params.deck_vent_soffit_elevations,
            &params.deck_vent_discharge_coefficients,
            &params.deck_vent_types,
        ),
        friction_length_m,
        friction_lengths,
        xs_approach: None,
        xs_departure: None,
        guide_banks_approach: None,
        guide_banks_departure: None,
    };
    let weir_coeff = if params.weir_coeff > 1e-6 {
        params.weir_coeff
    } else {
        default_weir_coeff_for_units(params.units)
    };
    let orifice_coeff = if params.orifice_coeff > 1e-6 {
        params.orifice_coeff
    } else {
        0.5
    };

    let direction = BridgeFlowDirection::from_q(params.q);
    let tw_hyd = match direction {
        BridgeFlowDirection::Downstream => params.tw_wsel,
        BridgeFlowDirection::Upstream => params.tw_wsel_reverse.unwrap_or(params.tw_wsel),
    };
    let reach = solve_bridge_coupled(
        params.q,
        params.low_chord,
        params.high_chord,
        params.pier_width,
        params.num_piers,
        params.pier_shape_type,
        weir_coeff,
        orifice_coeff,
        params.z_down,
        params.z_up,
        tw_hyd,
        params.units,
        &table_up,
        &table_down,
        &coupling,
        interval_length_metric(&params),
        deck.as_ref(),
        Some(&sections),
    );
    let (hw, tw) = hydraulic_hw_tw_reach(direction, reach.wsel_up, reach.wsel_down);
    BridgeSolveResult {
        wsel_up: hw,
        wsel_down: tw,
        head_loss: reach.head_loss,
        flow_regime: reach.flow_regime,
    }
}

/// Compute upstream headwater vs discharge at fixed tailwater (bridge rating curve).
pub fn compute_bridge_rating_curve(inputs: &BridgeRatingCurveInputs) -> BridgeRatingCurveResult {
    let mut q = Vec::with_capacity(inputs.q_values.len());
    let mut wsel = Vec::with_capacity(inputs.q_values.len());
    let mut wsel_down = Vec::with_capacity(inputs.q_values.len());
    let mut flow_regimes = Vec::with_capacity(inputs.q_values.len());
    let mut head_losses = Vec::with_capacity(inputs.q_values.len());

    for &q_sample in &inputs.q_values {
        if q_sample.abs() < 1e-12 {
            continue;
        }
        let mut params = inputs.bridge.clone();
        params.q = q_sample;
        let result = solve_bridge_from_params(&params);
        q.push(q_sample);
        wsel.push(result.wsel_up);
        wsel_down.push(result.wsel_down);
        flow_regimes.push(result.flow_regime);
        head_losses.push(result.head_loss);
    }

    BridgeRatingCurveResult {
        q,
        wsel,
        wsel_down,
        flow_regimes,
        head_losses,
    }
}
