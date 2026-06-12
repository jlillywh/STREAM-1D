//! JSON contract tests for the WASM API (same schema as `serde_wasm_bindgen`).

use stream1d::geometry::CrossSection;
use stream1d::solvers::{solve_steady, SteadyInputs};
use stream1d::utils::UnitSystem;
use stream1d::wasm_api::{build_api_metadata, API_VERSION};

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
    use stream1d::solvers::{compute_culvert_rating_curve, CulvertRatingCurveInputs, CulvertSolveParams};
    use stream1d::utils::UnitSystem;

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
    use stream1d::solvers::{
        compute_bridge_rating_curve, BridgeRatingCurveInputs, BridgeSolveParams,
    };
    use stream1d::utils::UnitSystem;

    let mut bridge = BridgeSolveParams::default();
    bridge.low_chord = 5.0;
    bridge.high_chord = 7.0;
    bridge.z_down = 0.0;
    bridge.z_up = 0.0;
    bridge.tw_wsel = 2.5;
    bridge.units = UnitSystem::Metric;
    bridge.low_flow_method = 3;
    bridge.channel_width = 10.0;
    bridge.manning_n = 0.03;
    let inputs = BridgeRatingCurveInputs {
        q_values: vec![10.0, 20.0],
        bridge,
    };
    let curve = compute_bridge_rating_curve(&inputs);
    assert_eq!(curve.q.len(), 2);
    let json = serde_json::to_string(&curve).unwrap();
    assert!(json.contains("flow_regimes"));
    assert!(json.contains("head_losses"));
}

#[test]
fn wasm_bridge_rating_curve_negative_q_contract() {
    use stream1d::solvers::{
        compute_bridge_rating_curve, BridgeRatingCurveInputs, BridgeSolveParams,
    };
    use stream1d::utils::UnitSystem;

    let mut bridge = BridgeSolveParams::default();
    bridge.low_chord = 5.0;
    bridge.high_chord = 7.0;
    bridge.z_down = 0.0;
    bridge.z_up = 0.0;
    bridge.tw_wsel = 2.5;
    bridge.tw_wsel_reverse = Some(2.6);
    bridge.units = UnitSystem::Metric;
    bridge.low_flow_method = 3;
    bridge.channel_width = 10.0;
    bridge.manning_n = 0.03;
    let curve = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![-20.0, 0.0, 20.0],
        bridge,
    });
    assert_eq!(curve.q.len(), 2, "Q=0 samples are skipped");
    assert!(curve.q.contains(&-20.0));
    assert!(curve.q.contains(&20.0));
    assert!(
        curve.wsel[0] > curve.wsel_down[0],
        "reverse hydraulic HW should exceed TW"
    );
    assert!(!curve.flow_regimes[0].is_empty());
}

#[test]
fn wasm_bridge_rating_curve_bidirectional_qmax_fixture() {
    use stream1d::solvers::{
        compute_bridge_rating_curve, BridgeRatingCurveInputs, BridgeSolveParams,
    };
    use stream1d::utils::UnitSystem;

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/verification/fixtures/bridge_reverse_flow_rating.json"
    );
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("read fixture")).expect("parse");
    let case = &json["cases"][0];
    let q_values: Vec<f64> = case["q_values_cms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let mut bridge = BridgeSolveParams::default();
    bridge.low_chord = case["low_chord_m"].as_f64().unwrap();
    bridge.high_chord = case["high_chord_m"].as_f64().unwrap();
    bridge.tw_wsel = json["tw_wsel_m"].as_f64().unwrap();
    bridge.units = UnitSystem::Metric;
    bridge.low_flow_method = case["low_flow_method"].as_i64().unwrap() as i32;
    bridge.channel_width = case["channel_width_m"].as_f64().unwrap();
    bridge.manning_n = case["manning_n"].as_f64().unwrap();
    let curve = compute_bridge_rating_curve(&BridgeRatingCurveInputs { q_values, bridge });
    assert_eq!(curve.q.len(), 6);
    assert!(curve.wsel.iter().zip(&curve.wsel_down).all(|(hw, tw)| hw > tw));
}

#[test]
fn steady_validation_bridge_opening_width_warning() {
    use stream1d::solvers::{validate_steady_inputs, SteadyInputs};
    use stream1d::geometry::CrossSection;
    use stream1d::utils::UnitSystem;

    let parent = CrossSection {
        station: 50.0,
        x: vec![100.0, 100.0, 130.0, 130.0],
        y: vec![10.0, 0.0, 0.0, 10.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    let mut inputs = SteadyInputs::default();
    inputs.cross_sections = vec![
        parent.clone(),
        CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 200.0, 200.0],
            y: vec![10.0, 0.0, 0.0, 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        },
    ];
    inputs.flow_rate = 10.0;
    inputs.bridge_stations = Some(vec![50.0]);
    inputs.bridge_low_chords = Some(vec![5.0]);
    inputs.bridge_high_chords = Some(vec![7.0]);
    inputs.bridge_deck_stations = Some(vec![vec![0.0, 35.0]]);
    inputs.bridge_deck_low_elevations = Some(vec![vec![5.0, 5.0]]);
    inputs.bridge_deck_high_elevations = Some(vec![vec![7.0, 7.0]]);
    inputs.bridge_opening_reach_station_origins = Some(vec![100.0]);
    inputs.bridge_upstream_cross_sections = Some(vec![parent]);
    let result = validate_steady_inputs(&inputs);
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("exceeds parent"));
}

#[test]
fn steady_validation_guide_bank_polyline_warning() {
    use stream1d::geometry::{GuideBankPolyline, GuideBanks};
    use stream1d::solvers::{validate_steady_inputs, SteadyInputs};
    let mut inputs = SteadyInputs::default();
    inputs.bridge_stations = Some(vec![50.0]);
    inputs.bridge_departure_guide_banks = Some(vec![GuideBanks {
        right_polylines: vec![GuideBankPolyline {
            stations: vec![1.0, 1.0],
            elevations: vec![2.0, 3.0],
        }],
        ..Default::default()
    }]);
    let result = validate_steady_inputs(&inputs);
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("departure"));
}

#[test]
fn wasm_api_metadata_version() {
    let meta = build_api_metadata();
    assert_eq!(meta.api_version, API_VERSION);
    assert_eq!(API_VERSION, 33);
    assert!(meta.culvert_tier1_fields.inputs.contains(&"culvert_inlet_types".to_string()));
    assert_eq!(
        meta.bridge_fields.rating_curve_entry_point,
        "computeBridgeRatingCurve"
    );
    for key in [
        "bridge_upstream_cross_sections",
        "bridge_downstream_cross_sections",
        "bridge_internal_cross_sections",
        "bridge_opening_reach_station_origins",
        "bridge_approach_guide_banks",
        "bridge_departure_guide_banks",
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
        "bridge_roadway_embankments",
        "bridge_pier_top_widths",
        "bridge_pier_bottom_widths",
        "bridge_pier_width_elevations",
        "bridge_pier_width_values",
        "bridge_pier_top_elevations",
        "bridge_pier_base_elevations",
        "bridge_pier_footing_top_elevations",
        "bridge_pier_footing_widths",
        "bridge_pier_footing_bottom_elevations",
        "bridge_pier_nosing_lengths",
        "bridge_pier_nosing_widths",
        "bridge_friction_weighting",
        "bridge_approach_friction_lengths",
        "bridge_departure_friction_lengths",
        "bridge_opening_blockage_factors",
        "bridge_pier_debris_widths",
        "bridge_pier_debris_heights",
        "bridge_ice_thicknesses",
        "bridge_ice_modes",
        "bridge_deck_ice_thicknesses",
    ] {
        assert!(
            meta.bridge_fields.inputs.contains(&key.to_string()),
            "missing bridge metadata field {key}"
        );
    }
    for key in [
        "opening_reach_station_origin",
        "xs_internal",
        "abutment_left_width",
        "abutment_right_width",
        "abutment_left_top_elevation",
        "abutment_right_top_elevation",
        "abutment_left_top_profile_stations",
        "abutment_right_top_profile_elevations",
        "roadway_embankment",
        "pier_top_widths",
        "pier_bottom_widths",
        "pier_width_elevations",
        "pier_width_values",
        "pier_top_elevations",
        "pier_base_elevations",
        "pier_footing_top_elevations",
        "pier_footing_widths",
        "pier_footing_bottom_elevations",
        "pier_nosing_lengths",
        "pier_nosing_widths",
        "friction_weighting",
        "approach_friction_length",
        "departure_friction_length",
        "opening_blockage_factor",
        "pier_debris_widths",
        "pier_debris_heights",
        "ice_thickness",
        "ice_mode",
        "deck_ice_thickness",
        "tw_wsel_reverse",
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
fn wasm_bridge_roadway_embankment_deserializes_and_composes() {
    use stream1d::solvers::bridge_roadway_compose::{
        apply_roadway_embankment_compose_steady, steady_composed_embankment_blocked,
    };

    let json = r#"{
        "cross_sections": [
            {"station": 1000, "x": [0, 10], "y": [0, 0], "n_stations": [0], "n_values": [0.03], "unit_system": "Metric"},
            {"station": 0, "x": [0, 10], "y": [0, 0], "n_stations": [0], "n_values": [0.03], "unit_system": "Metric"}
        ],
        "flow_rate": 10,
        "regime": 0,
        "bridge_stations": [500],
        "bridge_roadway_embankments": [{
            "deck": {
                "stations": [0, 10],
                "low_elevations": [5, 5],
                "high_elevations": [7, 7]
            },
            "left": {
                "embankment_profile": {
                    "stations": [-3, 0],
                    "elevations": [6.5, 7]
                },
                "abutment": { "width": 1.0, "top_elevation": 0 }
            }
        }]
    }"#;
    let mut inputs: SteadyInputs = serde_json::from_str(json).expect("deserialize");
    apply_roadway_embankment_compose_steady(&mut inputs);
    assert_eq!(
        inputs.bridge_deck_stations.as_ref().unwrap()[0],
        vec![0.0, 10.0]
    );
    assert!((inputs.bridge_abutment_left_widths.as_ref().unwrap()[0] - 1.0).abs() < 1e-9);
    let us_left = inputs
        .bridge_ineffective_left_stations_upstream
        .as_ref()
        .unwrap()[0]
        .clone();
    assert_eq!(us_left, vec![-3.0, 0.0]);
    assert!(steady_composed_embankment_blocked(&inputs, 0).is_some());
}

#[test]
fn wasm_unsteady_roadway_embankment_deserializes_and_composes() {
    use stream1d::solvers::bridge_roadway_compose::{
        apply_roadway_embankment_compose_unsteady, unsteady_composed_embankment_blocked,
    };
    use stream1d::solvers::UnsteadyInputs;

    let json = r#"{
        "cross_sections": [
            {"station": 100, "x": [0, 30], "y": [0, 0], "n_stations": [0], "n_values": [0.03], "unit_system": "Metric"},
            {"station": 0, "x": [0, 30], "y": [0, 0], "n_stations": [0], "n_values": [0.03], "unit_system": "Metric"}
        ],
        "initial_wsel": [2.5, 2.5],
        "initial_q": [15, 15],
        "dt": 60,
        "num_steps": 1,
        "upstream_q_hydrograph": [15],
        "downstream_wsel_hydrograph": [2.5],
        "bridge_stations": [50],
        "bridge_roadway_embankments": [{
            "deck": {
                "stations": [0, 10],
                "low_elevations": [5, 5],
                "high_elevations": [6.5, 6.5]
            },
            "left": {
                "embankment_profile": {
                    "stations": [-6, 0],
                    "elevations": [1.5, 6.5]
                }
            }
        }]
    }"#;
    let mut inputs: UnsteadyInputs = serde_json::from_str(json).expect("unsteady roadway JSON");
    apply_roadway_embankment_compose_unsteady(&mut inputs.bridge);
    assert_eq!(
        inputs.bridge.bridge_deck_stations.as_ref().unwrap()[0],
        vec![0.0, 10.0]
    );
    assert_eq!(
        inputs
            .bridge
            .bridge_ineffective_left_stations_upstream
            .as_ref()
            .unwrap()[0],
        vec![-6.0, 0.0]
    );
    assert!(unsteady_composed_embankment_blocked(&inputs.bridge, 0).is_some());
}

#[test]
fn wasm_rating_curve_roadway_embankment_deserializes_and_composes() {
    use stream1d::solvers::bridge_roadway_compose::{
        apply_roadway_embankment_compose_params, rating_composed_embankment_blocked,
    };
    use stream1d::solvers::{compute_bridge_rating_curve, BridgeRatingCurveInputs};

    let json = r#"{
        "q_values": [15.0],
        "low_chord": 5.0,
        "high_chord": 6.5,
        "z_down": 0.0,
        "z_up": 0.0,
        "tw_wsel": 2.5,
        "units": "Metric",
        "low_flow_method": 3,
        "channel_width": 30.0,
        "manning_n": 0.03,
        "opening_reach_station_origin": 10.0,
        "roadway_embankment": {
            "deck": {
                "stations": [0, 10],
                "low_elevations": [5, 5],
                "high_elevations": [6.5, 6.5]
            },
            "left": {
                "embankment_profile": {
                    "stations": [-6, 0],
                    "elevations": [1.5, 6.5]
                }
            },
            "right": {
                "embankment_profile": {
                    "stations": [10, 16],
                    "elevations": [6.5, 1.5]
                }
            }
        }
    }"#;
    let mut inputs: BridgeRatingCurveInputs = serde_json::from_str(json).expect("rating JSON");
    apply_roadway_embankment_compose_params(&mut inputs.bridge);
    assert_eq!(
        inputs.bridge.deck_stations.as_deref(),
        Some(&[0.0, 10.0][..])
    );
    assert!(rating_composed_embankment_blocked(&inputs.bridge).is_some());

    let curve = compute_bridge_rating_curve(&inputs);
    assert_eq!(curve.wsel.len(), 1);
    assert!(curve.wsel[0].is_finite());
}

#[test]
fn wasm_bridge_rating_curve_per_side_abutments() {
    use stream1d::solvers::{
        compute_bridge_rating_curve, BridgeRatingCurveInputs, BridgeSolveParams,
    };
    use stream1d::utils::UnitSystem;

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
    let mut symmetric_bridge = BridgeSolveParams::default();
    symmetric_bridge.low_chord = 5.0;
    symmetric_bridge.high_chord = 7.0;
    symmetric_bridge.z_down = 0.0;
    symmetric_bridge.z_up = 0.0;
    symmetric_bridge.tw_wsel = 2.5;
    symmetric_bridge.units = UnitSystem::Metric;
    symmetric_bridge.low_flow_method = 4;
    symmetric_bridge.abutment_block_width = 5.0;
    symmetric_bridge.channel_width = 10.0;
    symmetric_bridge.manning_n = 0.03;
    let symmetric = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![15.0, 20.0],
        bridge: symmetric_bridge,
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
        ineffective_flow_areas: None,
    guide_banks: None,
    }
    .to_metric()
    .compute_properties_at_elevation(2.0);
    assert!(row.area < open.area);
}

#[test]
fn cross_section_ineffective_flow_areas_deserialize() {
    let json = r#"{
        "station": 100.0,
        "x": [0.0, 0.0, 10.0, 10.0, 30.0, 30.0, 40.0, 40.0],
        "y": [5.0, 0.0, 0.0, 5.0, 0.0, 5.0, 0.0, 5.0],
        "n_stations": [0.0],
        "n_values": [0.03],
        "unit_system": "Metric",
        "is_overbank": [false, false, false, false, true, true, true, true],
        "ineffective_flow_areas": {
            "left_blocks": [{ "station": 30.0, "elevation": 3.0 }],
            "right_blocks": []
        }
    }"#;
    let xs: CrossSection = serde_json::from_str(json).expect("ineffective on XS");
    let areas = xs
        .ineffective_flow_areas
        .as_ref()
        .expect("ineffective_flow_areas");
    assert_eq!(areas.left_blocks.len(), 1);
    assert!((areas.left_blocks[0].station - 30.0).abs() < 1e-9);
}

#[test]
fn cross_section_ineffective_areas_alias_and_pair_arrays() {
    let json = r#"{
        "station": 100.0,
        "x": [0.0, 0.0, 40.0, 40.0],
        "y": [5.0, 0.0, 0.0, 5.0],
        "n_stations": [0.0],
        "n_values": [0.03],
        "unit_system": "Metric",
        "ineffective_areas": {
            "left": [[8.0, 2.5], [12.0, 3.0]],
            "right": [[32.0, 2.8]]
        }
    }"#;
    let xs: CrossSection = serde_json::from_str(json).expect("ineffective_areas alias");
    let areas = xs.ineffective_flow_areas.expect("parsed ineffective");
    assert_eq!(areas.left_blocks.len(), 2);
    assert_eq!(areas.right_blocks.len(), 1);
    assert!((areas.left_blocks[1].elevation - 3.0).abs() < 1e-9);
}

#[test]
fn wasm_steady_bridge_bu_bd_v22_fixture() {
    let json = include_str!("fixtures/wasm_steady_bridge_bu_bd_v22.json");
    let inputs: SteadyInputs = serde_json::from_str(json).expect("v22 bridge fixture");
    assert_eq!(
        inputs
            .bridge_upstream_cross_sections
            .as_ref()
            .map(|v| v.len()),
        Some(1)
    );
    assert_eq!(
        inputs
            .bridge_internal_cross_sections
            .as_ref()
            .and_then(|v| v.first())
            .map(|cuts| cuts.len()),
        Some(1)
    );
    let internal = inputs
        .bridge_internal_cross_sections
        .as_ref()
        .and_then(|v| v.first())
        .and_then(|cuts| cuts.first())
        .expect("internal cut");
    assert!(internal.ineffective_flow_areas.is_some());

    let result = solve_steady(&inputs);
    assert_eq!(result.wsel.len(), inputs.cross_sections.len());
    assert!((result.wsel[2] - 3.0).abs() < 1e-9);
    assert!(result.wsel[1] > 3.0);
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
    use stream1d::solvers::{solve_unsteady, UnsteadyInputs};

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
fn bridge_bu_bd_v22_unsteady_deserialize() {
    use stream1d::solvers::{solve_unsteady, UnsteadyInputs};

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
        "bridge_low_flow_methods": [1],
        "bridge_opening_reach_station_origins": [0.0],
        "bridge_upstream_cross_sections": [{
            "station": 52.0,
            "x": [0.0, 0.0, 10.0, 10.0],
            "y": [10.05, 0.05, 0.05, 10.05],
            "n_stations": [0.0],
            "n_values": [0.03],
            "unit_system": "Metric"
        }],
        "bridge_downstream_cross_sections": [{
            "station": 48.0,
            "x": [0.0, 0.0, 10.0, 10.0],
            "y": [10.0, 0.0, 0.0, 10.0],
            "n_stations": [0.0],
            "n_values": [0.03],
            "unit_system": "Metric"
        }]
    }"#;
    let inputs: UnsteadyInputs = serde_json::from_str(json).expect("unsteady v22 BU/BD");
    assert_eq!(
        inputs
            .bridge
            .bridge_upstream_cross_sections
            .as_ref()
            .map(|v| v.len()),
        Some(1)
    );
    assert_eq!(
        inputs
            .bridge
            .bridge_opening_reach_station_origins
            .as_ref()
            .unwrap()[0],
        0.0
    );
    let result = solve_unsteady(&inputs);
    assert_eq!(result.wsel.len(), 2);
    assert_eq!(result.wsel[0].len(), 2);
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

#[test]
fn wasm_tapered_pier_fields_deserialize_and_solve() {
    use stream1d::solvers::bridge::{compute_bridge_rating_curve, BridgeRatingCurveInputs, BridgeSolveParams};

    let steady_json = r#"{
        "cross_sections": [
            {"station": 100, "x": [0, 10], "y": [0, 0], "n_stations": [0], "n_values": [0.03], "unit_system": "Metric"},
            {"station": 0, "x": [0, 10], "y": [0, 0], "n_stations": [0], "n_values": [0.03], "unit_system": "Metric"}
        ],
        "flow_rate": 15,
        "regime": 0,
        "downstream_wsel": 2.5,
        "bridge_stations": [50],
        "bridge_low_chords": [4],
        "bridge_high_chords": [6],
        "bridge_pier_widths": [1.5],
        "bridge_num_piers": [1],
        "bridge_pier_shapes": [0],
        "bridge_low_flow_methods": [1],
        "bridge_pier_stations": [[5.0]],
        "bridge_pier_top_widths": [[1.0]],
        "bridge_pier_bottom_widths": [[3.0]],
        "bridge_deck_stations": [[0, 10]],
        "bridge_deck_low_elevations": [[4, 4]],
        "bridge_deck_high_elevations": [[6, 6]]
    }"#;
    let inputs: SteadyInputs = serde_json::from_str(steady_json).expect("tapered pier steady JSON");
    assert_eq!(
        inputs.bridge_pier_top_widths.as_ref().unwrap()[0],
        vec![1.0]
    );
    let result = solve_steady(&inputs);
    assert_eq!(result.wsel.len(), 2);
    assert!(result.wsel[0] > 2.5);

    let mut rating_params = BridgeSolveParams::default();
    rating_params.low_chord = 4.0;
    rating_params.high_chord = 6.0;
    rating_params.tw_wsel = 2.5;
    rating_params.pier_width = 1.5;
    rating_params.num_piers = 1;
    rating_params.low_flow_method = 1;
    rating_params.pier_stations = Some(vec![5.0]);
    rating_params.pier_top_widths = Some(vec![1.0]);
    rating_params.pier_bottom_widths = Some(vec![2.0]);
    rating_params.deck_stations = Some(vec![0.0, 10.0]);
    rating_params.deck_low_elevations = Some(vec![4.0, 4.0]);
    rating_params.deck_high_elevations = Some(vec![6.0, 6.0]);
    let curve = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![15.0],
        bridge: rating_params,
    });
    assert_eq!(curve.wsel.len(), 1);
    assert!(curve.wsel[0] > 2.5);
}

#[test]
fn wasm_pier_width_profile_deserializes_and_solves() {
    use stream1d::solvers::bridge::{compute_bridge_rating_curve, BridgeRatingCurveInputs, BridgeSolveParams};

    let steady_json = r#"{
        "cross_sections": [
            {"station": 100, "x": [0, 10], "y": [0, 0], "n_stations": [0], "n_values": [0.03], "unit_system": "Metric"},
            {"station": 0, "x": [0, 10], "y": [0, 0], "n_stations": [0], "n_values": [0.03], "unit_system": "Metric"}
        ],
        "flow_rate": 15,
        "regime": 0,
        "downstream_wsel": 2.5,
        "bridge_stations": [50],
        "bridge_low_chords": [4],
        "bridge_high_chords": [6],
        "bridge_pier_widths": [1.0],
        "bridge_num_piers": [1],
        "bridge_pier_shapes": [0],
        "bridge_low_flow_methods": [1],
        "bridge_pier_stations": [[5.0]],
        "bridge_pier_width_elevations": [[[0.0, 4.0]]],
        "bridge_pier_width_values": [[[2.0, 1.0]]],
        "bridge_deck_stations": [[0, 10]],
        "bridge_deck_low_elevations": [[4, 4]],
        "bridge_deck_high_elevations": [[6, 6]]
    }"#;
    let inputs: SteadyInputs = serde_json::from_str(steady_json).expect("profile pier steady JSON");
    assert_eq!(
        inputs.bridge_pier_width_elevations.as_ref().unwrap()[0][0],
        vec![0.0, 4.0]
    );
    let result = solve_steady(&inputs);
    assert_eq!(result.wsel.len(), 2);
    assert!(result.wsel[0] > 2.5);

    let mut rating_params = BridgeSolveParams::default();
    rating_params.low_chord = 4.0;
    rating_params.high_chord = 6.0;
    rating_params.tw_wsel = 2.5;
    rating_params.pier_width = 1.0;
    rating_params.num_piers = 1;
    rating_params.low_flow_method = 1;
    rating_params.pier_stations = Some(vec![5.0]);
    rating_params.pier_width_elevations = Some(vec![vec![0.0, 4.0]]);
    rating_params.pier_width_values = Some(vec![vec![2.0, 1.0]]);
    rating_params.deck_stations = Some(vec![0.0, 10.0]);
    rating_params.deck_low_elevations = Some(vec![4.0, 4.0]);
    rating_params.deck_high_elevations = Some(vec![6.0, 6.0]);
    let curve = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![15.0],
        bridge: rating_params,
    });
    assert_eq!(curve.wsel.len(), 1);
    assert!(curve.wsel[0] > 2.5);
}

#[test]
fn wasm_tapered_pier_unsteady_deserializes_and_solves() {
    use stream1d::solvers::{solve_unsteady, UnsteadyInputs};

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
        "initial_wsel": [2.5, 2.5],
        "initial_q": [15.0, 15.0],
        "dt": 60.0,
        "num_steps": 2,
        "upstream_q_hydrograph": [15.0, 15.0],
        "downstream_wsel_hydrograph": [2.5, 2.5],
        "bridge_stations": [50.0],
        "bridge_low_chords": [4.0],
        "bridge_high_chords": [6.0],
        "bridge_pier_widths": [1.5],
        "bridge_num_piers": [1],
        "bridge_pier_shapes": [0],
        "bridge_low_flow_methods": [1],
        "bridge_pier_stations": [[5.0]],
        "bridge_pier_top_widths": [[1.0]],
        "bridge_pier_bottom_widths": [[3.0]],
        "bridge_deck_stations": [[0.0, 10.0]],
        "bridge_deck_low_elevations": [[4.0, 4.0]],
        "bridge_deck_high_elevations": [[6.0, 6.0]]
    }"#;
    let inputs: UnsteadyInputs = serde_json::from_str(json).expect("unsteady tapered pier JSON");
    assert_eq!(
        inputs.bridge.bridge_pier_top_widths.as_ref().unwrap()[0],
        vec![1.0]
    );
    let result = solve_unsteady(&inputs);
    assert_eq!(result.wsel.len(), 2);
    assert_eq!(result.wsel[0].len(), 2);
    assert!(result.wsel[0][0] > 2.5);
}

#[test]
fn wasm_footing_nosing_rating_hw_exceeds_shaft_only() {
    use stream1d::solvers::bridge::{compute_bridge_rating_curve, BridgeRatingCurveInputs, BridgeSolveParams};

    let mut shaft = BridgeSolveParams::default();
    shaft.low_chord = 4.0;
    shaft.high_chord = 6.0;
    shaft.tw_wsel = 2.5;
    shaft.pier_width = 1.0;
    shaft.num_piers = 1;
    shaft.low_flow_method = 1;
    shaft.pier_stations = Some(vec![5.0]);
    shaft.pier_top_widths = Some(vec![1.0]);
    shaft.pier_bottom_widths = Some(vec![1.0]);
    shaft.pier_base_elevations = Some(vec![1.0]);
    shaft.deck_stations = Some(vec![0.0, 10.0]);
    shaft.deck_low_elevations = Some(vec![4.0, 4.0]);
    shaft.deck_high_elevations = Some(vec![6.0, 6.0]);

    let mut attach = shaft.clone();
    attach.pier_footing_top_elevations = Some(vec![1.0]);
    attach.pier_footing_widths = Some(vec![3.0]);
    attach.pier_footing_bottom_elevations = Some(vec![0.0]);
    attach.pier_nosing_lengths = Some(vec![0.5]);

    let hw_shaft = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![15.0],
        bridge: shaft,
    })
    .wsel[0];
    let hw_attach = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![15.0],
        bridge: attach,
    })
    .wsel[0];
    assert!(
        hw_attach > hw_shaft + 1e-4,
        "footing+nosing rating HW {hw_attach} vs shaft-only {hw_shaft}"
    );
}

#[test]
fn wasm_pier_footing_nosing_deserialize_and_solve() {
    use stream1d::solvers::bridge::{compute_bridge_rating_curve, BridgeRatingCurveInputs, BridgeSolveParams};

    let steady_json = r#"{
        "cross_sections": [
            {"station": 100, "x": [0, 10], "y": [0, 0], "n_stations": [0], "n_values": [0.03], "unit_system": "Metric"},
            {"station": 0, "x": [0, 10], "y": [0, 0], "n_stations": [0], "n_values": [0.03], "unit_system": "Metric"}
        ],
        "flow_rate": 15,
        "regime": 0,
        "downstream_wsel": 2.5,
        "bridge_stations": [50],
        "bridge_low_chords": [4],
        "bridge_high_chords": [6],
        "bridge_pier_widths": [1.0],
        "bridge_num_piers": [1],
        "bridge_pier_shapes": [0],
        "bridge_low_flow_methods": [1],
        "bridge_pier_stations": [[5.0]],
        "bridge_pier_top_widths": [[1.0]],
        "bridge_pier_bottom_widths": [[1.0]],
        "bridge_pier_base_elevations": [[1.0]],
        "bridge_pier_footing_top_elevations": [[1.0]],
        "bridge_pier_footing_widths": [[3.0]],
        "bridge_pier_footing_bottom_elevations": [[0.0]],
        "bridge_pier_nosing_lengths": [[0.5]],
        "bridge_deck_stations": [[0, 10]],
        "bridge_deck_low_elevations": [[4, 4]],
        "bridge_deck_high_elevations": [[6, 6]]
    }"#;
    let inputs: SteadyInputs = serde_json::from_str(steady_json).expect("footing/nosing steady JSON");
    assert_eq!(
        inputs.bridge_pier_footing_widths.as_ref().unwrap()[0],
        vec![3.0]
    );
    assert_eq!(
        inputs.bridge_pier_nosing_lengths.as_ref().unwrap()[0],
        vec![0.5]
    );
    let result = solve_steady(&inputs);
    assert_eq!(result.wsel.len(), 2);
    assert!(result.wsel[0] > 2.5);

    let mut rating_params = BridgeSolveParams::default();
    rating_params.low_chord = 4.0;
    rating_params.high_chord = 6.0;
    rating_params.tw_wsel = 2.5;
    rating_params.pier_width = 1.0;
    rating_params.num_piers = 1;
    rating_params.low_flow_method = 1;
    rating_params.pier_stations = Some(vec![5.0]);
    rating_params.pier_top_widths = Some(vec![1.0]);
    rating_params.pier_bottom_widths = Some(vec![1.0]);
    rating_params.pier_base_elevations = Some(vec![1.0]);
    rating_params.pier_footing_top_elevations = Some(vec![1.0]);
    rating_params.pier_footing_widths = Some(vec![3.0]);
    rating_params.pier_footing_bottom_elevations = Some(vec![0.0]);
    rating_params.pier_nosing_lengths = Some(vec![0.5]);
    rating_params.deck_stations = Some(vec![0.0, 10.0]);
    rating_params.deck_low_elevations = Some(vec![4.0, 4.0]);
    rating_params.deck_high_elevations = Some(vec![6.0, 6.0]);
    let curve = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![15.0],
        bridge: rating_params,
    });
    assert_eq!(curve.wsel.len(), 1);
    assert!(curve.wsel[0] > 2.5);

    use stream1d::solvers::{solve_unsteady, UnsteadyInputs};

    let unsteady_json = r#"{
        "cross_sections": [
            {"station": 100, "x": [0, 10], "y": [0, 0], "n_stations": [0], "n_values": [0.03], "unit_system": "Metric"},
            {"station": 0, "x": [0, 10], "y": [0, 0], "n_stations": [0], "n_values": [0.03], "unit_system": "Metric"}
        ],
        "initial_wsel": [2.5, 2.5],
        "initial_q": [15.0, 15.0],
        "dt": 60.0,
        "num_steps": 2,
        "upstream_q_hydrograph": [15.0, 15.0],
        "downstream_wsel_hydrograph": [2.5, 2.5],
        "bridge_stations": [50.0],
        "bridge_low_chords": [4.0],
        "bridge_high_chords": [6.0],
        "bridge_pier_widths": [1.0],
        "bridge_num_piers": [1],
        "bridge_pier_shapes": [0],
        "bridge_low_flow_methods": [1],
        "bridge_pier_stations": [[5.0]],
        "bridge_pier_footing_top_elevations": [[1.0]],
        "bridge_pier_footing_widths": [[3.0]],
        "bridge_pier_nosing_lengths": [[0.5]],
        "bridge_deck_stations": [[0.0, 10.0]],
        "bridge_deck_low_elevations": [[4.0, 4.0]],
        "bridge_deck_high_elevations": [[6.0, 6.0]]
    }"#;
    let unsteady: UnsteadyInputs = serde_json::from_str(unsteady_json).expect("footing/nosing unsteady JSON");
    assert_eq!(
        unsteady.bridge.bridge_pier_nosing_lengths.as_ref().unwrap()[0],
        vec![0.5]
    );
    let unsteady_result = solve_unsteady(&unsteady);
    assert_eq!(unsteady_result.wsel.len(), 2);
    assert!(unsteady_result.wsel[0][0] > 2.5);
}
