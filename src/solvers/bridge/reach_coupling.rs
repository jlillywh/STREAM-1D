//! Shared bridge reach coupling helpers (steady + unsteady DRY).

use crate::geometry::{CrossSection, GeometryTable, IneffectiveFlowAreas};
use crate::solvers::bridge_abutment::abutment_user_input_from_steady;
use crate::solvers::bridge_interior::{
    resolve_approach_departure_sections, resolve_bridge_face_solve_geometry, BridgeFaceSolveGeometry,
    BridgeFaceSolveParams, BridgeInteriorInput,
};
use crate::solvers::bridge_roadway_compose::ComposedEmbankmentBlocked;
use crate::utils::{UnitSystem, FT_TO_M};

use super::geometry::{build_bridge_deck_profile, BridgeDeckProfile};
use super::ice_debris::ice_debris_params_for_bridge;
use super::section::BridgeFrictionWeighting;
use super::types::BridgeCouplingParams;

/// Per-bridge array fields shared between [`SteadyInputs`](crate::solvers::steady::SteadyInputs)
/// and [`UnsteadyBridgeInputs`](crate::solvers::unsteady::UnsteadyBridgeInputs).
pub struct BridgeReachFields<'a> {
    pub low_chords: &'a Option<Vec<f64>>,
    pub high_chords: &'a Option<Vec<f64>>,
    pub deck_stations: &'a Option<Vec<Vec<f64>>>,
    pub deck_low_elevations: &'a Option<Vec<Vec<f64>>>,
    pub deck_high_elevations: &'a Option<Vec<Vec<f64>>>,
    pub ineffective_left_stations: &'a Option<Vec<Vec<f64>>>,
    pub ineffective_left_elevations: &'a Option<Vec<Vec<f64>>>,
    pub ineffective_right_stations: &'a Option<Vec<Vec<f64>>>,
    pub ineffective_right_elevations: &'a Option<Vec<Vec<f64>>>,
    pub ineffective_left_stations_upstream: &'a Option<Vec<Vec<f64>>>,
    pub ineffective_left_elevations_upstream: &'a Option<Vec<Vec<f64>>>,
    pub ineffective_right_stations_upstream: &'a Option<Vec<Vec<f64>>>,
    pub ineffective_right_elevations_upstream: &'a Option<Vec<Vec<f64>>>,
    pub ineffective_left_stations_downstream: &'a Option<Vec<Vec<f64>>>,
    pub ineffective_left_elevations_downstream: &'a Option<Vec<Vec<f64>>>,
    pub ineffective_right_stations_downstream: &'a Option<Vec<Vec<f64>>>,
    pub ineffective_right_elevations_downstream: &'a Option<Vec<Vec<f64>>>,
    pub abutment_block_widths: &'a Option<Vec<f64>>,
    pub abutment_left_widths: &'a Option<Vec<f64>>,
    pub abutment_right_widths: &'a Option<Vec<f64>>,
    pub abutment_left_stations: &'a Option<Vec<f64>>,
    pub abutment_right_stations: &'a Option<Vec<f64>>,
    pub abutment_left_top_elevations: &'a Option<Vec<f64>>,
    pub abutment_right_top_elevations: &'a Option<Vec<f64>>,
    pub abutment_left_top_profile_stations: &'a Option<Vec<Vec<f64>>>,
    pub abutment_left_top_profile_elevations: &'a Option<Vec<Vec<f64>>>,
    pub abutment_right_top_profile_stations: &'a Option<Vec<Vec<f64>>>,
    pub abutment_right_top_profile_elevations: &'a Option<Vec<Vec<f64>>>,
    pub low_flow_methods: &'a Option<Vec<i32>>,
    pub high_flow_methods: &'a Option<Vec<i32>>,
    pub lengths: &'a Option<Vec<f64>>,
    pub friction_weighting: &'a Option<Vec<i32>>,
    pub approach_friction_lengths: &'a Option<Vec<f64>>,
    pub departure_friction_lengths: &'a Option<Vec<f64>>,
    pub opening_blockage_factors: &'a Option<Vec<f64>>,
    pub pier_debris_widths: &'a Option<Vec<Vec<f64>>>,
    pub pier_debris_heights: &'a Option<Vec<Vec<f64>>>,
    pub ice_thicknesses: &'a Option<Vec<f64>>,
    pub ice_modes: &'a Option<Vec<i32>>,
    pub deck_ice_thicknesses: &'a Option<Vec<f64>>,
    pub wspro_coeffs: &'a Option<Vec<f64>>,
    pub pressure_flow_coeffs_inlet: &'a Option<Vec<f64>>,
    pub max_weir_submergence: &'a Option<Vec<f64>>,
    pub coeff_contraction: Option<f64>,
    pub coeff_expansion: Option<f64>,
    pub skew_angles: &'a Option<Vec<f64>>,
    pub pier_stations: &'a Option<Vec<Vec<f64>>>,
    pub pier_top_widths: &'a Option<Vec<Vec<f64>>>,
    pub pier_bottom_widths: &'a Option<Vec<Vec<f64>>>,
    pub pier_width_elevations: &'a Option<Vec<Vec<Vec<f64>>>>,
    pub pier_width_values: &'a Option<Vec<Vec<Vec<f64>>>>,
    pub pier_top_elevations: &'a Option<Vec<Vec<f64>>>,
    pub pier_base_elevations: &'a Option<Vec<Vec<f64>>>,
    pub pier_footing_top_elevations: &'a Option<Vec<Vec<f64>>>,
    pub pier_footing_widths: &'a Option<Vec<Vec<f64>>>,
    pub pier_footing_bottom_elevations: &'a Option<Vec<Vec<f64>>>,
    pub pier_nosing_lengths: &'a Option<Vec<Vec<f64>>>,
    pub pier_nosing_widths: &'a Option<Vec<Vec<f64>>>,
    pub deck_vent_left_stations: &'a Option<Vec<Vec<f64>>>,
    pub deck_vent_right_stations: &'a Option<Vec<Vec<f64>>>,
    pub deck_vent_stations: &'a Option<Vec<Vec<f64>>>,
    pub deck_vent_widths: &'a Option<Vec<Vec<f64>>>,
    pub deck_vent_invert_elevations: &'a Option<Vec<Vec<f64>>>,
    pub deck_vent_soffit_elevations: &'a Option<Vec<Vec<f64>>>,
    pub deck_vent_discharge_coefficients: &'a Option<Vec<Vec<f64>>>,
    pub deck_vent_types: &'a Option<Vec<Vec<i32>>>,
    pub composed_embankment_blocked: &'a Option<Vec<Option<ComposedEmbankmentBlocked>>>,
}

fn face_blocks(
    face_stations: &Option<Vec<Vec<f64>>>,
    face_elevations: &Option<Vec<Vec<f64>>>,
    legacy_stations: &Option<Vec<Vec<f64>>>,
    legacy_elevations: &Option<Vec<Vec<f64>>>,
    b_idx: usize,
) -> (Vec<f64>, Vec<f64>) {
    let stations = face_stations
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .filter(|blocks| !blocks.is_empty())
        .cloned()
        .or_else(|| legacy_stations.as_ref().and_then(|v| v.get(b_idx)).cloned())
        .unwrap_or_default();
    let elevations = face_elevations
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .filter(|blocks| !blocks.is_empty())
        .cloned()
        .or_else(|| legacy_elevations.as_ref().and_then(|v| v.get(b_idx)).cloned())
        .unwrap_or_default();
    (stations, elevations)
}

pub fn ineffective_upstream_for(
    fields: &BridgeReachFields<'_>,
    b_idx: usize,
) -> Option<IneffectiveFlowAreas> {
    let (left_s, left_e) = face_blocks(
        fields.ineffective_left_stations_upstream,
        fields.ineffective_left_elevations_upstream,
        fields.ineffective_left_stations,
        fields.ineffective_left_elevations,
        b_idx,
    );
    let (right_s, right_e) = face_blocks(
        fields.ineffective_right_stations_upstream,
        fields.ineffective_right_elevations_upstream,
        fields.ineffective_right_stations,
        fields.ineffective_right_elevations,
        b_idx,
    );
    IneffectiveFlowAreas::from_block_pairs(&left_s, &left_e, &right_s, &right_e)
}

pub fn ineffective_downstream_for(
    fields: &BridgeReachFields<'_>,
    b_idx: usize,
) -> Option<IneffectiveFlowAreas> {
    let (left_s, left_e) = face_blocks(
        fields.ineffective_left_stations_downstream,
        fields.ineffective_left_elevations_downstream,
        fields.ineffective_left_stations,
        fields.ineffective_left_elevations,
        b_idx,
    );
    let (right_s, right_e) = face_blocks(
        fields.ineffective_right_stations_downstream,
        fields.ineffective_right_elevations_downstream,
        fields.ineffective_right_stations,
        fields.ineffective_right_elevations,
        b_idx,
    );
    IneffectiveFlowAreas::from_block_pairs(&left_s, &left_e, &right_s, &right_e)
}

pub fn deck_profile_for(
    fields: &BridgeReachFields<'_>,
    b_idx: usize,
    raw_units: UnitSystem,
) -> Option<BridgeDeckProfile> {
    let low_chord = fields
        .low_chords
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .copied()
        .unwrap_or(0.0);
    let high_chord = fields
        .high_chords
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .copied()
        .unwrap_or(0.0);
    build_bridge_deck_profile(
        low_chord,
        high_chord,
        fields
            .deck_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .map(|s| s.as_slice()),
        fields
            .deck_low_elevations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .map(|s| s.as_slice()),
        fields
            .deck_high_elevations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .map(|s| s.as_slice()),
        raw_units,
    )
}

pub fn coupling_for(fields: &BridgeReachFields<'_>, b_idx: usize) -> BridgeCouplingParams {
    let abutment = abutment_user_input_from_steady(
        fields
            .abutment_block_widths
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied(),
        fields.abutment_left_widths.as_ref(),
        fields.abutment_right_widths.as_ref(),
        fields.abutment_left_stations.as_ref(),
        fields.abutment_right_stations.as_ref(),
        fields.abutment_left_top_elevations.as_ref(),
        fields.abutment_right_top_elevations.as_ref(),
        fields.abutment_left_top_profile_stations.as_ref(),
        fields.abutment_left_top_profile_elevations.as_ref(),
        fields.abutment_right_top_profile_stations.as_ref(),
        fields.abutment_right_top_profile_elevations.as_ref(),
        b_idx,
    );
    BridgeCouplingParams {
        abutment,
        low_flow_method: fields
            .low_flow_methods
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0),
        high_flow_method: fields
            .high_flow_methods
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0),
        length: fields
            .lengths
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0),
        wspro_coeff: fields
            .wspro_coeffs
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.8),
        coeff_contraction: fields.coeff_contraction.unwrap_or(0.1),
        coeff_expansion: fields.coeff_expansion.unwrap_or(0.3),
        pressure_coeff_inlet: fields
            .pressure_flow_coeffs_inlet
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0),
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: fields
            .max_weir_submergence
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.98),
        friction_weighting: BridgeFrictionWeighting::from_i32(
            fields
                .friction_weighting
                .as_ref()
                .and_then(|v| v.get(b_idx))
                .copied()
                .unwrap_or(0),
        ),
        approach_friction_length: fields
            .approach_friction_lengths
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0),
        departure_friction_length: fields
            .departure_friction_lengths
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0),
        ice_debris: ice_debris_params_for_bridge(
            fields.opening_blockage_factors,
            fields.pier_debris_widths,
            fields.pier_debris_heights,
            fields.ice_thicknesses,
            fields.ice_modes,
            fields.deck_ice_thicknesses,
            b_idx,
        ),
    }
}

pub struct BridgeFaceGeometryRequest<'a> {
    pub fields: &'a BridgeReachFields<'a>,
    pub interior: &'a BridgeInteriorInput,
    pub b_idx: usize,
    pub i: usize,
    pub raw_units: UnitSystem,
    pub num_slices: usize,
    pub densified_stations: &'a [f64],
    pub densified_tables: &'a [GeometryTable],
    pub densified_xs: &'a [Option<CrossSection>],
    pub densified_z_mins: &'a [f64],
    pub interval_length_m: f64,
    pub anchor_reach_xs: Option<&'a CrossSection>,
}

pub fn face_geometry_for(req: BridgeFaceGeometryRequest<'_>) -> BridgeFaceSolveGeometry {
    let reach_z_up_user = if req.raw_units == UnitSystem::USCustomary {
        req.densified_z_mins[req.i] / FT_TO_M
    } else {
        req.densified_z_mins[req.i]
    };
    let reach_z_down_user = if req.raw_units == UnitSystem::USCustomary {
        req.densified_z_mins[req.i + 1] / FT_TO_M
    } else {
        req.densified_z_mins[req.i + 1]
    };
    let (approach_xs, departure_xs, guide_banks_approach, guide_banks_departure) =
        resolve_approach_departure_sections(
            req.interior,
            req.i,
            req.densified_stations,
            req.densified_xs,
            req.raw_units,
        );
    resolve_bridge_face_solve_geometry(BridgeFaceSolveParams {
        interior: req.interior,
        anchor_reach_xs: req.anchor_reach_xs,
        reach_xs_up: req.densified_xs[req.i].as_ref(),
        reach_xs_down: req.densified_xs[req.i + 1].as_ref(),
        reach_table_up: &req.densified_tables[req.i],
        reach_table_down: &req.densified_tables[req.i + 1],
        reach_z_up_user,
        reach_z_down_user,
        raw_units: req.raw_units,
        num_slices: req.num_slices,
        ineffective_up: ineffective_upstream_for(req.fields, req.b_idx),
        ineffective_down: ineffective_downstream_for(req.fields, req.b_idx),
        skew_deg: req
            .fields
            .skew_angles
            .as_ref()
            .and_then(|v| v.get(req.b_idx))
            .copied()
            .unwrap_or(0.0),
        pier_stations: req
            .fields
            .pier_stations
            .as_ref()
            .and_then(|v| v.get(req.b_idx))
            .cloned(),
        interval_length_m: req.interval_length_m,
        bridge_length_user: req
            .fields
            .lengths
            .as_ref()
            .and_then(|v| v.get(req.b_idx))
            .copied()
            .unwrap_or(0.0),
        friction_weighting: BridgeFrictionWeighting::from_i32(
            req.fields
                .friction_weighting
                .as_ref()
                .and_then(|v| v.get(req.b_idx))
                .copied()
                .unwrap_or(0),
        ),
        approach_friction_length_user: req
            .fields
            .approach_friction_lengths
            .as_ref()
            .and_then(|v| v.get(req.b_idx))
            .copied()
            .unwrap_or(0.0),
        departure_friction_length_user: req
            .fields
            .departure_friction_lengths
            .as_ref()
            .and_then(|v| v.get(req.b_idx))
            .copied()
            .unwrap_or(0.0),
        approach_xs,
        departure_xs,
        guide_banks_approach,
        guide_banks_departure,
        pier_widths: crate::solvers::pier_geometry::pier_width_user_for_bridge_index(
            req.fields.pier_top_widths,
            req.fields.pier_bottom_widths,
            req.fields.pier_width_elevations,
            req.fields.pier_width_values,
            req.fields.pier_top_elevations,
            req.fields.pier_base_elevations,
            req.b_idx,
        ),
        pier_attachments: crate::solvers::pier_geometry::pier_attachments_user_for_bridge_index(
            req.fields.pier_footing_top_elevations,
            req.fields.pier_footing_widths,
            req.fields.pier_footing_bottom_elevations,
            req.fields.pier_nosing_lengths,
            req.fields.pier_nosing_widths,
            req.b_idx,
        ),
        deck_vents: crate::solvers::deck_vent_geometry::deck_vents_user_for_bridge_index(
            req.fields.deck_vent_left_stations,
            req.fields.deck_vent_right_stations,
            req.fields.deck_vent_stations,
            req.fields.deck_vent_widths,
            req.fields.deck_vent_invert_elevations,
            req.fields.deck_vent_soffit_elevations,
            req.fields.deck_vent_discharge_coefficients,
            req.fields.deck_vent_types,
            req.b_idx,
        ),
        embankment_blocked: crate::solvers::bridge_roadway_compose::composed_embankment_blocked_for(
            req.fields.composed_embankment_blocked,
            req.b_idx,
        ),
    })
}
