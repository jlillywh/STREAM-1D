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
    assert!(out_json.contains("culvert_q_barrels"));
    assert!(out_json.contains("culvert_wsel_inlet"));
}

#[test]
fn wasm_culvert_rating_curve_contract() {
    use streams1d::solvers::{compute_culvert_rating_curve, CulvertRatingCurveInputs, CulvertSolveParams};
    use streams1d::utils::UnitSystem;

    let inputs = CulvertRatingCurveInputs {
        q_values: vec![50.0, 100.0],
        culvert: CulvertSolveParams {
            q: 0.0,
            shape_type: 0,
            inlet_type: 1,
            span: 5.0,
            rise: 5.0,
            roughness_n: 0.012,
            length: 100.0,
            entrance_loss_coeff: 0.5,
            exit_loss_coeff: 1.0,
            z_down: 9.0,
            z_up: 10.0,
            tw_wsel: 12.0,
            units: UnitSystem::USCustomary,
            manning_n_bottom: 0.012,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: 1,
        },
    };
    let curve = compute_culvert_rating_curve(&inputs);
    assert_eq!(curve.q.len(), 2);
    let json = serde_json::to_string(&curve).unwrap();
    assert!(json.contains("barrel_froude"));
}

#[test]
fn wasm_api_metadata_version() {
    let meta = build_api_metadata();
    assert_eq!(meta.api_version, API_VERSION);
    assert!(meta.culvert_tier1_fields.inputs.contains(&"culvert_inlet_types".to_string()));
}
