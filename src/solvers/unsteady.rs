use crate::utils::{G_METRIC, UnitSystem, FT_TO_M};
use crate::geometry::{
    flow_area_for_row, geometry_row_at_elevation, CrossSection, DensifyReachModifierPolicy,
    GeometryTable, IneffectiveFlowAreas,
};

#[path = "unsteady/structure_coupling.rs"]
mod structure_coupling;
#[path = "unsteady/preissmann.rs"]
mod preissmann;
#[path = "unsteady/culvert_implicit.rs"]
mod culvert_implicit;

pub use preissmann::{solve_preissmann_step, PreissmannStepParams, UnsteadyStructureCouplingMode};

/// Culvert model fields for unsteady routing (flattened into JSON; same keys as steady).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UnsteadyCulvertInputs {
    #[serde(default)]
    pub culvert_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_shape_types: Option<Vec<i32>>,
    #[serde(default)]
    pub culvert_spans: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_rises: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_roughness_ns: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_lengths: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_entrance_loss_coeffs: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_exit_loss_coeffs: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_barrels: Option<Vec<i32>>,
    #[serde(default)]
    pub culvert_roughness_n_bottoms: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_depth_bottom_ns: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_depth_blockeds: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_inlet_types: Option<Vec<i32>>,
    #[serde(default)]
    pub culvert_z_ups: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_z_downs: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_crest_elevs: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_weir_coeffs: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_weir_lengths: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_skew_angles: Option<Vec<f64>>,
    #[serde(default)]
    pub culvert_active_barrels: Option<Vec<i32>>,
    #[serde(default)]
    pub culvert_barrel_spans: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub culvert_barrel_rises: Option<Vec<Vec<f64>>>,
}

/// Bridge model fields for unsteady routing (flattened into JSON; same keys as steady).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UnsteadyBridgeInputs {
    #[serde(default)]
    pub bridge_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_low_chords: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_high_chords: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_pier_widths: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_num_piers: Option<Vec<i32>>,
    #[serde(default)]
    pub bridge_pier_shapes: Option<Vec<i32>>,
    #[serde(default)]
    pub bridge_weir_coeffs: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_orifice_coeffs: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_abutment_block_widths: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_abutment_left_widths: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_abutment_right_widths: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_abutment_left_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_abutment_right_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_abutment_left_top_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_abutment_right_top_elevations: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_abutment_left_top_profile_stations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_abutment_left_top_profile_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_abutment_right_top_profile_stations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_abutment_right_top_profile_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_low_flow_methods: Option<Vec<i32>>,
    #[serde(default)]
    pub bridge_high_flow_methods: Option<Vec<i32>>,
    #[serde(default)]
    pub bridge_lengths: Option<Vec<f64>>,
    /// Friction weighting per bridge: 0 = opening only (default), 1 = HEC-RAS approach + opening + departure.
    #[serde(default)]
    pub bridge_friction_weighting: Option<Vec<i32>>,
    /// Override approach friction length per bridge (user units). 0 = auto from river stations.
    #[serde(default)]
    pub bridge_approach_friction_lengths: Option<Vec<f64>>,
    /// Override departure friction length per bridge (user units). 0 = auto from river stations.
    #[serde(default)]
    pub bridge_departure_friction_lengths: Option<Vec<f64>>,
    /// Net opening area multiplier per bridge (0–1]. Omit or `1.0` = no extra blockage.
    #[serde(default)]
    pub bridge_opening_blockage_factors: Option<Vec<f64>>,
    /// Floating pier debris total width per bridge `[bridge][pier]` (opening coordinates).
    #[serde(default)]
    pub bridge_pier_debris_widths: Option<Vec<Vec<f64>>>,
    /// Floating pier debris height below WSEL per bridge `[bridge][pier]`.
    #[serde(default)]
    pub bridge_pier_debris_heights: Option<Vec<Vec<f64>>>,
    /// Constant ice thickness through opening per bridge (user units).
    #[serde(default)]
    pub bridge_ice_thicknesses: Option<Vec<f64>>,
    /// Ice mode per bridge: `0` = none, `1` = constant thickness, `2` = reserved.
    #[serde(default)]
    pub bridge_ice_modes: Option<Vec<i32>>,
    /// Roadway ice lowering weir crest per bridge (user units).
    #[serde(default)]
    pub bridge_deck_ice_thicknesses: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_wspro_coeffs: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_pressure_flow_coeffs_inlet: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_max_weir_submergence: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_deck_stations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_deck_low_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_deck_high_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_left_stations: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_left_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_right_stations: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_right_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_left_stations_upstream: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_left_elevations_upstream: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_right_stations_upstream: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_right_elevations_upstream: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_left_stations_downstream: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_left_elevations_downstream: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_right_stations_downstream: Option<Vec<Vec<f64>>>,
    #[serde(default, deserialize_with = "crate::geometry::ineffective_serde::deserialize_bridge_block_arrays", serialize_with = "crate::geometry::ineffective_serde::serialize_bridge_block_arrays")]
    pub bridge_ineffective_right_elevations_downstream: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_skew_angles: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_pier_stations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_pier_top_widths: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_pier_bottom_widths: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_pier_width_elevations: Option<Vec<Vec<Vec<f64>>>>,
    #[serde(default)]
    pub bridge_pier_width_values: Option<Vec<Vec<Vec<f64>>>>,
    #[serde(default)]
    pub bridge_pier_top_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_pier_base_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_pier_footing_top_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_pier_footing_widths: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_pier_footing_bottom_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_pier_nosing_lengths: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_pier_nosing_widths: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_deck_vent_left_stations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_deck_vent_right_stations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_deck_vent_stations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_deck_vent_widths: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_deck_vent_invert_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_deck_vent_soffit_elevations: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_deck_vent_discharge_coefficients: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub bridge_deck_vent_types: Option<Vec<Vec<i32>>>,
    #[serde(default)]
    pub bridge_upstream_cross_sections: Option<Vec<CrossSection>>,
    #[serde(default)]
    pub bridge_downstream_cross_sections: Option<Vec<CrossSection>>,
    #[serde(default)]
    pub bridge_internal_cross_sections: Option<Vec<Vec<CrossSection>>>,
    #[serde(default)]
    pub bridge_opening_reach_station_origins: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_opening_anchor_modes: Option<Vec<i32>>,
    #[serde(default)]
    pub bridge_opening_anchor_reach_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_approach_cross_sections: Option<Vec<CrossSection>>,
    #[serde(default)]
    pub bridge_departure_cross_sections: Option<Vec<CrossSection>>,
    #[serde(default)]
    pub bridge_approach_reach_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_departure_reach_stations: Option<Vec<f64>>,
    #[serde(default)]
    pub bridge_approach_guide_banks: Option<Vec<crate::geometry::GuideBanks>>,
    #[serde(default)]
    pub bridge_departure_guide_banks: Option<Vec<crate::geometry::GuideBanks>>,
    #[serde(default)]
    pub bridge_roadway_embankments: Option<
        Vec<Option<crate::solvers::bridge_roadway_compose::BridgeRoadwayEmbankment>>,
    >,
    #[serde(default, skip_serializing)]
    pub(crate) bridge_composed_embankment_blocked: Option<
        Vec<Option<crate::solvers::bridge_roadway_compose::ComposedEmbankmentBlocked>>,
    >,
}

pub(crate) fn bridge_reach_fields(
    inputs: &UnsteadyInputs,
) -> crate::solvers::bridge::reach_coupling::BridgeReachFields<'_> {
    let b = &inputs.bridge;
    crate::solvers::bridge::reach_coupling::BridgeReachFields {
        low_chords: &b.bridge_low_chords,
        high_chords: &b.bridge_high_chords,
        deck_stations: &b.bridge_deck_stations,
        deck_low_elevations: &b.bridge_deck_low_elevations,
        deck_high_elevations: &b.bridge_deck_high_elevations,
        ineffective_left_stations: &b.bridge_ineffective_left_stations,
        ineffective_left_elevations: &b.bridge_ineffective_left_elevations,
        ineffective_right_stations: &b.bridge_ineffective_right_stations,
        ineffective_right_elevations: &b.bridge_ineffective_right_elevations,
        ineffective_left_stations_upstream: &b.bridge_ineffective_left_stations_upstream,
        ineffective_left_elevations_upstream: &b.bridge_ineffective_left_elevations_upstream,
        ineffective_right_stations_upstream: &b.bridge_ineffective_right_stations_upstream,
        ineffective_right_elevations_upstream: &b.bridge_ineffective_right_elevations_upstream,
        ineffective_left_stations_downstream: &b.bridge_ineffective_left_stations_downstream,
        ineffective_left_elevations_downstream: &b.bridge_ineffective_left_elevations_downstream,
        ineffective_right_stations_downstream: &b.bridge_ineffective_right_stations_downstream,
        ineffective_right_elevations_downstream: &b.bridge_ineffective_right_elevations_downstream,
        abutment_block_widths: &b.bridge_abutment_block_widths,
        abutment_left_widths: &b.bridge_abutment_left_widths,
        abutment_right_widths: &b.bridge_abutment_right_widths,
        abutment_left_stations: &b.bridge_abutment_left_stations,
        abutment_right_stations: &b.bridge_abutment_right_stations,
        abutment_left_top_elevations: &b.bridge_abutment_left_top_elevations,
        abutment_right_top_elevations: &b.bridge_abutment_right_top_elevations,
        abutment_left_top_profile_stations: &b.bridge_abutment_left_top_profile_stations,
        abutment_left_top_profile_elevations: &b.bridge_abutment_left_top_profile_elevations,
        abutment_right_top_profile_stations: &b.bridge_abutment_right_top_profile_stations,
        abutment_right_top_profile_elevations: &b.bridge_abutment_right_top_profile_elevations,
        low_flow_methods: &b.bridge_low_flow_methods,
        high_flow_methods: &b.bridge_high_flow_methods,
        lengths: &b.bridge_lengths,
        friction_weighting: &b.bridge_friction_weighting,
        approach_friction_lengths: &b.bridge_approach_friction_lengths,
        departure_friction_lengths: &b.bridge_departure_friction_lengths,
        opening_blockage_factors: &b.bridge_opening_blockage_factors,
        pier_debris_widths: &b.bridge_pier_debris_widths,
        pier_debris_heights: &b.bridge_pier_debris_heights,
        ice_thicknesses: &b.bridge_ice_thicknesses,
        ice_modes: &b.bridge_ice_modes,
        deck_ice_thicknesses: &b.bridge_deck_ice_thicknesses,
        wspro_coeffs: &b.bridge_wspro_coeffs,
        pressure_flow_coeffs_inlet: &b.bridge_pressure_flow_coeffs_inlet,
        max_weir_submergence: &b.bridge_max_weir_submergence,
        coeff_contraction: inputs.coeff_contraction,
        coeff_expansion: inputs.coeff_expansion,
        skew_angles: &b.bridge_skew_angles,
        pier_stations: &b.bridge_pier_stations,
        pier_top_widths: &b.bridge_pier_top_widths,
        pier_bottom_widths: &b.bridge_pier_bottom_widths,
        pier_width_elevations: &b.bridge_pier_width_elevations,
        pier_width_values: &b.bridge_pier_width_values,
        pier_top_elevations: &b.bridge_pier_top_elevations,
        pier_base_elevations: &b.bridge_pier_base_elevations,
        pier_footing_top_elevations: &b.bridge_pier_footing_top_elevations,
        pier_footing_widths: &b.bridge_pier_footing_widths,
        pier_footing_bottom_elevations: &b.bridge_pier_footing_bottom_elevations,
        pier_nosing_lengths: &b.bridge_pier_nosing_lengths,
        pier_nosing_widths: &b.bridge_pier_nosing_widths,
        deck_vent_left_stations: &b.bridge_deck_vent_left_stations,
        deck_vent_right_stations: &b.bridge_deck_vent_right_stations,
        deck_vent_stations: &b.bridge_deck_vent_stations,
        deck_vent_widths: &b.bridge_deck_vent_widths,
        deck_vent_invert_elevations: &b.bridge_deck_vent_invert_elevations,
        deck_vent_soffit_elevations: &b.bridge_deck_vent_soffit_elevations,
        deck_vent_discharge_coefficients: &b.bridge_deck_vent_discharge_coefficients,
        deck_vent_types: &b.bridge_deck_vent_types,
        composed_embankment_blocked: &b.bridge_composed_embankment_blocked,
    }
}

pub(crate) fn bridge_ineffective_upstream_for(
    inputs: &UnsteadyInputs,
    b_idx: usize,
) -> Option<IneffectiveFlowAreas> {
    crate::solvers::bridge::reach_coupling::ineffective_upstream_for(&bridge_reach_fields(inputs), b_idx)
}

pub(crate) fn bridge_ineffective_downstream_for(
    inputs: &UnsteadyInputs,
    b_idx: usize,
) -> Option<IneffectiveFlowAreas> {
    crate::solvers::bridge::reach_coupling::ineffective_downstream_for(&bridge_reach_fields(inputs), b_idx)
}

pub(crate) fn bridge_face_geometry_for(
    inputs: &UnsteadyInputs,
    b_idx: usize,
    i: usize,
    raw_units: UnitSystem,
    num_slices: usize,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    interval_length_m: f64,
) -> crate::solvers::bridge_interior::BridgeFaceSolveGeometry {
    let fields = bridge_reach_fields(inputs);
    let b = &inputs.bridge;
    let interior = crate::solvers::bridge_interior::interior_from_unsteady(b, b_idx);
    let anchor_reach_xs = interior
        .opening_anchor_reach_station
        .and_then(|st| {
            crate::solvers::bridge_interior::cross_section_at_reach_station_dense(
                densified_stations,
                densified_xs,
                st,
                raw_units,
            )
        });
    let densified_xs_opt: Vec<Option<CrossSection>> =
        densified_xs.iter().cloned().map(Some).collect();
    crate::solvers::bridge::reach_coupling::face_geometry_for(
        crate::solvers::bridge::reach_coupling::BridgeFaceGeometryRequest {
            fields: &fields,
            interior: &interior,
            b_idx,
            i,
            raw_units,
            num_slices,
            densified_stations,
            densified_tables,
            densified_xs: &densified_xs_opt,
            densified_z_mins,
            interval_length_m,
            anchor_reach_xs: anchor_reach_xs.as_ref(),
        },
    )
}

pub(crate) fn bridge_deck_profile_for(
    inputs: &UnsteadyInputs,
    b_idx: usize,
    raw_units: UnitSystem,
) -> Option<crate::solvers::bridge::BridgeDeckProfile> {
    crate::solvers::bridge::reach_coupling::deck_profile_for(&bridge_reach_fields(inputs), b_idx, raw_units)
}

pub(crate) fn bridge_coupling_for(inputs: &UnsteadyInputs, b_idx: usize) -> crate::solvers::bridge::BridgeCouplingParams {
    crate::solvers::bridge::reach_coupling::coupling_for(&bridge_reach_fields(inputs), b_idx)
}

/// Input parameters for the unsteady-state solver.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnsteadyInputs {
    /// Cross-sections defining the river reach.
    pub cross_sections: Vec<CrossSection>,
    /// Initial water surface elevations (WSEL) at each section (in user units).
    pub initial_wsel: Vec<f64>,
    /// Initial flow rates (Q) at each section (in user units).
    pub initial_q: Vec<f64>,
    /// Simulation time step size (in seconds).
    pub dt: f64,
    /// Number of time steps to run.
    pub num_steps: usize,
    /// Upstream flow hydrograph boundary condition (in user units, array of size num_steps).
    pub upstream_q_hydrograph: Vec<f64>,
    /// Downstream stage hydrograph boundary condition (in user units, array of size num_steps).
    pub downstream_wsel_hydrograph: Vec<f64>,
    /// Preissmann weighting factor theta (typically 0.55 to 0.7, default 0.6).
    pub theta: Option<f64>,
    /// Number of uniform vertical slices for geometry lookup tables (default 100).
    pub num_slices: Option<usize>,
    /// Maximum distance between adjacent sections before automatic interpolation (optional, in user units).
    pub max_spacing: Option<f64>,
    /// Reach modifier inheritance on `max_spacing` interior nodes: 0=none, 1=upstream, 2=downstream, 3=nearest.
    #[serde(default)]
    pub densify_reach_modifier_policy: Option<u8>,
    /// Contraction loss coefficient (default 0.1).
    pub coeff_contraction: Option<f64>,
    /// Expansion loss coefficient (default 0.3).
    pub coeff_expansion: Option<f64>,

    /// Culvert model inputs (same JSON keys as steady `SteadyInputs`).
    #[serde(default)]
    #[serde(flatten)]
    pub culvert: UnsteadyCulvertInputs,

    /// Bridge model inputs (same JSON keys as steady `SteadyInputs`).
    #[serde(default)]
    #[serde(flatten)]
    pub bridge: UnsteadyBridgeInputs,

    /// How inline culverts and bridges are coupled each post-step pass:
    /// `0` = combined downstream-first (default), `1` = all culverts then all bridges,
    /// `2` = all bridges then all culverts.
    #[serde(default)]
    pub structure_coupling_order: Option<i32>,

    /// Preissmann structure coupling: `0` = post-step only (default), `2` = implicit Jacobian (opt-in).
    #[serde(default)]
    pub unsteady_structure_coupling_mode: Option<i32>,
}

/// Output results from the unsteady-state solver.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnsteadyResult {
    /// Time history of water surface elevations (WSEL) [step][section] (in user units).
    pub wsel: Vec<Vec<f64>>,
    /// Time history of flow rates (Q) [step][section] (in user units).
    pub q: Vec<Vec<f64>>,
    /// Time history of flow velocities [step][section] (in user units).
    pub velocity: Vec<Vec<f64>>,
    /// Maximum Courant number encountered during initial conditions.
    pub max_courant: Option<f64>,
    /// Recommended optimal time-step size to ensure stability (in seconds).
    pub recommended_dt: Option<f64>,
    /// Per-culvert control mechanism each time step (`"inlet"` | `"outlet"` | `"overtopping"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub culvert_control_types: Option<Vec<Vec<String>>>,
    /// Tier 2a culvert diagnostics [step][culvert] (same field names as steady `SteadyResult`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub culvert_wsel_inlet: Option<Vec<Vec<f64>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub culvert_wsel_outlet: Option<Vec<Vec<f64>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub culvert_q_barrels: Option<Vec<Vec<f64>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub culvert_q_weirs: Option<Vec<Vec<f64>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub culvert_barrel_depths: Option<Vec<Vec<f64>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub culvert_barrel_velocities: Option<Vec<Vec<f64>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub culvert_barrel_froude: Option<Vec<Vec<f64>>>,
    /// Per-bridge flow regime each time step (`low_a` | `low_b` | `low_c` | `pressure` | `weir`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_flow_regimes: Option<Vec<Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_wsel_upstream: Option<Vec<Vec<f64>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_wsel_downstream: Option<Vec<Vec<f64>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_head_losses: Option<Vec<Vec<f64>>>,
}

/// Solves a single unsteady time step (reach-only; no structure interval tags).
pub fn solve_unsteady_step(
    tables: &[GeometryTable],
    xs_list: &[CrossSection],
    z_mins: &[f64],
    y_current: &[f64],
    q_current: &[f64],
    dt: f64,
    q_up_next: f64,
    y_down_next: f64,
    theta: f64,
    c_contraction: f64,
    c_expansion: f64,
) -> Option<(Vec<f64>, Vec<f64>)> {
    let params = PreissmannStepParams {
        tables,
        xs_list,
        z_mins,
        y_current,
        q_current,
        dt,
        q_up_next,
        y_down_next,
        theta,
        c_contraction,
        c_expansion,
        structure_coupling_mode: UnsteadyStructureCouplingMode::PostStepOnly,
        culvert_intervals: &[],
        bridge_intervals: &[],
        unsteady_inputs: None,
        raw_units: UnitSystem::Metric,
        #[cfg(test)]
        implicit_hook_probe: None,
    };
    preissmann::solve_preissmann_step(&params)
}

/// Solves unsteady-state Saint-Venant flow routing.
pub fn solve_unsteady(inputs: &UnsteadyInputs) -> UnsteadyResult {
    let inputs = crate::solvers::bridge_roadway_compose::composed_unsteady_inputs(inputs);
    let raw_units = inputs.cross_sections.first().map(|xs| xs.unit_system).unwrap_or(UnitSystem::Metric);
    let dt = inputs.dt;
    let num_slices = inputs.num_slices.unwrap_or(100);
    let theta = inputs.theta.unwrap_or(0.85).clamp(0.85, 1.0);
    let c_contraction = inputs.coeff_contraction.unwrap_or(0.1);
    let c_expansion = inputs.coeff_expansion.unwrap_or(0.3);

    // Convert cross-sections to metric and sort descending by station
    let mut xs_list: Vec<CrossSection> = inputs.cross_sections.iter().map(|xs| xs.to_metric()).collect();
    xs_list.sort_by(|a, b| b.station.partial_cmp(&a.station).unwrap());
    let m = xs_list.len();

    // Map from original index to sorted index for indexing initial states
    let mut original_mapping = vec![0; m];
    for (orig_idx, orig_xs) in inputs.cross_sections.iter().enumerate() {
        let mut sorted_idx = 0;
        for (s_idx, s_xs) in xs_list.iter().enumerate() {
            if (s_xs.station - (orig_xs.station * if raw_units == UnitSystem::USCustomary { FT_TO_M } else { 1.0 })).abs() < 1e-4 {
                sorted_idx = s_idx;
                break;
            }
        }
        original_mapping[orig_idx] = sorted_idx;
    }

    // Setup initial conditions in metric
    let mut y_current = vec![0.0; m];
    let mut q_current = vec![0.0; m];

    let initial_wsel_warmed = if !inputs.initial_wsel.is_empty() {
        let steady_inputs = crate::solvers::steady::SteadyInputs {
            cross_sections: inputs.cross_sections.clone(),
            flow_rate: inputs.initial_q.first().cloned().unwrap_or(0.0),
            num_slices: inputs.num_slices,
            coeff_contraction: inputs.coeff_contraction,
            coeff_expansion: inputs.coeff_expansion,
            regime: 0, // Subcritical GVF sweep
            downstream_wsel: inputs.downstream_wsel_hydrograph.first().cloned(),
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
            bridge_ineffective_left_elevations: inputs.bridge.bridge_ineffective_left_elevations.clone(),
            bridge_ineffective_right_stations: inputs.bridge.bridge_ineffective_right_stations.clone(),
            bridge_ineffective_right_elevations: inputs.bridge.bridge_ineffective_right_elevations.clone(),
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
        };
        let steady_res = crate::solvers::steady::solve_steady(&steady_inputs);
        steady_res.wsel
    } else {
        inputs.initial_wsel.clone()
    };

    for orig_idx in 0..m {
        let sorted_idx = original_mapping[orig_idx];
        let wsel_val = initial_wsel_warmed[orig_idx];
        let q_val = inputs.initial_q[orig_idx];
        
        y_current[sorted_idx] = if raw_units == UnitSystem::USCustomary { wsel_val * FT_TO_M } else { wsel_val };
        q_current[sorted_idx] = if raw_units == UnitSystem::USCustomary { q_val * crate::utils::CFS_TO_CMS } else { q_val };
    }

    // Pre-build geometry tables for sorted cross sections
    let tables: Vec<GeometryTable> = xs_list.iter().map(|xs| xs.generate_lookup_table(num_slices)).collect();
    let z_mins: Vec<f64> = xs_list.iter().map(|xs| xs.y.iter().cloned().fold(f64::INFINITY, f64::min)).collect();

    // DENSIFICATION STEP: Automatic Reach Interpolation
    let max_sp = inputs.max_spacing.map(|sp| {
        if raw_units == UnitSystem::USCustomary { sp * FT_TO_M } else { sp }
    }).unwrap_or_else(|| {
        if raw_units == UnitSystem::USCustomary { 50.0 * FT_TO_M } else { 15.0 }
    });
    let densify_policy = DensifyReachModifierPolicy::from_option(inputs.densify_reach_modifier_policy);

    let mut densified_tables = Vec::new();
    let mut densified_z_mins = Vec::new();
    let mut densified_stations = Vec::new();
    let mut densified_xs = Vec::new();
    let mut densified_y_current = Vec::new();
    let mut densified_q_current = Vec::new();
    let mut original_to_densified = Vec::new();

    for i in 0..m {
        let current_idx = densified_tables.len();
        original_to_densified.push(current_idx);

        densified_tables.push(tables[i].clone());
        densified_z_mins.push(z_mins[i]);
        densified_stations.push(xs_list[i].station);
        densified_xs.push(xs_list[i].clone());
        densified_y_current.push(y_current[i]);
        densified_q_current.push(q_current[i]);

        if i < m - 1 {
            let dx = xs_list[i].station - xs_list[i + 1].station;
            if max_sp > 0.0 && dx > max_sp {
                let num_spaces = (dx / max_sp).ceil() as usize;
                let ds = dx / num_spaces as f64;
                for k in 1..num_spaces {
                        let t = k as f64 / num_spaces as f64;
                        let s_interp = xs_list[i].station - k as f64 * ds;

                        let (t_interp, z_interp, xs_opt) =
                            crate::geometry::densify_interior_node(
                                &xs_list[i],
                                &xs_list[i + 1],
                                &tables[i],
                                z_mins[i],
                                &tables[i + 1],
                                z_mins[i + 1],
                                s_interp,
                                t,
                                num_slices,
                                densify_policy,
                            );

                        let y_interp = (1.0 - t) * y_current[i] + t * y_current[i + 1];
                        let q_interp = (1.0 - t) * q_current[i] + t * q_current[i + 1];

                        let xs_interp = xs_opt.unwrap_or_else(|| {
                            let mut xs = xs_list[i].clone();
                            xs.station = s_interp;
                            xs
                        });

                        densified_tables.push(t_interp);
                        densified_z_mins.push(z_interp);
                        densified_stations.push(s_interp);
                        densified_xs.push(xs_interp);
                        densified_y_current.push(y_interp);
                        densified_q_current.push(q_interp);
                    }
                }
            }
        }

    let bridge_face_intervals = crate::solvers::bridge_interior::apply_bridge_reach_layout_unsteady(
        &inputs,
        raw_units,
        num_slices,
        &mut densified_stations,
        &mut densified_tables,
        &mut densified_z_mins,
        &mut densified_xs,
        &mut densified_y_current,
        &mut densified_q_current,
    );

    let dm = densified_tables.len();

    let culvert_intervals = inputs
        .culvert
        .culvert_stations
        .as_ref()
        .map(|stations| structure_coupling::find_structure_intervals(stations, &densified_stations, raw_units))
        .unwrap_or_default();

    let bridge_intervals =
        crate::solvers::bridge_interior::bridge_intervals_from_faces(&bridge_face_intervals);

    let structure_coupling_mode = UnsteadyStructureCouplingMode::from_i32(
        inputs.unsteady_structure_coupling_mode.unwrap_or(0),
    );

    let track_culvert_diagnostics = !culvert_intervals.is_empty();
    let track_bridge_diagnostics = !bridge_intervals.is_empty();
    let has_structures = track_culvert_diagnostics || track_bridge_diagnostics;

    if has_structures {
        structure_coupling::apply_structure_internal_boundaries(
            &inputs,
            raw_units,
            &densified_tables,
            &densified_xs,
            &densified_z_mins,
            &densified_stations,
            &mut densified_y_current,
            &densified_q_current,
            &culvert_intervals,
            &bridge_intervals,
        );
    }

    // Calculate Courant number (Cr) and recommended dt based on initial conditions on the densified grid
    let mut max_courant = 0.0;
    let mut recommended_dt = f64::INFINITY;

    for k in 0..dm {
        let y_val = densified_y_current[k];
        let q_val = densified_q_current[k];
        let row = geometry_row_at_elevation(
            &densified_tables[k],
            Some(&densified_xs[k]),
            y_val,
            None,
            None,
        );

        let area = row.area;
        let top_width = row.top_width;
        let flow_area = flow_area_for_row(&row);
        let vel = if flow_area > 1e-6 { q_val / flow_area } else { 0.0 };
        let d_hyd = if top_width > 1e-6 { area / top_width } else { 0.0 };
        let celerity = (G_METRIC * d_hyd).sqrt();
        let wave_speed = vel.abs() + celerity;

        let dx = if dm < 2 {
            1.0
        } else if k == 0 {
            densified_stations[0] - densified_stations[1]
        } else if k == dm - 1 {
            densified_stations[dm - 2] - densified_stations[dm - 1]
        } else {
            let dx_prev = densified_stations[k - 1] - densified_stations[k];
            let dx_next = densified_stations[k] - densified_stations[k + 1];
            dx_prev.min(dx_next)
        };

        if dx > 1e-9 {
            let cr = (wave_speed * dt) / dx;
            if cr > max_courant {
                max_courant = cr;
            }
            if wave_speed > 1e-6 {
                let dt_opt = (5.0 * dx) / wave_speed;
                if dt_opt < recommended_dt {
                    recommended_dt = dt_opt;
                }
            }
        }
    }

    let (max_courant_val, recommended_dt_val) = if dm >= 2 {
        (
            Some(max_courant),
            if recommended_dt.is_infinite() { None } else { Some(recommended_dt) }
        )
    } else {
        (None, None)
    };

    // Prepare time hydrographs in metric
    let mut q_up_hydrograph = vec![0.0; inputs.num_steps];
    let mut y_down_hydrograph = vec![0.0; inputs.num_steps];
    for step in 0..inputs.num_steps {
        q_up_hydrograph[step] = if raw_units == UnitSystem::USCustomary {
            inputs.upstream_q_hydrograph[step] * crate::utils::CFS_TO_CMS
        } else {
            inputs.upstream_q_hydrograph[step]
        };
        y_down_hydrograph[step] = if raw_units == UnitSystem::USCustomary {
            inputs.downstream_wsel_hydrograph[step] * FT_TO_M
        } else {
            inputs.downstream_wsel_hydrograph[step]
        };
    }

    // Enforce initial WSEL clamping to prevent starting with dry/negative depth, and stabilize initial Q
    for k in 0..dm {
        let min_wsel = densified_z_mins[k] + 0.05;
        if densified_y_current[k] < min_wsel {
            densified_y_current[k] = min_wsel;
        }
        let row = geometry_row_at_elevation(
            &densified_tables[k],
            Some(&densified_xs[k]),
            densified_y_current[k],
            None,
            None,
        );
        let area = row.area.max(1e-6);
        let depth = (densified_y_current[k] - densified_z_mins[k]).max(0.0);
        let max_phys_vel = 15.0 * (depth / 0.1).min(1.0).max(0.1);
        let max_q = area * max_phys_vel;
        densified_q_current[k] = densified_q_current[k].clamp(-max_q, max_q);
    }

    let mut history_wsel = Vec::new();
    let mut history_q = Vec::new();
    let mut history_vel = Vec::new();
    let mut history_culvert_control_types: Option<Vec<Vec<String>>> =
        track_culvert_diagnostics.then(|| Vec::with_capacity(inputs.num_steps));
    let mut history_culvert_wsel_inlet: Option<Vec<Vec<f64>>> =
        track_culvert_diagnostics.then(|| Vec::with_capacity(inputs.num_steps));
    let mut history_culvert_wsel_outlet: Option<Vec<Vec<f64>>> =
        track_culvert_diagnostics.then(|| Vec::with_capacity(inputs.num_steps));
    let mut history_culvert_q_barrels: Option<Vec<Vec<f64>>> =
        track_culvert_diagnostics.then(|| Vec::with_capacity(inputs.num_steps));
    let mut history_culvert_q_weirs: Option<Vec<Vec<f64>>> =
        track_culvert_diagnostics.then(|| Vec::with_capacity(inputs.num_steps));
    let mut history_culvert_barrel_depths: Option<Vec<Vec<f64>>> =
        track_culvert_diagnostics.then(|| Vec::with_capacity(inputs.num_steps));
    let mut history_culvert_barrel_velocities: Option<Vec<Vec<f64>>> =
        track_culvert_diagnostics.then(|| Vec::with_capacity(inputs.num_steps));
    let mut history_culvert_barrel_froude: Option<Vec<Vec<f64>>> =
        track_culvert_diagnostics.then(|| Vec::with_capacity(inputs.num_steps));
    let mut history_bridge_flow_regimes: Option<Vec<Vec<String>>> =
        track_bridge_diagnostics.then(|| Vec::with_capacity(inputs.num_steps));
    let mut history_bridge_wsel_upstream: Option<Vec<Vec<f64>>> =
        track_bridge_diagnostics.then(|| Vec::with_capacity(inputs.num_steps));
    let mut history_bridge_wsel_downstream: Option<Vec<Vec<f64>>> =
        track_bridge_diagnostics.then(|| Vec::with_capacity(inputs.num_steps));
    let mut history_bridge_head_losses: Option<Vec<Vec<f64>>> =
        track_bridge_diagnostics.then(|| Vec::with_capacity(inputs.num_steps));

    // Loop through time steps
    for step in 0..inputs.num_steps {
        let q_up_next = q_up_hydrograph[step];
        let mut y_down_next = y_down_hydrograph[step];

        // Clamp downstream stage BC to prevent dry downstream boundary
        let ds_z_min = densified_z_mins[dm - 1];
        if y_down_next < ds_z_min + 0.05 {
            y_down_next = ds_z_min + 0.05;
        }

        let step_params = PreissmannStepParams {
            tables: &densified_tables,
            xs_list: &densified_xs,
            z_mins: &densified_z_mins,
            y_current: &densified_y_current,
            q_current: &densified_q_current,
            dt,
            q_up_next,
            y_down_next,
            theta,
            c_contraction,
            c_expansion,
            structure_coupling_mode,
            culvert_intervals: &culvert_intervals,
            bridge_intervals: &bridge_intervals,
            unsteady_inputs: Some(&inputs),
            raw_units,
            #[cfg(test)]
            implicit_hook_probe: None,
        };

        if let Some((y_next, q_next)) = preissmann::solve_preissmann_step(&step_params) {
            densified_y_current = y_next;
            densified_q_current = q_next;

             let structure_step_results = structure_coupling::apply_structure_internal_boundaries(
                &inputs,
                raw_units,
                &densified_tables,
                &densified_xs,
                &densified_z_mins,
                &densified_stations,
                &mut densified_y_current,
                &densified_q_current,
                &culvert_intervals,
                &bridge_intervals,
            );

            if let (Some(step_results), Some(ctrl)) = (
                structure_step_results.culvert.as_ref(),
                history_culvert_control_types.as_mut(),
            ) {
                ctrl.push(step_results.iter().map(|r| r.control_type.clone()).collect());
                history_culvert_wsel_inlet
                    .as_mut()
                    .unwrap()
                    .push(step_results.iter().map(|r| r.wsel_inlet).collect());
                history_culvert_wsel_outlet
                    .as_mut()
                    .unwrap()
                    .push(step_results.iter().map(|r| r.wsel_outlet).collect());
                history_culvert_q_barrels
                    .as_mut()
                    .unwrap()
                    .push(step_results.iter().map(|r| r.q_barrel).collect());
                history_culvert_q_weirs
                    .as_mut()
                    .unwrap()
                    .push(step_results.iter().map(|r| r.q_weir).collect());
                history_culvert_barrel_depths
                    .as_mut()
                    .unwrap()
                    .push(step_results.iter().map(|r| r.barrel_depth).collect());
                history_culvert_barrel_velocities
                    .as_mut()
                    .unwrap()
                    .push(step_results.iter().map(|r| r.barrel_velocity).collect());
                history_culvert_barrel_froude
                    .as_mut()
                    .unwrap()
                    .push(step_results.iter().map(|r| r.barrel_froude).collect());
            }

            if let (Some(step_results), Some(regimes)) = (
                structure_step_results.bridge.as_ref(),
                history_bridge_flow_regimes.as_mut(),
            ) {
                regimes.push(step_results.iter().map(|r| r.flow_regime.clone()).collect());
                history_bridge_wsel_upstream
                    .as_mut()
                    .unwrap()
                    .push(step_results.iter().map(|r| r.wsel_up).collect());
                history_bridge_wsel_downstream
                    .as_mut()
                    .unwrap()
                    .push(step_results.iter().map(|r| r.wsel_down).collect());
                history_bridge_head_losses
                    .as_mut()
                    .unwrap()
                    .push(step_results.iter().map(|r| r.head_loss).collect());
            }

            // Clamp solved WSEL to prevent dry nodes/negative depth, and limit velocity
            for k in 0..dm {
                let min_wsel = densified_z_mins[k] + 0.05;
                densified_y_current[k] = densified_y_current[k].max(min_wsel);
                let row = geometry_row_at_elevation(
                    &densified_tables[k],
                    Some(&densified_xs[k]),
                    densified_y_current[k],
                    None,
                    None,
                );
                let area = row.area.max(1e-6);
                let depth = (densified_y_current[k] - densified_z_mins[k]).max(0.0);

                if k > 0 {
                    let max_phys_vel = 15.0 * (depth / 0.1).min(1.0).max(0.1);
                    let max_q = area * max_phys_vel;
                    densified_q_current[k] = densified_q_current[k].clamp(-max_q, max_q);
                } else {
                    densified_q_current[0] = q_up_next;
                }
            }
            // Enforce downstream boundary stage exactly
            densified_y_current[dm - 1] = y_down_next;
        } else {
            // If the matrix solver fails to invert (rare), maintain current state as fallback
        }

        // Convert current step back to user units and original layout
        let mut step_wsel = vec![0.0; m];
        let mut step_q = vec![0.0; m];
        let mut step_vel = vec![0.0; m];

        for orig_idx in 0..m {
            let sorted_xs_idx = original_mapping[orig_idx];
            let sorted_idx = original_to_densified[sorted_xs_idx];
            let w_val = densified_y_current[sorted_idx];
            let q_val = densified_q_current[sorted_idx];
            
            let row = geometry_row_at_elevation(
                &densified_tables[sorted_idx],
                Some(&densified_xs[sorted_idx]),
                w_val,
                None,
                None,
            );
            let flow_area = flow_area_for_row(&row);
            let vel_val = if flow_area > 1e-6 { q_val / flow_area } else { 0.0 };

            if raw_units == UnitSystem::USCustomary {
                step_wsel[orig_idx] = w_val / FT_TO_M;
                step_q[orig_idx] = q_val / crate::utils::CFS_TO_CMS;
                step_vel[orig_idx] = vel_val / FT_TO_M;
            } else {
                step_wsel[orig_idx] = w_val;
                step_q[orig_idx] = q_val;
                step_vel[orig_idx] = vel_val;
            }
        }

        history_wsel.push(step_wsel);
        history_q.push(step_q);
        history_vel.push(step_vel);
    }

    UnsteadyResult {
        wsel: history_wsel,
        q: history_q,
        velocity: history_vel,
        max_courant: max_courant_val,
        recommended_dt: recommended_dt_val,
        culvert_control_types: history_culvert_control_types,
        culvert_wsel_inlet: history_culvert_wsel_inlet,
        culvert_wsel_outlet: history_culvert_wsel_outlet,
        culvert_q_barrels: history_culvert_q_barrels,
        culvert_q_weirs: history_culvert_q_weirs,
        culvert_barrel_depths: history_culvert_barrel_depths,
        culvert_barrel_velocities: history_culvert_barrel_velocities,
        culvert_barrel_froude: history_culvert_barrel_froude,
        bridge_flow_regimes: history_bridge_flow_regimes,
        bridge_wsel_upstream: history_bridge_wsel_upstream,
        bridge_wsel_downstream: history_bridge_wsel_downstream,
        bridge_head_losses: history_bridge_head_losses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsteady_stability() {
        // Set up 3 cross-sections spaced 500m apart (total 1000m length).
        // Rectangular channel: width = 10m, Manning's n = 0.02.
        // Stationing: 1000, 500, 0.
        // Bed elevations: 1.0, 0.5, 0.0.
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0 + 1.0, 1.0, 1.0, 5.0 + 1.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs500 = CrossSection {
            station: 500.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0 + 0.5, 0.5, 0.5, 5.0 + 0.5],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        // Run a simulation keeping inputs constant at 14.0 cms (uniform flow equilibrium depth = 1.0m) and WSEL = 1.0m downstream
        let inputs = UnsteadyInputs {
            cross_sections: vec![xs1000, xs500, xs0],
            initial_wsel: vec![2.0, 1.5, 1.0], // constant depth = 1.0m
            initial_q: vec![14.0, 14.0, 14.0],
            dt: 60.0,
            num_steps: 5,
            upstream_q_hydrograph: vec![14.0; 5],
            downstream_wsel_hydrograph: vec![1.0; 5],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: None,
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs::default(),
            bridge: UnsteadyBridgeInputs::default(),
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };

        let result = solve_unsteady(&inputs);

        // Result assertions
        assert_eq!(result.wsel.len(), 5);
        assert_eq!(result.q.len(), 5);

        // Verify that the flow rates Q remain close to 14.0 cms over the simulation
        for step in 0..5 {
            for node in 0..3 {
                let q_val = result.q[step][node];
                assert!((q_val - 14.0).abs() < 1e-1, "Step {} Node {} Q was {}", step, node, q_val);
            }
        }
    }

    #[test]
    fn test_unsteady_reach_densification() {
        // Set up 2 cross-sections spaced 1000m apart.
        // Bed slope is 0.001 (z1 = 1.0m, z2 = 0.0m).
        // Rectangular channel: width = 10m.
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![6.0, 1.0, 1.0, 6.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        // Run with a max spacing of 100.0m (which should create 9 intermediate cross sections, total 11 sections internally)
        let inputs = UnsteadyInputs {
            cross_sections: vec![xs1000, xs0],
            initial_wsel: vec![2.0, 1.0], // constant depth = 1.0m
            initial_q: vec![14.0, 14.0],
            dt: 10.0,
            num_steps: 5,
            upstream_q_hydrograph: vec![14.0; 5],
            downstream_wsel_hydrograph: vec![1.0; 5],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: Some(100.0),
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs::default(),
            bridge: UnsteadyBridgeInputs::default(),
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };

        let result = solve_unsteady(&inputs);

        // Verification
        // The solver should converge successfully. Check that output size matches original input size (2)
        assert_eq!(result.wsel.len(), 5);
        assert_eq!(result.wsel[0].len(), 2);
        
        // Downstream boundary condition is preserved at the end of the reach
        assert!((result.wsel[4][1] - 1.0).abs() < 1e-1);

        // Check that max_courant and recommended_dt are calculated
        assert!(result.max_courant.is_some());
        assert!(result.recommended_dt.is_some());
        
        let cr = result.max_courant.unwrap();
        assert!(cr > 0.0, "max_courant was {}", cr);
    }

    #[test]
    fn unsteady_blocked_densify_profile_matches_explicit_station_grid() {
        use crate::geometry::BlockedObstruction;

        let section = |station: f64, z: f64| -> CrossSection {
            CrossSection {
                station,
                x: vec![0.0, 0.0, 10.0, 10.0],
                y: vec![5.0 + z, z, z, 5.0 + z],
                n_stations: vec![0.0],
                n_values: vec![0.02],
                unit_system: UnitSystem::Metric,
                is_overbank: None,
                blocked_obstructions: Some(vec![BlockedObstruction {
                    stations: vec![3.0, 7.0],
                    elevations: vec![1.5, 1.5],
                }]),
                ineffective_flow_areas: None,
                guide_banks: None,
            }
        };

        let sparse = UnsteadyInputs {
            cross_sections: vec![section(200.0, 0.1), section(0.0, 0.0)],
            initial_wsel: vec![2.0, 1.8],
            initial_q: vec![14.0, 14.0],
            dt: 10.0,
            num_steps: 3,
            upstream_q_hydrograph: vec![14.0; 3],
            downstream_wsel_hydrograph: vec![1.8; 3],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: Some(50.0),
            densify_reach_modifier_policy: Some(1),
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs::default(),
            bridge: UnsteadyBridgeInputs::default(),
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };
        let dense = UnsteadyInputs {
            cross_sections: vec![
                section(200.0, 0.1),
                section(150.0, 0.075),
                section(100.0, 0.05),
                section(50.0, 0.025),
                section(0.0, 0.0),
            ],
            initial_wsel: vec![2.0, 1.9, 1.85, 1.82, 1.8],
            initial_q: vec![14.0; 5],
            dt: 10.0,
            num_steps: 3,
            upstream_q_hydrograph: vec![14.0; 3],
            downstream_wsel_hydrograph: vec![1.8; 3],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: None,
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs::default(),
            bridge: UnsteadyBridgeInputs::default(),
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };

        let r_sparse = solve_unsteady(&sparse);
        let r_dense = solve_unsteady(&dense);
        let last = r_sparse.wsel.len() - 1;
        assert!(
            (r_sparse.wsel[last][0] - r_dense.wsel[last][0]).abs() < 0.05,
            "unsteady upstream WSEL sparse {} vs explicit {}",
            r_sparse.wsel[last][0],
            r_dense.wsel[last][0]
        );
        assert!(
            (r_sparse.wsel[last][1] - r_dense.wsel[last][4]).abs() < 0.05,
            "unsteady downstream WSEL sparse {} vs explicit {}",
            r_sparse.wsel[last][1],
            r_dense.wsel[last][4]
        );
    }

    #[test]
    fn unsteady_densify_downstream_policy_runs() {
        use crate::geometry::IneffectiveFlowAreas;

        let xs_us = CrossSection {
            station: 200.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.1, 0.1, 0.1, 5.1],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: Some(
                IneffectiveFlowAreas::from_block_pairs(&[], &[], &[9.0], &[2.5]).unwrap(),
            ),
            guide_banks: None,
        };
        let xs_ds = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: Some(
                IneffectiveFlowAreas::from_block_pairs(&[], &[], &[2.0], &[2.5]).unwrap(),
            ),
            guide_banks: None,
        };
        let inputs = UnsteadyInputs {
            cross_sections: vec![xs_us, xs_ds],
            initial_wsel: vec![2.0, 1.8],
            initial_q: vec![14.0, 14.0],
            dt: 10.0,
            num_steps: 2,
            upstream_q_hydrograph: vec![14.0; 2],
            downstream_wsel_hydrograph: vec![1.8; 2],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: Some(50.0),
            densify_reach_modifier_policy: Some(2),
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs::default(),
            bridge: UnsteadyBridgeInputs::default(),
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };
        let result = solve_unsteady(&inputs);
        assert_eq!(result.wsel.len(), 2);
        assert!(result.wsel[1][0].is_finite());
    }

    #[test]
    fn unsteady_bridge_layout_without_explicit_faces() {
        let xs200 = CrossSection {
            station: 200.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.1, 0.1, 0.1, 5.1],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let inputs = UnsteadyInputs {
            cross_sections: vec![xs200, xs0],
            initial_wsel: vec![2.0, 1.8],
            initial_q: vec![14.0, 14.0],
            dt: 10.0,
            num_steps: 2,
            upstream_q_hydrograph: vec![14.0; 2],
            downstream_wsel_hydrograph: vec![1.8; 2],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: Some(50.0),
            densify_reach_modifier_policy: Some(1),
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs::default(),
            bridge: UnsteadyBridgeInputs {
                bridge_stations: Some(vec![100.0]),
                bridge_lengths: Some(vec![10.0]),
                bridge_low_chords: Some(vec![5.0]),
                bridge_high_chords: Some(vec![7.0]),
                bridge_weir_coeffs: Some(vec![1.44]),
                bridge_orifice_coeffs: Some(vec![0.5]),
                bridge_ineffective_left_stations_upstream: Some(vec![vec![5.0]]),
                bridge_ineffective_left_elevations_upstream: Some(vec![vec![3.0]]),
                ..Default::default()
            },
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };
        let result = solve_unsteady(&inputs);
        assert_eq!(result.wsel.len(), 2);
        assert!(result.wsel[1][0] > result.wsel[1][1]);
    }

    #[test]
    fn test_project_11_debug() {
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 12.0, 22.0, 34.0],
            y: vec![26.0, 20.0, 20.0, 26.0],
            n_stations: vec![0.0],
            n_values: vec![0.025],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs750 = CrossSection {
            station: 750.0,
            x: vec![0.0, 12.0, 18.0, 22.0, 34.0],
            y: vec![23.5, 17.5, 17.0, 17.5, 23.5],
            n_stations: vec![0.0],
            n_values: vec![0.025; 5],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs500 = CrossSection {
            station: 500.0,
            x: vec![0.0, 12.0, 22.0, 34.0],
            y: vec![21.0, 15.0, 15.0, 21.0],
            n_stations: vec![0.0],
            n_values: vec![0.025],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs250 = CrossSection {
            station: 250.0,
            x: vec![0.0, 12.0, 22.0, 34.0],
            y: vec![18.5, 12.5, 12.5, 18.5],
            n_stations: vec![0.0],
            n_values: vec![0.025],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs150 = CrossSection {
            station: 150.0,
            x: vec![0.0, 12.0, 22.0, 34.0],
            y: vec![18.5, 12.5, 12.5, 18.5],
            n_stations: vec![0.0],
            n_values: vec![0.025],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 12.0, 22.0, 34.0],
            y: vec![16.0, 10.0, 10.0, 16.0],
            n_stations: vec![0.0],
            n_values: vec![0.025],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let mut upstream_q = vec![35.0; 100];
        for i in 0..30 {
            upstream_q[i] = 35.0 + (87.5 - 35.0) * (i as f64 / 30.0);
        }
        for i in 30..100 {
            upstream_q[i] = 87.5 - (87.5 - 35.0) * ((i - 30) as f64 / 70.0);
        }

        let inputs = UnsteadyInputs {
            cross_sections: vec![xs1000, xs750, xs500, xs250, xs150, xs0],
            initial_wsel: vec![21.0, 18.0, 16.0, 13.5, 13.5, 12.0],
            initial_q: vec![35.0; 6],
            dt: 10.0,
            num_steps: 100,
            upstream_q_hydrograph: upstream_q,
            downstream_wsel_hydrograph: vec![12.0; 100],
            theta: Some(1.0),
            num_slices: Some(100),
            max_spacing: None,
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs::default(),
            bridge: UnsteadyBridgeInputs::default(),
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };

        let result = solve_unsteady(&inputs);
        println!("Recommended DT = {:?}", result.recommended_dt);
        for step in (0..100).step_by(10) {
            println!("Step {}: WSEL = {:?}", step, result.wsel[step]);
            println!("Step {}: Q    = {:?}", step, result.q[step]);
        }
    }

    #[test]
    fn test_unsteady_inline_culvert() {
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![6.0, 1.0, 1.0, 6.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs500 = CrossSection {
            station: 500.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.5, 0.5, 0.5, 5.5],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let without = UnsteadyInputs {
            cross_sections: vec![xs1000.clone(), xs500.clone(), xs0.clone()],
            initial_wsel: vec![2.5, 2.0, 1.5],
            initial_q: vec![20.0, 20.0, 20.0],
            dt: 60.0,
            num_steps: 3,
            upstream_q_hydrograph: vec![20.0; 3],
            downstream_wsel_hydrograph: vec![1.5; 3],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: None,
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs::default(),
            bridge: UnsteadyBridgeInputs::default(),
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };

        let with_culvert = UnsteadyInputs {
            culvert: UnsteadyCulvertInputs {
                culvert_stations: Some(vec![250.0]),
                culvert_shape_types: Some(vec![0]),
                culvert_spans: Some(vec![2.0]),
                culvert_rises: Some(vec![2.0]),
                culvert_roughness_ns: Some(vec![0.013]),
                culvert_lengths: Some(vec![30.0]),
                culvert_entrance_loss_coeffs: Some(vec![0.5]),
                culvert_exit_loss_coeffs: Some(vec![1.0]),
                culvert_barrels: Some(vec![1]),
                culvert_inlet_types: Some(vec![1]),
                ..Default::default()
            },
            ..without.clone()
        };

        let res_plain = solve_unsteady(&without);
        let res_culvert = solve_unsteady(&with_culvert);

        assert_eq!(res_culvert.wsel.len(), 3);
        assert!(res_culvert.wsel[2][0].is_finite());
        assert!(
            res_culvert.wsel[2][0] > res_plain.wsel[2][0],
            "culvert headwater should exceed plain channel upstream WSEL"
        );
        let ctrl = res_culvert
            .culvert_control_types
            .as_ref()
            .expect("culvert diagnostics on unsteady run");
        assert_eq!(ctrl.len(), 3);
        assert_eq!(ctrl[0].len(), 1);
        assert!(ctrl[0][0] == "inlet" || ctrl[0][0] == "outlet" || ctrl[0][0] == "overtopping");
        assert!(res_culvert.culvert_q_barrels.as_ref().unwrap()[2][0] > 0.0);
    }

    #[test]
    fn test_unsteady_inline_culvert_us_customary() {
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![6.0, 1.0, 1.0, 6.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs500 = CrossSection {
            station: 500.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.5, 0.5, 0.5, 5.5],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let inputs = UnsteadyInputs {
            cross_sections: vec![xs1000, xs500, xs0],
            initial_wsel: vec![8.0, 6.5, 5.0],
            initial_q: vec![70.0, 70.0, 70.0],
            dt: 60.0,
            num_steps: 3,
            upstream_q_hydrograph: vec![70.0; 3],
            downstream_wsel_hydrograph: vec![5.0; 3],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: None,
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs {
                culvert_stations: Some(vec![250.0]),
                culvert_shape_types: Some(vec![0]),
                culvert_spans: Some(vec![6.0]),
                culvert_rises: Some(vec![6.0]),
                culvert_roughness_ns: Some(vec![0.013]),
                culvert_lengths: Some(vec![100.0]),
                culvert_entrance_loss_coeffs: Some(vec![0.5]),
                culvert_exit_loss_coeffs: Some(vec![1.0]),
                culvert_barrels: Some(vec![1]),
                culvert_inlet_types: Some(vec![1]),
                culvert_z_ups: Some(vec![6.0]),
                culvert_z_downs: Some(vec![5.0]),
                ..Default::default()
            },
            bridge: UnsteadyBridgeInputs::default(),
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };

        let result = solve_unsteady(&inputs);
        assert_eq!(result.wsel.len(), 3);
        assert!(result.wsel[2].iter().all(|w| w.is_finite()));
        assert!(result.q[2].iter().all(|q| q.is_finite()));
    }

    #[test]
    fn test_unsteady_inline_bridge() {
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![6.0, 1.0, 1.0, 6.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs500 = CrossSection {
            station: 500.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.5, 0.5, 0.5, 5.5],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let without = UnsteadyInputs {
            cross_sections: vec![xs1000.clone(), xs500.clone(), xs0.clone()],
            initial_wsel: vec![2.5, 2.0, 1.5],
            initial_q: vec![15.0, 15.0, 15.0],
            dt: 60.0,
            num_steps: 3,
            upstream_q_hydrograph: vec![15.0; 3],
            downstream_wsel_hydrograph: vec![1.5; 3],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: None,
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs::default(),
            bridge: UnsteadyBridgeInputs::default(),
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };

        let with_bridge = UnsteadyInputs {
            bridge: UnsteadyBridgeInputs {
                bridge_stations: Some(vec![250.0]),
                bridge_low_chords: Some(vec![5.0]),
                bridge_high_chords: Some(vec![7.0]),
                bridge_pier_widths: Some(vec![0.5]),
                bridge_num_piers: Some(vec![2]),
                bridge_pier_shapes: Some(vec![0]),
                bridge_weir_coeffs: Some(vec![1.44]),
                bridge_orifice_coeffs: Some(vec![0.5]),
                ..Default::default()
            },
            ..without.clone()
        };

        let res_bridge = solve_unsteady(&with_bridge);

        assert_eq!(res_bridge.wsel.len(), 3);
        assert!(res_bridge.wsel[2].iter().all(|w| w.is_finite()));

        let regimes = res_bridge
            .bridge_flow_regimes
            .as_ref()
            .expect("bridge diagnostics on unsteady run");
        let hw = res_bridge
            .bridge_wsel_upstream
            .as_ref()
            .expect("bridge upstream WSEL history")[2][0];
        let tw = res_bridge
            .bridge_wsel_downstream
            .as_ref()
            .expect("bridge downstream WSEL history")[2][0];
        let hl = res_bridge
            .bridge_head_losses
            .as_ref()
            .expect("bridge head loss history")[2][0];

        assert_eq!(regimes.len(), 3);
        assert_eq!(regimes[0].len(), 1);
        assert_eq!(regimes[2][0], "low_a");
        assert!(hl > 0.0, "expected positive pier head loss, got {hl}");
        assert!(hw > tw, "upstream bridge WSEL {hw} should exceed tailwater {tw}");
        assert!((hw - tw - hl).abs() < 1e-4);
    }

    #[test]
    fn test_unsteady_inline_bridge_negative_q() {
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![6.0, 1.0, 1.0, 6.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let xs500 = CrossSection {
            station: 500.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.5, 0.5, 0.5, 5.5],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };

        let base = UnsteadyInputs {
            cross_sections: vec![xs1000.clone(), xs500.clone(), xs0.clone()],
            initial_wsel: vec![2.5, 2.0, 1.5],
            initial_q: vec![15.0, 15.0, 15.0],
            dt: 60.0,
            num_steps: 3,
            upstream_q_hydrograph: vec![15.0; 3],
            downstream_wsel_hydrograph: vec![1.5; 3],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: None,
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs::default(),
            bridge: UnsteadyBridgeInputs {
                bridge_stations: Some(vec![250.0]),
                bridge_low_chords: Some(vec![5.0]),
                bridge_high_chords: Some(vec![7.0]),
                bridge_low_flow_methods: Some(vec![3]),
                ..Default::default()
            },
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };
        let forward = solve_unsteady(&base);
        let reverse = solve_unsteady(&UnsteadyInputs {
            initial_q: vec![-15.0, -15.0, -15.0],
            upstream_q_hydrograph: vec![-15.0; 3],
            cross_sections: vec![xs1000, xs500, xs0],
            ..base
        });

        assert!(reverse.wsel.iter().flatten().all(|w| w.is_finite()));
        let rev_hl = reverse
            .bridge_head_losses
            .as_ref()
            .expect("bridge diagnostics")[2][0];
        let fwd_hl = forward
            .bridge_head_losses
            .as_ref()
            .expect("bridge diagnostics")[2][0];
        assert!(rev_hl > 0.0, "reverse-flow bridge head loss should be positive");
        assert!(
            (fwd_hl - rev_hl).abs() < 0.02,
            "symmetric |Q|=15 should yield similar head loss, got forward {fwd_hl} reverse {rev_hl}"
        );
    }

    #[test]
    fn test_unsteady_inline_bridge_q_reversal_hydrograph() {
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![6.0, 1.0, 1.0, 6.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let xs500 = CrossSection {
            station: 500.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.5, 0.5, 0.5, 5.5],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };

        let result = solve_unsteady(&UnsteadyInputs {
            cross_sections: vec![xs1000, xs500, xs0],
            initial_wsel: vec![2.5, 2.0, 1.5],
            initial_q: vec![15.0, 15.0, 15.0],
            dt: 60.0,
            num_steps: 4,
            upstream_q_hydrograph: vec![15.0, 15.0, -15.0, -15.0],
            downstream_wsel_hydrograph: vec![1.5; 4],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: None,
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs::default(),
            bridge: UnsteadyBridgeInputs {
                bridge_stations: Some(vec![250.0]),
                bridge_low_chords: Some(vec![5.0]),
                bridge_high_chords: Some(vec![7.0]),
                bridge_low_flow_methods: Some(vec![3]),
                ..Default::default()
            },
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        });

        assert_eq!(result.q.len(), 4);
        assert!(result.q[1][0] > 0.0);
        assert!(result.q[3][0] < 0.0);
        let hl_fwd = result.bridge_head_losses.as_ref().unwrap()[1][0];
        let hl_rev = result.bridge_head_losses.as_ref().unwrap()[3][0];
        assert!(hl_fwd > 0.0);
        assert!(hl_rev > 0.0);
    }

    #[test]
    fn test_unsteady_step_ineffective_reduces_conveyance() {
        use crate::geometry::IneffectiveFlowAreas;

        fn compound_channel(station: f64, z_bottom: f64) -> CrossSection {
            CrossSection {
                station,
                x: vec![0.0, 0.0, 10.0, 10.0, 30.0, 30.0, 40.0, 40.0],
                y: vec![
                    5.0 + z_bottom,
                    z_bottom,
                    z_bottom,
                    5.0 + z_bottom,
                    z_bottom,
                    5.0 + z_bottom,
                    z_bottom,
                    5.0 + z_bottom,
                ],
                n_stations: vec![0.0, 10.0],
                n_values: vec![0.03, 0.05],
                unit_system: UnitSystem::Metric,
                is_overbank: Some(vec![
                    false, false, false, false, true, true, true, true,
                ]),
                blocked_obstructions: None,
                ineffective_flow_areas: None,
                guide_banks: None,
            }
        }

        let xs_ds = compound_channel(0.0, 0.0);
        let mut xs_us_ineff = compound_channel(100.0, 0.0);
        xs_us_ineff.ineffective_flow_areas = Some(
            IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap(),
        );
        let xs_us_open = compound_channel(100.0, 0.0);

        let make_tables = |xs_us: &CrossSection| {
            let tables = vec![
                xs_us.generate_lookup_table(50),
                xs_ds.generate_lookup_table(50),
            ];
            let z_mins = vec![
                xs_us.y.iter().cloned().fold(f64::INFINITY, f64::min),
                xs_ds.y.iter().cloned().fold(f64::INFINITY, f64::min),
            ];
            (tables, z_mins)
        };

        let y0 = vec![2.0, 2.0];
        let q0 = vec![30.0, 30.0];
        let dt = 60.0;

        let (tables_open, z_open) = make_tables(&xs_us_open);
        let open = solve_unsteady_step(
            &tables_open,
            &[xs_us_open.clone(), xs_ds.clone()],
            &z_open,
            &y0,
            &q0,
            dt,
            30.0,
            2.0,
            0.6,
            0.1,
            0.3,
        )
        .expect("open step");

        let (tables_ineff, z_ineff) = make_tables(&xs_us_ineff);
        let ineffective = solve_unsteady_step(
            &tables_ineff,
            &[xs_us_ineff, xs_ds],
            &z_ineff,
            &y0,
            &q0,
            dt,
            30.0,
            2.0,
            0.6,
            0.1,
            0.3,
        )
        .expect("ineffective step");

        assert!(
            ineffective.0[0] > open.0[0],
            "upstream stage should rise when overbank conveyance is clipped: open={}, ineffective={}",
            open.0[0],
            ineffective.0[0],
        );
        assert!(
            (ineffective.0[1] - 2.0).abs() < 1e-3,
            "downstream BC should be enforced, got {}",
            ineffective.0[1],
        );
    }

    #[test]
    fn test_build_coupled_structure_order_modes() {
        let culvert_intervals = vec![(2, 0), (5, 1)];
        let bridge_intervals = vec![(4, 0), (7, 1)];

        use structure_coupling::{build_coupled_structure_order, StructureCouplingOrder, StructureKind};

        let combined = build_coupled_structure_order(
            &culvert_intervals,
            &bridge_intervals,
            StructureCouplingOrder::CombinedDownstream,
        );
        assert_eq!(combined.len(), 4);
        assert_eq!(combined[0].interval_i, 7);
        assert_eq!(combined[0].kind, StructureKind::Bridge);
        assert_eq!(combined[1].interval_i, 5);
        assert_eq!(combined[1].kind, StructureKind::Culvert);
        assert_eq!(combined[2].interval_i, 4);
        assert_eq!(combined[2].kind, StructureKind::Bridge);
        assert_eq!(combined[3].interval_i, 2);
        assert_eq!(combined[3].kind, StructureKind::Culvert);

        let culverts_first = build_coupled_structure_order(
            &culvert_intervals,
            &bridge_intervals,
            StructureCouplingOrder::CulvertsFirst,
        );
        assert_eq!(culverts_first[0].kind, StructureKind::Culvert);
        assert_eq!(culverts_first[1].kind, StructureKind::Culvert);
        assert_eq!(culverts_first[2].kind, StructureKind::Bridge);
        assert_eq!(culverts_first[3].kind, StructureKind::Bridge);

        let bridges_first = build_coupled_structure_order(
            &culvert_intervals,
            &bridge_intervals,
            StructureCouplingOrder::BridgesFirst,
        );
        assert_eq!(bridges_first[0].kind, StructureKind::Bridge);
        assert_eq!(bridges_first[2].kind, StructureKind::Culvert);
    }

    #[test]
    fn test_structure_coupling_order_mixed_reach() {
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![6.0, 1.0, 1.0, 6.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs500 = CrossSection {
            station: 500.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.5, 0.5, 0.5, 5.5],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
        };

        let base = UnsteadyInputs {
            cross_sections: vec![xs1000, xs500, xs0],
            initial_wsel: vec![2.5, 2.0, 1.5],
            initial_q: vec![20.0, 20.0, 20.0],
            dt: 60.0,
            num_steps: 3,
            upstream_q_hydrograph: vec![20.0; 3],
            downstream_wsel_hydrograph: vec![1.5; 3],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: None,
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs {
                culvert_stations: Some(vec![750.0]),
                culvert_shape_types: Some(vec![0]),
                culvert_spans: Some(vec![2.0]),
                culvert_rises: Some(vec![2.0]),
                culvert_roughness_ns: Some(vec![0.013]),
                culvert_lengths: Some(vec![30.0]),
                culvert_entrance_loss_coeffs: Some(vec![0.5]),
                culvert_exit_loss_coeffs: Some(vec![1.0]),
                culvert_barrels: Some(vec![1]),
                culvert_inlet_types: Some(vec![1]),
                ..Default::default()
            },
            bridge: UnsteadyBridgeInputs {
                bridge_stations: Some(vec![250.0]),
                bridge_low_chords: Some(vec![5.0]),
                bridge_high_chords: Some(vec![7.0]),
                bridge_pier_widths: Some(vec![0.5]),
                bridge_num_piers: Some(vec![2]),
                bridge_pier_shapes: Some(vec![0]),
                bridge_weir_coeffs: Some(vec![1.44]),
                bridge_orifice_coeffs: Some(vec![0.5]),
                ..Default::default()
            },
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };

        let combined = solve_unsteady(&base);
        let mut culverts_first_inputs = base.clone();
        culverts_first_inputs.structure_coupling_order = Some(1);
        let culverts_first = solve_unsteady(&culverts_first_inputs);
        let mut bridges_first_inputs = base.clone();
        bridges_first_inputs.structure_coupling_order = Some(2);
        let bridges_first = solve_unsteady(&bridges_first_inputs);

        for res in [&combined, &culverts_first, &bridges_first] {
            assert!(res.wsel[2].iter().all(|w| w.is_finite()));
            assert!(
                res.bridge_flow_regimes.is_some(),
                "bridge diagnostics should be present"
            );
            assert!(
                res.culvert_control_types.is_some(),
                "culvert diagnostics should be present"
            );
        }

        // Post-step coupling iterates to the same fixed point regardless of pass order.
        assert_eq!(
            combined.wsel[2], culverts_first.wsel[2],
            "converged coupling should be order-independent (combined vs culverts-first)"
        );
        assert_eq!(
            combined.wsel[2], bridges_first.wsel[2],
            "converged coupling should be order-independent (combined vs bridges-first)"
        );
    }

    #[test]
    fn test_unsteady_implicit_coupling_mode_stub_matches_default() {
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![6.0, 1.0, 1.0, 6.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let xs500 = CrossSection {
            station: 500.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.5, 0.5, 0.5, 5.5],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };

        let base = UnsteadyInputs {
            cross_sections: vec![xs1000, xs500, xs0],
            initial_wsel: vec![2.5, 2.0, 1.5],
            initial_q: vec![15.0, 15.0, 15.0],
            dt: 60.0,
            num_steps: 3,
            upstream_q_hydrograph: vec![15.0; 3],
            downstream_wsel_hydrograph: vec![1.5; 3],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: None,
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs::default(),
            bridge: UnsteadyBridgeInputs {
                bridge_stations: Some(vec![250.0]),
                bridge_low_chords: Some(vec![5.0]),
                bridge_high_chords: Some(vec![7.0]),
                bridge_pier_widths: Some(vec![0.5]),
                bridge_num_piers: Some(vec![2]),
                bridge_pier_shapes: Some(vec![0]),
                bridge_weir_coeffs: Some(vec![1.44]),
                bridge_orifice_coeffs: Some(vec![0.5]),
                ..Default::default()
            },
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: None,
        };

        let default_run = solve_unsteady(&base);
        let mut implicit = base;
        implicit.unsteady_structure_coupling_mode = Some(2);
        let implicit_run = solve_unsteady(&implicit);

        assert_eq!(default_run.wsel, implicit_run.wsel);
        assert_eq!(default_run.q, implicit_run.q);
    }

    fn inline_culvert_metric_xs() -> (CrossSection, CrossSection, CrossSection) {
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![6.0, 1.0, 1.0, 6.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let xs500 = CrossSection {
            station: 500.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.5, 0.5, 0.5, 5.5],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        (xs1000, xs500, xs0)
    }

    fn culvert_params_at_faces(
        q: f64,
        y_us: f64,
        y_ds: f64,
        xs_us: &CrossSection,
        xs_ds: &CrossSection,
    ) -> crate::solvers::culvert::CulvertSolveParams {
        let us_table = xs_us.generate_lookup_table(50);
        let ds_table = xs_ds.generate_lookup_table(50);
        let us_row = geometry_row_at_elevation(&us_table, Some(xs_us), y_us, None, None);
        let ds_row = geometry_row_at_elevation(&ds_table, Some(xs_ds), y_ds, None, None);
        crate::solvers::culvert::CulvertSolveParams {
            q,
            shape_type: 0,
            inlet_type: 1,
            span: 2.0,
            rise: 2.0,
            roughness_n: 0.013,
            length: 30.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 0.0,
            z_up: 0.5,
            tw_wsel: y_ds,
            units: UnitSystem::Metric,
            manning_n_bottom: 0.013,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: q / ds_row.channel_area.max(1e-9),
            us_velocity: q / us_row.channel_area.max(1e-9),
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 1,
            active_barrels: 1,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
        }
    }

    fn inline_culvert_reach_inputs() -> UnsteadyInputs {
        let (xs1000, xs500, xs0) = inline_culvert_metric_xs();
        UnsteadyInputs {
            cross_sections: vec![xs1000, xs500, xs0],
            initial_wsel: vec![1.8, 1.5, 0.5],
            initial_q: vec![10.0, 10.0, 10.0],
            dt: 60.0,
            num_steps: 80,
            upstream_q_hydrograph: vec![10.0; 80],
            downstream_wsel_hydrograph: vec![0.5; 80],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: Some(600.0),
            densify_reach_modifier_policy: None,
            coeff_contraction: None,
            coeff_expansion: None,
            culvert: UnsteadyCulvertInputs {
                culvert_stations: Some(vec![250.0]),
                culvert_shape_types: Some(vec![0]),
                culvert_spans: Some(vec![2.0]),
                culvert_rises: Some(vec![2.0]),
                culvert_roughness_ns: Some(vec![0.013]),
                culvert_lengths: Some(vec![30.0]),
                culvert_entrance_loss_coeffs: Some(vec![0.5]),
                culvert_exit_loss_coeffs: Some(vec![1.0]),
                culvert_barrels: Some(vec![1]),
                culvert_inlet_types: Some(vec![1]),
                ..Default::default()
            },
            bridge: UnsteadyBridgeInputs::default(),
            structure_coupling_order: None,
            unsteady_structure_coupling_mode: Some(2),
        }
    }

    #[test]
    fn test_unsteady_implicit_culvert_constant_q_matches_steady_hw() {
        let (_, xs500, xs0) = inline_culvert_metric_xs();
        let run = solve_unsteady(&inline_culvert_reach_inputs());
        let last = run.wsel.len() - 1;
        let q_face = run.q[last][1];
        let y_us = run.wsel[last][1];
        let y_ds = run.wsel[last][2];
        assert!(q_face.is_finite() && y_us.is_finite() && y_ds.is_finite());

        let params = culvert_params_at_faces(q_face, y_us, y_ds, &xs500, &xs0);
        let steady = crate::solvers::culvert::solve_culvert(&params);
        assert_eq!(steady.control_type, "inlet");

        let hw_diag = run
            .culvert_wsel_inlet
            .as_ref()
            .expect("culvert diagnostics")[last][0];
        assert!(
            (hw_diag - steady.wsel_inlet).abs() < 1e-4,
            "diagnostics {hw_diag} vs steady inlet {}",
            steady.wsel_inlet
        );

        let residual =
            crate::solvers::culvert::culvert_headwater_residual(y_us, y_ds, q_face, &params)
                .expect("inlet residual at final faces");
        assert!(
            residual.r.abs() < 0.05,
            "implicit face residual R={} (y_us={} HW_inlet={})",
            residual.r,
            y_us,
            steady.wsel_inlet
        );
        assert!(
            (y_us - steady.wsel).abs() < 0.05,
            "face WSEL {y_us} vs steady governing {}",
            steady.wsel
        );
    }

    #[test]
    fn test_unsteady_implicit_culvert_mild_pulse_stays_finite() {
        let mut inputs = inline_culvert_reach_inputs();
        inputs.num_steps = 8;
        inputs.upstream_q_hydrograph = vec![18.0, 19.0, 20.0, 21.0, 20.0, 19.0, 20.0, 20.0];
        let run = solve_unsteady(&inputs);
        assert!(run.wsel.iter().flatten().all(|w| w.is_finite()));
        assert!(run.q.iter().flatten().all(|q| q.is_finite()));
        assert!(run.wsel[7][1] > inputs.downstream_wsel_hydrograph[0]);
    }
}

