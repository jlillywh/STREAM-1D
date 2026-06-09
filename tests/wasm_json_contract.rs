//! JSON contract tests for the WASM API (same schema as `serde_wasm_bindgen`).

use streams1d::geometry::CrossSection;
use streams1d::solvers::{solve_steady, SteadyInputs};
use streams1d::utils::UnitSystem;
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
            active_barrels: 0,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
        },
    };
    let curve = compute_culvert_rating_curve(&inputs);
    assert_eq!(curve.q.len(), 2);
    let json = serde_json::to_string(&curve).unwrap();
    assert!(json.contains("barrel_froude"));
}

#[test]
fn wasm_bridge_rating_curve_contract() {
    use streams1d::solvers::{
        compute_bridge_rating_curve, BridgeRatingCurveInputs, BridgeSolveParams,
    };
    use streams1d::utils::UnitSystem;

    let inputs = BridgeRatingCurveInputs {
        q_values: vec![10.0, 20.0],
        bridge: BridgeSolveParams {
            low_chord: 5.0,
            high_chord: 7.0,
            z_down: 0.0,
            z_up: 0.0,
            tw_wsel: 2.5,
            units: UnitSystem::Metric,
            low_flow_method: 3,
            channel_width: 10.0,
            manning_n: 0.03,
            ..Default::default()
        },
    };
    let curve = compute_bridge_rating_curve(&inputs);
    assert_eq!(curve.q.len(), 2);
    let json = serde_json::to_string(&curve).unwrap();
    assert!(json.contains("flow_regimes"));
    assert!(json.contains("head_losses"));
}

#[test]
fn wasm_api_metadata_version() {
    let meta = build_api_metadata();
    assert_eq!(meta.api_version, API_VERSION);
    assert_eq!(API_VERSION, 21);
    assert!(meta.culvert_tier1_fields.inputs.contains(&"culvert_inlet_types".to_string()));
    assert_eq!(
        meta.bridge_fields.rating_curve_entry_point,
        "computeBridgeRatingCurve"
    );
    for key in [
        "bridge_abutment_left_widths",
        "bridge_abutment_right_widths",
        "bridge_abutment_left_stations",
        "bridge_abutment_right_stations",
        "bridge_abutment_left_top_elevations",
        "bridge_abutment_right_top_elevations",
        "bridge_abutment_left_top_profile_stations",
        "bridge_abutment_left_top_profile_elevations",
        "bridge_abutment_right_top_profile_stations",
        "bridge_abutment_right_top_profile_elevations",
    ] {
        assert!(
            meta.bridge_fields.inputs.contains(&key.to_string()),
            "missing bridge metadata field {key}"
        );
    }
    for key in [
        "abutment_left_width",
        "abutment_right_width",
        "abutment_left_top_elevation",
        "abutment_right_top_elevation",
        "abutment_left_top_profile_stations",
        "abutment_right_top_profile_elevations",
    ] {
        assert!(
            meta.bridge_fields
                .rating_curve_inputs
                .contains(&key.to_string()),
            "missing rating curve field {key}"
        );
    }
}

#[test]
fn wasm_bridge_rating_curve_per_side_abutments() {
    use streams1d::solvers::{
        compute_bridge_rating_curve, BridgeRatingCurveInputs, BridgeSolveParams,
    };
    use streams1d::utils::UnitSystem;

    let json = r#"{
        "q_values": [15.0, 20.0],
        "low_chord": 5.0,
        "high_chord": 7.0,
        "z_down": 0.0,
        "z_up": 0.0,
        "tw_wsel": 2.5,
        "units": "Metric",
        "low_flow_method": 4,
        "channel_width": 10.0,
        "manning_n": 0.03,
        "abutment_left_width": 1.0,
        "abutment_right_width": 4.0,
        "abutment_right_top_elevation": 2.5
    }"#;
    let inputs: BridgeRatingCurveInputs =
        serde_json::from_str(json).expect("rating curve per-side abutment JSON");
    assert_eq!(inputs.bridge.abutment_left_width, Some(1.0));
    assert_eq!(inputs.bridge.abutment_right_width, Some(4.0));
    assert_eq!(inputs.bridge.abutment_right_top_elevation, Some(2.5));

    let asymmetric = compute_bridge_rating_curve(&inputs);
    let symmetric = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![15.0, 20.0],
        bridge: BridgeSolveParams {
            low_chord: 5.0,
            high_chord: 7.0,
            z_down: 0.0,
            z_up: 0.0,
            tw_wsel: 2.5,
            units: UnitSystem::Metric,
            low_flow_method: 4,
            abutment_block_width: 5.0,
            channel_width: 10.0,
            manning_n: 0.03,
            ..Default::default()
        },
    });
    assert_eq!(asymmetric.wsel.len(), 2);
    assert!(
        (asymmetric.wsel[0] - symmetric.wsel[0]).abs() > 0.01,
        "per-side abutment tops should change rating-curve headwater"
    );
}

#[test]
fn bridge_ineffective_flat_arrays_roundtrip() {
    let json = r#"{
        "cross_sections": [{
            "station": 0.0,
            "x": [0.0, 0.0, 10.0, 10.0],
            "y": [5.0, 0.0, 0.0, 5.0],
            "n_stations": [0.0],
            "n_values": [0.03],
            "unit_system": "Metric"
        }],
        "flow_rate": 10.0,
        "regime": 0,
        "downstream_wsel": 1.0,
        "bridge_stations": [5.0],
        "bridge_low_chords": [3.0],
        "bridge_high_chords": [5.0],
        "bridge_ineffective_left_stations": [30.0],
        "bridge_ineffective_left_elevations": [2.5]
    }"#;
    let inputs: SteadyInputs = serde_json::from_str(json).expect("flat ineffective arrays");
    let blocks = inputs
        .bridge_ineffective_left_stations
        .as_ref()
        .expect("left stations");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0], vec![30.0]);

    let out = serde_json::to_string(&inputs).unwrap();
    let again: SteadyInputs = serde_json::from_str(&out).unwrap();
    assert_eq!(
        again.bridge_ineffective_left_stations,
        inputs.bridge_ineffective_left_stations
    );
}

#[test]
fn cross_section_blocked_obstructions_deserialize() {
    let json = r#"{
        "station": 100.0,
        "x": [0.0, 0.0, 10.0, 10.0],
        "y": [5.0, 0.0, 0.0, 5.0],
        "n_stations": [0.0],
        "n_values": [0.03],
        "unit_system": "Metric",
        "blocked_obstructions": [
            { "stations": [2.0, 8.0], "elevations": [1.5, 1.5] }
        ]
    }"#;
    let xs: CrossSection = serde_json::from_str(json).expect("blocked obstruction XS");
    let blocks = xs.blocked_obstructions.as_ref().expect("blocks");
    assert_eq!(blocks[0].stations, vec![2.0, 8.0]);
    let row = xs.to_metric().compute_properties_at_elevation(2.0);
    let open = CrossSection {
        station: 100.0,
        x: vec![0.0, 0.0, 10.0, 10.0],
        y: vec![5.0, 0.0, 0.0, 5.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        blocked_obstructions: None,
    }
    .to_metric()
    .compute_properties_at_elevation(2.0);
    assert!(row.area < open.area);
}

#[test]
fn bridge_abutment_per_side_fields_deserialize() {
    let json = r#"{
        "cross_sections": [{
            "station": 0.0,
            "x": [0.0, 0.0, 10.0, 10.0],
            "y": [5.0, 0.0, 0.0, 5.0],
            "n_stations": [0.0],
            "n_values": [0.03],
            "unit_system": "Metric"
        }, {
            "station": 100.0,
            "x": [0.0, 0.0, 10.0, 10.0],
            "y": [5.0, 0.0, 0.0, 5.0],
            "n_stations": [0.0],
            "n_values": [0.03],
            "unit_system": "Metric"
        }],
        "flow_rate": 10.0,
        "regime": 0,
        "downstream_wsel": 1.5,
        "bridge_stations": [50.0],
        "bridge_low_chords": [5.0],
        "bridge_high_chords": [7.0],
        "bridge_low_flow_methods": [4],
        "bridge_abutment_left_widths": [1.0],
        "bridge_abutment_right_widths": [4.0],
        "bridge_abutment_left_top_elevations": [0.0],
        "bridge_abutment_right_top_elevations": [2.5],
        "bridge_abutment_left_top_profile_stations": [[0.0, 1.0]],
        "bridge_abutment_left_top_profile_elevations": [[0.0, 0.0]]
    }"#;
    let inputs: SteadyInputs = serde_json::from_str(json).expect("per-side abutment fields");
    assert_eq!(inputs.bridge_abutment_left_widths.as_ref().unwrap()[0], 1.0);
    assert_eq!(inputs.bridge_abutment_right_widths.as_ref().unwrap()[0], 4.0);
    assert_eq!(
        inputs.bridge_abutment_right_top_elevations.as_ref().unwrap()[0],
        2.5
    );
    let profiles = inputs
        .bridge_abutment_left_top_profile_stations
        .as_ref()
        .unwrap();
    assert_eq!(profiles[0], vec![0.0, 1.0]);

    let result = solve_steady(&inputs);
    assert_eq!(result.wsel.len(), 2);

    let out = serde_json::to_string(&inputs).unwrap();
    let again: SteadyInputs = serde_json::from_str(&out).unwrap();
    assert_eq!(
        again.bridge_abutment_left_widths,
        inputs.bridge_abutment_left_widths
    );
    assert_eq!(
        again.bridge_abutment_right_top_elevations,
        inputs.bridge_abutment_right_top_elevations
    );
}

#[test]
fn bridge_abutment_per_side_unsteady_deserialize() {
    use streams1d::solvers::{solve_unsteady, UnsteadyInputs};

    let json = r#"{
        "cross_sections": [{
            "station": 100.0,
            "x": [0.0, 0.0, 10.0, 10.0],
            "y": [5.0, 0.0, 0.0, 5.0],
            "n_stations": [0.0],
            "n_values": [0.03],
            "unit_system": "Metric"
        }, {
            "station": 0.0,
            "x": [0.0, 0.0, 10.0, 10.0],
            "y": [5.0, 0.0, 0.0, 5.0],
            "n_stations": [0.0],
            "n_values": [0.03],
            "unit_system": "Metric"
        }],
        "initial_wsel": [2.0, 1.5],
        "initial_q": [15.0, 15.0],
        "dt": 60.0,
        "num_steps": 2,
        "upstream_q_hydrograph": [15.0, 15.0],
        "downstream_wsel_hydrograph": [1.5, 1.5],
        "bridge_stations": [50.0],
        "bridge_low_chords": [5.0],
        "bridge_high_chords": [7.0],
        "bridge_abutment_left_widths": [3.0]
    }"#;
    let inputs: UnsteadyInputs = serde_json::from_str(json).expect("unsteady per-side abutment");
    assert_eq!(
        inputs.bridge.bridge_abutment_left_widths.as_ref().unwrap()[0],
        3.0
    );
    let result = solve_unsteady(&inputs);
    assert_eq!(result.wsel.len(), 2);
}

#[test]
fn bridge_ineffective_nested_arrays_roundtrip() {
    let json = r#"{
        "cross_sections": [{
            "station": 0.0,
            "x": [0.0, 0.0, 10.0, 10.0],
            "y": [5.0, 0.0, 0.0, 5.0],
            "n_stations": [0.0],
            "n_values": [0.03],
            "unit_system": "Metric"
        }],
        "flow_rate": 10.0,
        "regime": 0,
        "downstream_wsel": 1.0,
        "bridge_stations": [5.0],
        "bridge_low_chords": [3.0],
        "bridge_high_chords": [5.0],
        "bridge_ineffective_left_stations": [[20.0, 30.0], [40.0]],
        "bridge_ineffective_left_elevations": [[2.0, 3.5], [3.0]]
    }"#;
    let inputs: SteadyInputs = serde_json::from_str(json).expect("nested ineffective arrays");
    let blocks = inputs
        .bridge_ineffective_left_stations
        .as_ref()
        .expect("left stations");
    assert_eq!(blocks[0], vec![20.0, 30.0]);
    assert_eq!(blocks[1], vec![40.0]);

    let out = serde_json::to_string(&inputs).unwrap();
    assert!(out.contains("[[20.0,30.0],[40.0]]") || out.contains("[[20.0, 30.0], [40.0]]"));
    let again: SteadyInputs = serde_json::from_str(&out).unwrap();
    assert_eq!(
        again.bridge_ineffective_left_stations,
        inputs.bridge_ineffective_left_stations
    );
}
