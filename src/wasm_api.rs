//! WASM API metadata and constants for host applications (browser / Web Worker).

use serde::Serialize;

/// API contract version — increment when SteadyInputs / SteadyResult fields change.
pub const API_VERSION: u32 = 4;

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

pub fn build_api_metadata() -> WasmApiMetadata {
    WasmApiMetadata {
        engine_version: ENGINE_VERSION.to_string(),
        api_version: API_VERSION,
        entry_points: vec![
            "init".to_string(),
            "solveSteady".to_string(),
            "solveUnsteady".to_string(),
            "getEngineVersion".to_string(),
            "getWasmApiMetadata".to_string(),
            "validateSteadyInputs".to_string(),
            "computeCulvertRatingCurve".to_string(),
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
        ],
        culvert_inlet_types: vec![
            EnumEntry {
                code: 0,
                name: "LegacyKeThreshold".to_string(),
                description: "Infer nomograph from entrance loss Ke (backward compatible)".to_string(),
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
            ],
        },
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
        assert!(json.contains("\"api_version\":4"));
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
        })
        .unwrap();
        assert!(result_json.contains("culvert_control_types"));
    }
}
