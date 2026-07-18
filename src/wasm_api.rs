//! WASM API metadata and constants for host applications (browser / Web Worker).

use serde::Serialize;

/// API contract version — increment when SteadyInputs / SteadyResult fields change.
pub const API_VERSION: u32 = 38;

/// Engine package version (keep in sync with `Cargo.toml`).
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize)]
pub struct EnumEntry {
    pub code: i32,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WasmApiMetadata {
    pub engine_version: String,
    pub api_version: u32,
    pub entry_points: Vec<String>,
    pub field_naming: String,
    pub culvert_shape_types: Vec<EnumEntry>,
    pub culvert_inlet_types: Vec<EnumEntry>,
    pub culvert_control_types: Vec<String>,
    pub culvert_tier1_fields: CulvertTier1Fields,
    pub culvert_tier2a_fields: CulvertTier2aFields,
    pub culvert_geometry_fields: CulvertGeometryFields,
    pub bridge_fields: BridgeFields,
    pub structure_coupling_orders: Vec<EnumEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CulvertTier1Fields {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CulvertTier2aFields {
    pub steady_outputs: Vec<String>,
    pub rating_curve_entry_point: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CulvertGeometryFields {
    pub inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeFields {
    pub inputs: Vec<String>,
    pub flow_regimes: Vec<String>,
    pub rating_curve_entry_point: String,
    /// Flattened `BridgeSolveParams` keys accepted by `computeBridgeRatingCurve` (not `bridge_*` prefixed).
    pub rating_curve_inputs: Vec<String>,
    pub rating_curve_outputs: Vec<String>,
}

pub fn build_api_metadata() -> WasmApiMetadata {
    WasmApiMetadata {
        engine_version: ENGINE_VERSION.to_string(),
        api_version: API_VERSION,
        entry_points: vec![
            "init".to_string(),
            "solveSteady".to_string(),
            "getEngineVersion".to_string(),
            "getWasmApiMetadata".to_string(),
            "validateSteadyInputs".to_string(),
            "computeCulvertRatingCurve".to_string(),
            "computeBridgeRatingCurve".to_string(),
        ],
        field_naming: "snake_case (match Rust/Python JSON)".to_string(),
        culvert_shape_types: vec![
            EnumEntry {
                code: 0,
                name: "Circular".to_string(),
                description: "Circular pipe; span = diameter".to_string(),
            },
            EnumEntry {
                code: 1,
                name: "Box".to_string(),
                description: "Rectangular box; span = width, rise = height".to_string(),
            },
            EnumEntry {
                code: 2,
                name: "Arch".to_string(),
                description: "Parabolic arch".to_string(),
            },
            EnumEntry {
                code: 3,
                name: "ConspanArch".to_string(),
                description: "ConSpan manufactured arch".to_string(),
            },
            EnumEntry {
                code: 4,
                name: "PipeArch".to_string(),
                description: "Corrugated pipe-arch; span = spring-line width, rise = total height"
                    .to_string(),
            },
            EnumEntry {
                code: 5,
                name: "Elliptical".to_string(),
                description: "Elliptical pipe; span = major axis, rise = minor axis".to_string(),
            },
            EnumEntry {
                code: 6,
                name: "Horseshoe".to_string(),
                description: "Horseshoe; span = spring-line width, rise = total height".to_string(),
            },
            EnumEntry {
                code: 7,
                name: "Custom".to_string(),
                description: "User-defined shape using area, perimeter, and top width tables"
                    .to_string(),
            },
        ],
        culvert_inlet_types: vec![
            EnumEntry {
                code: 0,
                name: "LegacyKeThreshold".to_string(),
                description: "Infer nomograph from entrance loss Ke (backward compatible)"
                    .to_string(),
            },
            EnumEntry {
                code: 1,
                name: "CircularSquareHeadwall".to_string(),
                description: "Circular — square edge with headwall".to_string(),
            },
            EnumEntry {
                code: 2,
                name: "CircularGrooveEnd".to_string(),
                description: "Circular — groove end with headwall".to_string(),
            },
            EnumEntry {
                code: 3,
                name: "CircularBeveled45".to_string(),
                description: "Circular — beveled ring 45°".to_string(),
            },
            EnumEntry {
                code: 4,
                name: "CircularProjecting".to_string(),
                description: "Circular — projecting".to_string(),
            },
            EnumEntry {
                code: 10,
                name: "BoxSquareEdge".to_string(),
                description: "Box — square edge 90°".to_string(),
            },
            EnumEntry {
                code: 11,
                name: "BoxFlaredWingwalls".to_string(),
                description: "Box — flared wingwalls".to_string(),
            },
            EnumEntry {
                code: 12,
                name: "BoxBeveledTop".to_string(),
                description: "Box — beveled top edge".to_string(),
            },
            EnumEntry {
                code: 20,
                name: "ArchProjecting".to_string(),
                description: "Arch / ConSpan — projecting".to_string(),
            },
            EnumEntry {
                code: 21,
                name: "ArchSmoothEntry".to_string(),
                description: "Arch / ConSpan — smooth entry headwall".to_string(),
            },
        ],
        culvert_control_types: vec![
            "inlet".to_string(),
            "outlet".to_string(),
            "overtopping".to_string(),
        ],
        culvert_tier1_fields: CulvertTier1Fields {
            inputs: vec![
                "culvert_inlet_types".to_string(),
                "culvert_z_ups".to_string(),
                "culvert_z_downs".to_string(),
                "culvert_crest_elevs".to_string(),
                "culvert_weir_coeffs".to_string(),
                "culvert_weir_lengths".to_string(),
                "culvert_chart_numbers".to_string(),
                "culvert_scale_numbers".to_string(),
            ],
            outputs: vec!["culvert_control_types".to_string()],
        },
        culvert_tier2a_fields: CulvertTier2aFields {
            steady_outputs: vec![
                "culvert_wsel_inlet".to_string(),
                "culvert_wsel_outlet".to_string(),
                "culvert_q_barrels".to_string(),
                "culvert_q_weirs".to_string(),
                "culvert_barrel_depths".to_string(),
                "culvert_barrel_velocities".to_string(),
                "culvert_barrel_froude".to_string(),
            ],
            rating_curve_entry_point: "computeCulvertRatingCurve".to_string(),
        },
        culvert_geometry_fields: CulvertGeometryFields {
            inputs: vec![
                "culvert_skew_angles".to_string(),
                "culvert_active_barrels".to_string(),
                "culvert_barrel_spans".to_string(),
                "culvert_barrel_rises".to_string(),
                "culvert_approach_reach_stations".to_string(),
                "culvert_departure_reach_stations".to_string(),
            ],
        },
        bridge_fields: BridgeFields {
            inputs: vec![
                "bridge_stations".to_string(),
                "bridge_low_chords".to_string(),
                "bridge_high_chords".to_string(),
                "bridge_pier_widths".to_string(),
                "bridge_num_piers".to_string(),
                "bridge_pier_shapes".to_string(),
                "bridge_weir_coeffs".to_string(),
                "bridge_orifice_coeffs".to_string(),
                "bridge_abutment_block_widths".to_string(),
                "bridge_abutment_left_widths".to_string(),
                "bridge_abutment_right_widths".to_string(),
                "bridge_abutment_left_stations".to_string(),
                "bridge_abutment_right_stations".to_string(),
                "bridge_abutment_left_top_elevations".to_string(),
                "bridge_abutment_right_top_elevations".to_string(),
                "bridge_abutment_left_top_profile_stations".to_string(),
                "bridge_abutment_left_top_profile_elevations".to_string(),
                "bridge_abutment_right_top_profile_stations".to_string(),
                "bridge_abutment_right_top_profile_elevations".to_string(),
                "bridge_low_flow_methods".to_string(),
                "bridge_high_flow_methods".to_string(),
                "bridge_lengths".to_string(),
                "bridge_friction_weighting".to_string(),
                "bridge_approach_friction_lengths".to_string(),
                "bridge_departure_friction_lengths".to_string(),
                "bridge_opening_blockage_factors".to_string(),
                "bridge_pier_debris_widths".to_string(),
                "bridge_pier_debris_heights".to_string(),
                "bridge_ice_thicknesses".to_string(),
                "bridge_ice_modes".to_string(),
                "bridge_deck_ice_thicknesses".to_string(),
                "bridge_wspro_coeffs".to_string(),
                "bridge_pressure_flow_coeffs_inlet".to_string(),
                "bridge_max_weir_submergence".to_string(),
                "bridge_deck_stations".to_string(),
                "bridge_deck_low_elevations".to_string(),
                "bridge_deck_high_elevations".to_string(),
                "bridge_ineffective_left_stations".to_string(),
                "bridge_ineffective_left_elevations".to_string(),
                "bridge_ineffective_right_stations".to_string(),
                "bridge_ineffective_right_elevations".to_string(),
                "bridge_ineffective_left_stations_upstream".to_string(),
                "bridge_ineffective_left_elevations_upstream".to_string(),
                "bridge_ineffective_right_stations_upstream".to_string(),
                "bridge_ineffective_right_elevations_upstream".to_string(),
                "bridge_ineffective_left_stations_downstream".to_string(),
                "bridge_ineffective_left_elevations_downstream".to_string(),
                "bridge_ineffective_right_stations_downstream".to_string(),
                "bridge_ineffective_right_elevations_downstream".to_string(),
                "bridge_skew_angles".to_string(),
                "bridge_pier_stations".to_string(),
                "bridge_pier_top_widths".to_string(),
                "bridge_pier_bottom_widths".to_string(),
                "bridge_pier_width_elevations".to_string(),
                "bridge_pier_width_values".to_string(),
                "bridge_pier_top_elevations".to_string(),
                "bridge_pier_base_elevations".to_string(),
                "bridge_pier_footing_top_elevations".to_string(),
                "bridge_pier_footing_widths".to_string(),
                "bridge_pier_footing_bottom_elevations".to_string(),
                "bridge_pier_nosing_lengths".to_string(),
                "bridge_pier_nosing_widths".to_string(),
                "bridge_upstream_cross_sections".to_string(),
                "bridge_downstream_cross_sections".to_string(),
                "bridge_internal_cross_sections".to_string(),
                "bridge_opening_reach_station_origins".to_string(),
                "bridge_opening_anchor_modes".to_string(),
                "bridge_opening_anchor_reach_stations".to_string(),
                "bridge_approach_cross_sections".to_string(),
                "bridge_departure_cross_sections".to_string(),
                "bridge_approach_reach_stations".to_string(),
                "bridge_departure_reach_stations".to_string(),
                "bridge_approach_guide_banks".to_string(),
                "bridge_departure_guide_banks".to_string(),
                "bridge_roadway_embankments".to_string(),
            ],

            flow_regimes: vec![
                "low_a".to_string(),
                "low_b".to_string(),
                "low_c".to_string(),
                "pressure".to_string(),
                "weir".to_string(),
                "energy".to_string(),
            ],
            rating_curve_entry_point: "computeBridgeRatingCurve".to_string(),
            rating_curve_inputs: vec![
                "q_values".to_string(),
                "low_chord".to_string(),
                "high_chord".to_string(),
                "z_up".to_string(),
                "z_down".to_string(),
                "tw_wsel".to_string(),
                "tw_wsel_reverse".to_string(),
                "units".to_string(),
                "pier_width".to_string(),
                "num_piers".to_string(),
                "pier_shape_type".to_string(),
                "weir_coeff".to_string(),
                "orifice_coeff".to_string(),
                "abutment_block_width".to_string(),
                "abutment_left_width".to_string(),
                "abutment_right_width".to_string(),
                "abutment_left_station".to_string(),
                "abutment_right_station".to_string(),
                "abutment_left_top_elevation".to_string(),
                "abutment_right_top_elevation".to_string(),
                "abutment_left_top_profile_stations".to_string(),
                "abutment_left_top_profile_elevations".to_string(),
                "abutment_right_top_profile_stations".to_string(),
                "abutment_right_top_profile_elevations".to_string(),
                "low_flow_method".to_string(),
                "high_flow_method".to_string(),
                "length".to_string(),
                "friction_weighting".to_string(),
                "approach_friction_length".to_string(),
                "departure_friction_length".to_string(),
                "opening_blockage_factor".to_string(),
                "pier_debris_widths".to_string(),
                "pier_debris_heights".to_string(),
                "ice_thickness".to_string(),
                "ice_mode".to_string(),
                "deck_ice_thickness".to_string(),
                "wspro_coeff".to_string(),
                "coeff_contraction".to_string(),
                "coeff_expansion".to_string(),
                "pressure_coeff_inlet".to_string(),
                "max_weir_submergence".to_string(),
                "skew_deg".to_string(),
                "pier_stations".to_string(),
                "pier_top_widths".to_string(),
                "pier_bottom_widths".to_string(),
                "pier_width_elevations".to_string(),
                "pier_width_values".to_string(),
                "pier_top_elevations".to_string(),
                "pier_base_elevations".to_string(),
                "pier_footing_top_elevations".to_string(),
                "pier_footing_widths".to_string(),
                "pier_footing_bottom_elevations".to_string(),
                "pier_nosing_lengths".to_string(),
                "pier_nosing_widths".to_string(),
                "deck_stations".to_string(),
                "deck_low_elevations".to_string(),
                "deck_high_elevations".to_string(),
                "ineffective_left_stations".to_string(),
                "ineffective_left_elevations".to_string(),
                "ineffective_right_stations".to_string(),
                "ineffective_right_elevations".to_string(),
                "channel_width".to_string(),
                "manning_n".to_string(),
                "num_slices".to_string(),
                "xs_up".to_string(),
                "xs_down".to_string(),
                "opening_reach_station_origin".to_string(),
                "xs_internal".to_string(),
                "roadway_embankment".to_string(),
            ],
            rating_curve_outputs: vec![
                "q".to_string(),
                "wsel".to_string(),
                "wsel_down".to_string(),
                "flow_regimes".to_string(),
                "head_losses".to_string(),
            ],
        },
        structure_coupling_orders: vec![
            EnumEntry {
                code: 0,
                name: "CombinedDownstream".to_string(),
                description: "Merge culverts and bridges; couple downstream structures first"
                    .to_string(),
            },
            EnumEntry {
                code: 1,
                name: "CulvertsFirst".to_string(),
                description: "All culverts (downstream-first), then all bridges (downstream-first)"
                    .to_string(),
            },
            EnumEntry {
                code: 2,
                name: "BridgesFirst".to_string(),
                description: "All bridges (downstream-first), then all culverts (downstream-first)"
                    .to_string(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::{SteadyInputs, SteadyResult};

    #[test]
    fn test_api_metadata_serializes() {
        let json = serde_json::to_string(&build_api_metadata()).unwrap();
        assert!(json.contains("culvert_inlet_types"));
        assert!(json.contains("\"api_version\":38"));
        assert!(json.contains("structure_coupling_orders"));
    }

    #[test]
    fn test_steady_inputs_tier1_json_roundtrip() {
        let sample = include_str!("../tests/fixtures/wasm_steady_culvert_tier1.json");
        let inputs: SteadyInputs = serde_json::from_str(sample).unwrap();
        assert!(inputs.culvert_inlet_types.is_some());
        assert!(inputs.culvert_crest_elevs.is_some());

        let result_json = serde_json::to_string(&SteadyResult {
            wsel: vec![5.25, 3.0],
            critical_wsel: vec![2.0, 1.0],
            velocity: vec![1.0, 2.0],
            area: vec![10.0, 20.0],
            froude: vec![0.2, 0.3],
            top_width: vec![10.0, 10.0],
            eg_slope: vec![0.001, 0.002],
            tributary_wsel: None,
            tributary_velocity: None,
            tributary_froude: None,
            culvert_control_types: Some(vec!["inlet".to_string()]),
            culvert_wsel_inlet: Some(vec![14.5]),
            culvert_wsel_outlet: Some(vec![13.0]),
            culvert_q_barrels: Some(vec![100.0]),
            culvert_q_weirs: Some(vec![0.0]),
            culvert_barrel_depths: Some(vec![3.0]),
            culvert_barrel_velocities: Some(vec![5.0]),
            culvert_barrel_froude: Some(vec![0.8]),
            inline_structure_wsel_inlet: None,
            inline_structure_wsel_outlet: None,
            inline_structure_q_weirs: None,
        })
        .unwrap();
        assert!(result_json.contains("culvert_control_types"));
    }
}
