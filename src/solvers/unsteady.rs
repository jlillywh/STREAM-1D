use crate::utils::{G_METRIC, UnitSystem, FT_TO_M, Mat2, Vec2, solve_block_tridiagonal, structure_in_reach_interval};
use crate::geometry::{
    conveyance_derivative_at_elevation, flow_area_for_row, geometry_row_at_elevation,
    CrossSection, DensifyReachModifierPolicy, GeometryTable, IneffectiveFlowAreas,
};

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
}

fn bridge_face_blocks(
    face_stations: Option<&Vec<Vec<f64>>>,
    face_elevations: Option<&Vec<Vec<f64>>>,
    legacy_stations: Option<&Vec<Vec<f64>>>,
    legacy_elevations: Option<&Vec<Vec<f64>>>,
    b_idx: usize,
) -> (Vec<f64>, Vec<f64>) {
    let stations = face_stations
        .and_then(|v| v.get(b_idx))
        .filter(|blocks| !blocks.is_empty())
        .cloned()
        .or_else(|| legacy_stations.and_then(|v| v.get(b_idx)).cloned())
        .unwrap_or_default();
    let elevations = face_elevations
        .and_then(|v| v.get(b_idx))
        .filter(|blocks| !blocks.is_empty())
        .cloned()
        .or_else(|| legacy_elevations.and_then(|v| v.get(b_idx)).cloned())
        .unwrap_or_default();
    (stations, elevations)
}

pub(crate) fn bridge_ineffective_upstream_for(
    inputs: &UnsteadyInputs,
    b_idx: usize,
) -> Option<IneffectiveFlowAreas> {
    let b = &inputs.bridge;
    let (left_s, left_e) = bridge_face_blocks(
        b.bridge_ineffective_left_stations_upstream.as_ref(),
        b.bridge_ineffective_left_elevations_upstream.as_ref(),
        b.bridge_ineffective_left_stations.as_ref(),
        b.bridge_ineffective_left_elevations.as_ref(),
        b_idx,
    );
    let (right_s, right_e) = bridge_face_blocks(
        b.bridge_ineffective_right_stations_upstream.as_ref(),
        b.bridge_ineffective_right_elevations_upstream.as_ref(),
        b.bridge_ineffective_right_stations.as_ref(),
        b.bridge_ineffective_right_elevations.as_ref(),
        b_idx,
    );
    IneffectiveFlowAreas::from_block_pairs(&left_s, &left_e, &right_s, &right_e)
}

pub(crate) fn bridge_ineffective_downstream_for(
    inputs: &UnsteadyInputs,
    b_idx: usize,
) -> Option<IneffectiveFlowAreas> {
    let b = &inputs.bridge;
    let (left_s, left_e) = bridge_face_blocks(
        b.bridge_ineffective_left_stations_downstream.as_ref(),
        b.bridge_ineffective_left_elevations_downstream.as_ref(),
        b.bridge_ineffective_left_stations.as_ref(),
        b.bridge_ineffective_left_elevations.as_ref(),
        b_idx,
    );
    let (right_s, right_e) = bridge_face_blocks(
        b.bridge_ineffective_right_stations_downstream.as_ref(),
        b.bridge_ineffective_right_elevations_downstream.as_ref(),
        b.bridge_ineffective_right_stations.as_ref(),
        b.bridge_ineffective_right_elevations.as_ref(),
        b_idx,
    );
    IneffectiveFlowAreas::from_block_pairs(&left_s, &left_e, &right_s, &right_e)
}

fn bridge_face_geometry_for(
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
    let b = &inputs.bridge;
    let reach_z_up_user = if raw_units == UnitSystem::USCustomary {
        densified_z_mins[i] / FT_TO_M
    } else {
        densified_z_mins[i]
    };
    let reach_z_down_user = if raw_units == UnitSystem::USCustomary {
        densified_z_mins[i + 1] / FT_TO_M
    } else {
        densified_z_mins[i + 1]
    };
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
    let (approach_xs, departure_xs, guide_banks_approach, guide_banks_departure) =
        crate::solvers::bridge_interior::resolve_approach_departure_sections(
            &interior,
            i,
            densified_stations,
            &densified_xs_opt,
            raw_units,
        );
    crate::solvers::bridge_interior::resolve_bridge_face_solve_geometry(
        &interior,
        anchor_reach_xs.as_ref(),
        Some(&densified_xs[i]),
        Some(&densified_xs[i + 1]),
        &densified_tables[i],
        &densified_tables[i + 1],
        reach_z_up_user,
        reach_z_down_user,
        raw_units,
        num_slices,
        bridge_ineffective_upstream_for(inputs, b_idx),
        bridge_ineffective_downstream_for(inputs, b_idx),
        b.bridge_skew_angles
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0),
        b.bridge_pier_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .cloned(),
        interval_length_m,
        b.bridge_lengths
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0),
        approach_xs,
        departure_xs,
        guide_banks_approach,
        guide_banks_departure,
    )
}

fn bridge_deck_profile_for(
    inputs: &UnsteadyInputs,
    b_idx: usize,
    raw_units: UnitSystem,
) -> Option<crate::solvers::bridge::BridgeDeckProfile> {
    let b = &inputs.bridge;
    let low_chord = b
        .bridge_low_chords
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .copied()
        .unwrap_or(0.0);
    let high_chord = b
        .bridge_high_chords
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .copied()
        .unwrap_or(0.0);
    crate::solvers::bridge::build_bridge_deck_profile(
        low_chord,
        high_chord,
        b.bridge_deck_stations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .map(|s| s.as_slice()),
        b.bridge_deck_low_elevations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .map(|s| s.as_slice()),
        b.bridge_deck_high_elevations
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .map(|s| s.as_slice()),
        raw_units,
    )
}

fn bridge_coupling_for(inputs: &UnsteadyInputs, b_idx: usize) -> crate::solvers::bridge::BridgeCouplingParams {
    let b = &inputs.bridge;
    let abutment = crate::solvers::bridge_abutment::abutment_user_input_from_steady(
        b.bridge_abutment_block_widths
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied(),
        b.bridge_abutment_left_widths.as_ref(),
        b.bridge_abutment_right_widths.as_ref(),
        b.bridge_abutment_left_stations.as_ref(),
        b.bridge_abutment_right_stations.as_ref(),
        b.bridge_abutment_left_top_elevations.as_ref(),
        b.bridge_abutment_right_top_elevations.as_ref(),
        b.bridge_abutment_left_top_profile_stations.as_ref(),
        b.bridge_abutment_left_top_profile_elevations.as_ref(),
        b.bridge_abutment_right_top_profile_stations.as_ref(),
        b.bridge_abutment_right_top_profile_elevations.as_ref(),
        b_idx,
    );
    crate::solvers::bridge::BridgeCouplingParams {
        abutment,
        low_flow_method: inputs
            .bridge
            .bridge_low_flow_methods
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0),
        high_flow_method: inputs
            .bridge
            .bridge_high_flow_methods
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0),
        length: inputs
            .bridge
            .bridge_lengths
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0),
        wspro_coeff: inputs
            .bridge
            .bridge_wspro_coeffs
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.8),
        coeff_contraction: inputs.coeff_contraction.unwrap_or(0.1),
        coeff_expansion: inputs.coeff_expansion.unwrap_or(0.3),
        pressure_coeff_inlet: inputs
            .bridge
            .bridge_pressure_flow_coeffs_inlet
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.0),
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: inputs
            .bridge
            .bridge_max_weir_submergence
            .as_ref()
            .and_then(|v| v.get(b_idx))
            .copied()
            .unwrap_or(0.98),
    }
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

const CULVERT_HW_MAX_ITER: usize = 12;
const BRIDGE_HW_MAX_ITER: usize = 8;
const CULVERT_HW_TOL_FT: f64 = 0.001;
const CULVERT_HW_TOL_M: f64 = 0.0003;
const CULVERT_STEP_MAX_PASSES: usize = 5;
const CULVERT_STEP_TOL_FT: f64 = 0.01;
const CULVERT_STEP_TOL_M: f64 = 0.003;

/// Post-step coupling order when both culverts and bridges are present.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum StructureCouplingOrder {
    /// Merge culverts and bridges; process by reach interval downstream-first.
    CombinedDownstream = 0,
    /// Legacy: all culverts (downstream-first), then all bridges (downstream-first).
    CulvertsFirst = 1,
    /// All bridges (downstream-first), then all culverts (downstream-first).
    BridgesFirst = 2,
}

impl StructureCouplingOrder {
    fn from_i32(val: i32) -> Self {
        match val {
            1 => StructureCouplingOrder::CulvertsFirst,
            2 => StructureCouplingOrder::BridgesFirst,
            _ => StructureCouplingOrder::CombinedDownstream,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum StructureKind {
    Culvert,
    Bridge,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct CoupledStructure {
    interval_i: usize,
    kind: StructureKind,
    idx: usize,
}

struct StructureCouplingStepResults {
    culvert: Option<Vec<crate::solvers::culvert::CulvertSolveResult>>,
    bridge: Option<Vec<crate::solvers::bridge::BridgeSolveResult>>,
}

fn build_coupled_structure_order(
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

fn find_structure_intervals(
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
            if structure_in_reach_interval(s_st_metric, &densified_stations, i) {
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

fn build_unsteady_culvert_params(
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

fn apply_structure_internal_boundaries(
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
    let num_culverts = inputs.culvert.culvert_stations.as_ref().map(|s| s.len()).unwrap_or(0);
    let num_bridges = inputs.bridge.bridge_stations.as_ref().map(|s| s.len()).unwrap_or(0);
    let order = StructureCouplingOrder::from_i32(inputs.structure_coupling_order.unwrap_or(0));
    let coupled = build_coupled_structure_order(culvert_intervals, bridge_intervals, order);

    if coupled.is_empty() {
        return StructureCouplingStepResults {
            culvert: None,
            bridge: None,
        };
    }

    let step_tol = culvert_step_tolerance(raw_units);
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

    for _pass in 0..CULVERT_STEP_MAX_PASSES {
        let mut max_delta = 0.0_f64;

        for structure in &coupled {
            let i = structure.interval_i;
            let tw_wsel_user = if raw_units == UnitSystem::USCustomary {
                y_metric[i + 1] / FT_TO_M
            } else {
                y_metric[i + 1]
            };
            let prev_hw_user = if raw_units == UnitSystem::USCustomary {
                y_metric[i] / FT_TO_M
            } else {
                y_metric[i]
            };

            match structure.kind {
                StructureKind::Culvert => {
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
                    let interval_length_m = densified_stations[i] - densified_stations[i + 1];
                    let result = converge_bridge_headwater(
                        inputs,
                        structure.idx,
                        i,
                        raw_units,
                        densified_stations,
                        densified_tables,
                        densified_xs,
                        densified_z_mins,
                        q_metric,
                        tw_wsel_user,
                        prev_hw_user,
                        interval_length_m,
                    );
                    max_delta = max_delta.max((result.wsel_up - prev_hw_user).abs());
                    y_metric[i] = if raw_units == UnitSystem::USCustomary {
                        result.wsel_up * FT_TO_M
                    } else {
                        result.wsel_up
                    };
                    bridge_results[structure.idx] = result;
                }
            }
        }

        if max_delta <= step_tol {
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
    }
}

fn bridge_hw_tolerance(raw_units: UnitSystem) -> f64 {
    culvert_hw_tolerance(raw_units)
}

fn converge_bridge_headwater(
    inputs: &UnsteadyInputs,
    b_idx: usize,
    i: usize,
    raw_units: UnitSystem,
    densified_stations: &[f64],
    densified_tables: &[GeometryTable],
    densified_xs: &[CrossSection],
    densified_z_mins: &[f64],
    q_metric: &[f64],
    tw_wsel_user: f64,
    initial_hw: f64,
    interval_length_m: f64,
) -> crate::solvers::bridge::BridgeSolveResult {
    let b = &inputs.bridge;
    let low_chord = b.bridge_low_chords.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0);
    let high_chord = b.bridge_high_chords.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0);
    let pier_width = b.bridge_pier_widths.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0);
    let num_piers = b.bridge_num_piers.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0);
    let pier_shape = b.bridge_pier_shapes.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0);
    let weir_coeff = b
        .bridge_weir_coeffs
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .copied()
        .unwrap_or(if raw_units == UnitSystem::USCustomary {
            2.6
        } else {
            1.44
        });
    let orifice_coeff = b
        .bridge_orifice_coeffs
        .as_ref()
        .and_then(|v| v.get(b_idx))
        .copied()
        .unwrap_or(0.5);
    let coupling = bridge_coupling_for(inputs, b_idx);
    let deck = bridge_deck_profile_for(inputs, b_idx, raw_units);
    let deck_ref = deck.as_ref();
    let num_slices = inputs.num_slices.unwrap_or(100);
    let face_geo = bridge_face_geometry_for(
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

    let q_user = if raw_units == UnitSystem::USCustomary {
        q_metric[i] / crate::utils::CFS_TO_CMS
    } else {
        q_metric[i]
    };

    let tol = bridge_hw_tolerance(raw_units);
    let mut wsel_up_user = initial_hw;
    let mut result = crate::solvers::bridge::solve_bridge_coupled(
        q_user,
        low_chord,
        high_chord,
        pier_width,
        num_piers,
        pier_shape,
        weir_coeff,
        orifice_coeff,
        face_geo.z_down_user,
        face_geo.z_up_user,
        tw_wsel_user,
        raw_units,
        &face_geo.table_up,
        &face_geo.table_down,
        &coupling,
        interval_length_m,
        deck_ref,
        Some(&face_geo.sections),
    );

    for _ in 0..BRIDGE_HW_MAX_ITER {
        result = crate::solvers::bridge::solve_bridge_coupled(
            q_user,
            low_chord,
            high_chord,
            pier_width,
            num_piers,
            pier_shape,
            weir_coeff,
            orifice_coeff,
            face_geo.z_down_user,
            face_geo.z_up_user,
            tw_wsel_user,
            raw_units,
            &face_geo.table_up,
            &face_geo.table_down,
            &coupling,
            interval_length_m,
            deck_ref,
            Some(&face_geo.sections),
        );
        if (result.wsel_up - wsel_up_user).abs() <= tol {
            break;
        }
        wsel_up_user = result.wsel_up;
    }
    result
}

/// Helper to compute numerical derivative of conveyance K with respect to elevation y.
fn compute_dk_dy(table: &GeometryTable, xs: &CrossSection, elev: f64) -> f64 {
    conveyance_derivative_at_elevation(table, Some(xs), elev, None, None, 0.01)
}

/// Solves a single unsteady time step.
pub fn solve_unsteady_step(
    tables: &[GeometryTable],
    xs_list: &[CrossSection],
    z_mins: &[f64],
    y_current: &[f64], // current WSEL (metric)
    q_current: &[f64], // current Q (metric)
    dt: f64,
    q_up_next: f64,    // upstream flow BC at t+1 (metric)
    y_down_next: f64,  // downstream stage BC at t+1 (metric)
    theta: f64,
    c_contraction: f64,
    c_expansion: f64,
) -> Option<(Vec<f64>, Vec<f64>)> {
    let n = y_current.len();
    if n < 2 {
        return None;
    }

    // Allocate block tridiagonal matrices
    let mut a = vec![Mat2::zero(); n];
    let mut b = vec![Mat2::zero(); n];
    let mut c = vec![Mat2::zero(); n];
    let mut d = vec![Vec2::zero(); n];

    // Node 0: Upstream Boundary Condition
    // BC: \Delta Q_0 = q_up_next - q_current[0]
    // Equation 1 of node 0: 0 * \Delta y_0 + 1 * \Delta Q_0 = q_up_next - q_current[0]
    let b0_11 = 0.0;
    let b0_12 = 1.0;
    let d0_1 = q_up_next - q_current[0];

    // Node N-1: Downstream Boundary Condition
    // BC: \Delta y_{N-1} = y_down_next - y_current[N-1]
    // Equation 2 of node N-1: 1 * \Delta y_{N-1} + 0 * \Delta Q_{N-1} = y_down_next - y_current[N-1]
    let bn_21 = 1.0;
    let bn_22 = 0.0;
    let dn_2 = y_down_next - y_current[n - 1];

    // Populate intervals (0 to N-2)
    for i in 0..n - 1 {
        let dx = xs_list[i].station - xs_list[i + 1].station; // Reach length
        if dx <= 0.0 {
            return None; // Invalid station spacing
        }

        // Section properties at current time step (ineffective/blocked/guide-bank aware)
        let row_i = geometry_row_at_elevation(&tables[i], Some(&xs_list[i]), y_current[i], None, None);
        let row_ip =
            geometry_row_at_elevation(&tables[i + 1], Some(&xs_list[i + 1]), y_current[i + 1], None, None);

        let a_i = row_i.area.max(1e-6);
        let a_ip = row_ip.area.max(1e-6);
        let flow_a_i = flow_area_for_row(&row_i).max(1e-6);
        let flow_a_ip = flow_area_for_row(&row_ip).max(1e-6);
        let t_i = row_i.top_width.max(1e-6);
        let t_ip = row_ip.top_width.max(1e-6);

        let v_i = q_current[i] / flow_a_i;
        let v_ip = q_current[i + 1] / flow_a_ip;

        // Conveyance and its derivatives
        let k_i = row_i.conveyance.max(1e-6);
        let k_ip = row_ip.conveyance.max(1e-6);

        let dk_dy_i = compute_dk_dy(&tables[i], &xs_list[i], y_current[i]);
        let dk_dy_ip = compute_dk_dy(&tables[i + 1], &xs_list[i + 1], y_current[i + 1]);

        // Friction slope and derivatives
        let q_avg = 0.5 * (q_current[i] + q_current[i + 1]);
        let k_avg = 0.5 * (k_i + k_ip);
        let k_avg_clamp = k_avg.max(0.01);
        let sf = (q_avg * q_avg.abs()) / (k_avg_clamp * k_avg_clamp);

        // dSf/dQ
        let d_sf_d_q = 2.0 * q_avg.abs() / (k_avg_clamp * k_avg_clamp);
        
        // Compute local depths to suppress derivatives at dry/shallow nodes
        let z_min_i = z_mins[i];
        let z_min_ip = z_mins[i + 1];
        let depth_i = (y_current[i] - z_min_i).max(0.0);
        let depth_ip = (y_current[i + 1] - z_min_ip).max(0.0);

        // dSf/dy (evaluated for node i and i+1)
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

        // Averaged variables
        let a_avg = (a_i * a_ip).sqrt();

        // 1. CONTINUTIY EQUATION COEFFICIENTS
        // C1 * \Delta y_i + C2 * \Delta Q_i + C3 * \Delta y_{i+1} + C4 * \Delta Q_{i+1} = CE
        let c1 = t_i / (2.0 * dt);
        let c2 = -theta / dx;
        let c3 = t_ip / (2.0 * dt);
        let c4 = theta / dx;
        let ce = (q_current[i] - q_current[i + 1]) / dx;

        // Froude number convective term suppression for mixed flow stability
        let d_hyd_i = a_i / t_i;
        let celerity_i = (G_METRIC * d_hyd_i).sqrt();
        let fr_i = if celerity_i > 1e-6 { v_i.abs() / celerity_i } else { 0.0 };
        let factor_i = if fr_i < 1.0 { (1.0 - fr_i * fr_i).max(0.0) } else { 0.0 };

        let d_hyd_ip = a_ip / t_ip;
        let celerity_ip = (G_METRIC * d_hyd_ip).sqrt();
        let fr_ip = if celerity_ip > 1e-6 { v_ip.abs() / celerity_ip } else { 0.0 };
        let factor_ip = if fr_ip < 1.0 { (1.0 - fr_ip * fr_ip).max(0.0) } else { 0.0 };

        // 2. MOMENTUM EQUATION COEFFICIENTS
        // M1 * \Delta y_i + M2 * \Delta Q_i + M3 * \Delta y_{i+1} + M4 * \Delta Q_{i+1} = ME
        
        // Contraction/Expansion losses
        let c_ec = if v_ip.abs() > v_i.abs() { c_contraction } else { c_expansion };
        let sign_v = (v_ip * v_ip - v_i * v_i).signum();
        let s_ce_force = a_avg * (c_ec / (2.0 * dx)) * (v_ip * v_ip - v_i * v_i).abs();

        let dfce_dyi = a_avg * (c_ec / dx) * sign_v * (v_i * v_i * t_i / flow_a_i);
        let dfce_dqi = -a_avg * (c_ec / dx) * sign_v * (v_i / flow_a_i);
        let dfce_dyip = -a_avg * (c_ec / dx) * sign_v * (v_ip * v_ip * t_ip / flow_a_ip);
        let dfce_dqip = a_avg * (c_ec / dx) * sign_v * (v_ip / flow_a_ip);

        let m1 = theta / dx * (v_i * v_i * t_i) * factor_i - G_METRIC * a_avg * theta / dx + G_METRIC * a_avg * theta * d_sf_dy_i + theta * dfce_dyi;
        let m2 = (1.0 / (2.0 * dt)) - theta / dx * (2.0 * v_i) * factor_i + 0.5 * G_METRIC * a_avg * theta * d_sf_d_q + theta * dfce_dqi;
        let m3 = -theta / dx * (v_ip * v_ip * t_ip) * factor_ip + G_METRIC * a_avg * theta / dx + G_METRIC * a_avg * theta * d_sf_dy_ip + theta * dfce_dyip;
        let m4 = (1.0 / (2.0 * dt)) + theta / dx * (2.0 * v_ip) * factor_ip + 0.5 * G_METRIC * a_avg * theta * d_sf_d_q + theta * dfce_dqip;

        let flux_i = (q_current[i] * q_current[i] / a_i) * factor_i;
        let flux_ip = (q_current[i + 1] * q_current[i + 1] / a_ip) * factor_ip;
        let me = (flux_i - flux_ip) / dx + G_METRIC * a_avg * (y_current[i] - y_current[i + 1]) / dx - G_METRIC * a_avg * sf - s_ce_force;

        // Pack into block tridiagonal matrices
        if i == 0 {
            b[0] = Mat2 {
                m11: b0_11, m12: b0_12,
                m21: c1,    m22: c2,
            };
            c[0] = Mat2 {
                m11: 0.0, m12: 0.0,
                m21: c3,  m22: c4,
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

        // Pack momentum equation of interval i into block i + 1
        a[i + 1] = Mat2 {
            m11: m1, m12: m2,
            m21: 0.0, m22: 0.0,
        };
        b[i + 1].m11 = m3;
        b[i + 1].m12 = m4;
        d[i + 1].v1 = me;

        // Pack downstream boundary condition into block n - 1
        if i == n - 2 {
            b[n - 1].m21 = bn_21;
            b[n - 1].m22 = bn_22;
            d[n - 1].v2 = dn_2;
        }
    }

    // Solve system
    let delta = solve_block_tridiagonal(&a, &b, &c, &d)?;

    // Apply updates
    let mut y_next = vec![0.0; n];
    let mut q_next = vec![0.0; n];
    for i in 0..n {
        let dy = delta[i].v1.clamp(-1.0, 1.0);
        let dq = delta[i].v2.clamp(-25.0, 25.0);

        y_next[i] = y_current[i] + dy;
        q_next[i] = q_current[i] + dq;
    }

    // Explicitly enforce boundary conditions exactly
    q_next[0] = q_up_next;
    y_next[n - 1] = y_down_next;

    Some((y_next, q_next))
}

/// Solves unsteady-state Saint-Venant flow routing.
pub fn solve_unsteady(inputs: &UnsteadyInputs) -> UnsteadyResult {
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
        inputs,
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
        .map(|stations| find_structure_intervals(stations, &densified_stations, raw_units))
        .unwrap_or_default();

    let bridge_intervals =
        crate::solvers::bridge_interior::bridge_intervals_from_faces(&bridge_face_intervals);

    let track_culvert_diagnostics = !culvert_intervals.is_empty();
    let track_bridge_diagnostics = !bridge_intervals.is_empty();
    let has_structures = track_culvert_diagnostics || track_bridge_diagnostics;

    if has_structures {
        apply_structure_internal_boundaries(
            inputs,
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

        // Solve next time step
        if let Some((y_next, q_next)) = solve_unsteady_step(
            &densified_tables,
            &densified_xs,
            &densified_z_mins,
            &densified_y_current,
            &densified_q_current,
            dt,
            q_up_next,
            y_down_next,
            theta,
            c_contraction,
            c_expansion,
        ) {
            densified_y_current = y_next;
            densified_q_current = q_next;

             let structure_step_results = apply_structure_internal_boundaries(
                inputs,
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

        if step < 5 {
            println!("Step {}: densified WSEL (ft) = {:?}", step, densified_y_current.iter().map(|&w| w / FT_TO_M).collect::<Vec<f64>>());
            println!("Step {}: densified Q (cfs)    = {:?}", step, densified_q_current.iter().map(|&q| q / crate::utils::CFS_TO_CMS).collect::<Vec<f64>>());
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
}

