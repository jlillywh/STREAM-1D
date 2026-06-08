//! JSON contract tests for the WASM API (same schema as `serde_wasm_bindgen`).

use streams1d::solvers::{solve_steady, SteadyInputs};
use streams1d::wasm_api::{build_api_metadata, API_VERSION};

#[test]
fn wasm_fixture_deserializes_and_solves() {
    let json = include_str!("fixtures/wasm_steady_culvert_tier1.json");
    let inputs: SteadyInputs = serde_json::from_str(json).expect("fixture must deserialize");
    let result = solve_steady(&inputs);

    assert_eq!(result.wsel.len(), inputs.cross_sections.len());
    let controls = result
        .culvert_control_types
        .as_ref()
        .expect("culvert_control_types in WASM result");
    assert_eq!(controls.len(), 1);
    assert!(
        controls[0] == "inlet" || controls[0] == "outlet" || controls[0] == "overtopping",
        "unexpected control type: {}",
        controls[0]
    );

    let out_json = serde_json::to_string(&result).unwrap();
    assert!(out_json.contains("culvert_control_types"));
}

#[test]
fn wasm_api_metadata_version() {
    let meta = build_api_metadata();
    assert_eq!(meta.api_version, API_VERSION);
    assert!(meta.culvert_tier1_fields.inputs.contains(&"culvert_inlet_types".to_string()));
}
