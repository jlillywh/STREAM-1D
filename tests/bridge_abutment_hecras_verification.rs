//! HEC-RAS-style verification for per-side bridge abutment geometry.
//!
//! Benchmarks are documented in `python/verification/bridge_abutment_hecras.json`.
//! `expected_a_eff_tw_m2` values are hand-derived from rectangular channel geometry;
//! `expected_wsel_up_m` values are reference solutions for WSPRO (low-flow method 4).

use stream1d::geometry::{CrossSection, GeometryTable};
use stream1d::solvers::bridge::{solve_bridge_wsel, BridgeCouplingParams};
use stream1d::solvers::bridge_abutment::{resolve_abutments, BridgeAbutmentUserInput};
use stream1d::utils::UnitSystem;

#[derive(serde::Deserialize)]
struct BenchmarkFile {
    tolerance_m: f64,
    cases: Vec<BenchmarkCase>,
}

#[derive(serde::Deserialize)]
struct BenchmarkCase {
    name: String,
    #[allow(dead_code)]
    notes: String,
    channel_width_m: f64,
    q_cms: f64,
    tw_m: f64,
    low_chord_m: f64,
    high_chord_m: f64,
    low_flow_method: i32,
    bridge_length_m: f64,
    abutment_left_width_m: f64,
    abutment_right_width_m: f64,
    #[serde(default)]
    abutment_left_top_elevation_m: Option<f64>,
    #[serde(default)]
    abutment_right_top_elevation_m: Option<f64>,
    expected_a_eff_tw_m2: f64,
    expected_wsel_up_m: f64,
}

fn rectangular_table(width: f64, z_bed: f64, num_slices: usize) -> GeometryTable {
    let xs = CrossSection {
        station: 0.0,
        x: vec![0.0, 0.0, width, width],
        y: vec![z_bed + 10.0, z_bed, z_bed, z_bed + 10.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        blocked_obstructions: None,
    ineffective_flow_areas: None,
    };
    xs.generate_lookup_table(num_slices)
}

fn hand_rectangular_a_eff(
    channel_width_m: f64,
    wsel_m: f64,
    z_bed_m: f64,
    abutment_left_width_m: f64,
    abutment_right_width_m: f64,
    left_top_m: Option<f64>,
    right_top_m: Option<f64>,
) -> f64 {
    let abut = resolve_abutments(
        &BridgeAbutmentUserInput {
            left_width: Some(abutment_left_width_m),
            right_width: Some(abutment_right_width_m),
            left_top_elevation: left_top_m,
            right_top_elevation: right_top_m,
            ..Default::default()
        },
        0.0,
        channel_width_m,
        1.0,
        UnitSystem::Metric,
    );
    let a_base = channel_width_m * (wsel_m - z_bed_m).max(0.0);
    (a_base - abut.submerged_area_m2(wsel_m, z_bed_m)).max(1e-5)
}

fn coupling_for_case(case: &BenchmarkCase) -> BridgeCouplingParams {
    BridgeCouplingParams {
        abutment: BridgeAbutmentUserInput {
            left_width: Some(case.abutment_left_width_m),
            right_width: Some(case.abutment_right_width_m),
            left_top_elevation: case.abutment_left_top_elevation_m,
            right_top_elevation: case.abutment_right_top_elevation_m,
            ..Default::default()
        },
        low_flow_method: case.low_flow_method,
        length: case.bridge_length_m,
        ..Default::default()
    }
}

#[test]
fn bridge_abutment_hecras_benchmarks() {
    let file: BenchmarkFile =
        serde_json::from_str(include_str!("../python/verification/bridge_abutment_hecras.json"))
            .expect("bridge abutment benchmark JSON");
    let table = rectangular_table(10.0, 0.0, 50);

    for case in &file.cases {
        let hand_a_eff = hand_rectangular_a_eff(
            case.channel_width_m,
            case.tw_m,
            0.0,
            case.abutment_left_width_m,
            case.abutment_right_width_m,
            case.abutment_left_top_elevation_m,
            case.abutment_right_top_elevation_m,
        );
        assert!(
            (hand_a_eff - case.expected_a_eff_tw_m2).abs() < 1e-6,
            "{}: hand A_eff {:.4} != expected {:.4}",
            case.name,
            hand_a_eff,
            case.expected_a_eff_tw_m2
        );

        let coupling = coupling_for_case(case);
        let hw = solve_bridge_wsel(
            case.q_cms,
            case.low_chord_m,
            case.high_chord_m,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            case.tw_m,
            UnitSystem::Metric,
            &table,
            &table,
            &coupling,
            case.bridge_length_m,
            None,
            None,
        );
        assert!(
            (hw - case.expected_wsel_up_m).abs() < file.tolerance_m,
            "{}: WSPRO headwater calc {:.4} m vs reference {:.4} m",
            case.name,
            hw,
            case.expected_wsel_up_m
        );

        let abut = resolve_abutments(
            &coupling.abutment,
            0.0,
            case.channel_width_m,
            1.0,
            UnitSystem::Metric,
        );
        let a_eff = hand_rectangular_a_eff(
            case.channel_width_m,
            case.tw_m,
            0.0,
            abut.left_width_m(),
            abut.right_width_m(),
            case.abutment_left_top_elevation_m,
            case.abutment_right_top_elevation_m,
        );
        assert!(
            (a_eff - case.expected_a_eff_tw_m2).abs() < 1e-3,
            "{}: resolved A_eff@TW {:.4} != expected {:.4}",
            case.name,
            a_eff,
            case.expected_a_eff_tw_m2
        );
    }
}
