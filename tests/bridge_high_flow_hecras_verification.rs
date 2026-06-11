//! HEC-RAS-style regression for bridge high-flow hydraulics (Phase 4 acceptance).
//!
//! Benchmarks: `verification/fixtures/bridge_high_flow_hecras.json`
//! Tolerance: ±2 mm headwater (same as other bridge HEC fixtures).
//!
//! Covers sluice-gate pressure, submerged orifice, low-flow → pressure reconcile,
//! combined pressure + weir, crowned-deck segment weir, and explicit energy high flow.
//! Discharge balance at solved WSEL is covered in `src/solvers/bridge_tests.rs`.

use stream1d::geometry::{CrossSection, GeometryTable};
use stream1d::solvers::bridge::{
    build_bridge_deck_profile, solve_bridge_coupled, BridgeCouplingParams, BridgeDeckProfile,
};
use stream1d::utils::UnitSystem;

#[derive(serde::Deserialize)]
struct BenchmarkFile {
    tolerance_m: f64,
    #[allow(dead_code)]
    tolerance_q_cms: f64,
    #[allow(dead_code)]
    reference: String,
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
    high_flow_method: i32,
    bridge_length_m: f64,
    orifice_coeff: f64,
    weir_coeff: f64,
    expected_wsel_up_m: f64,
    expected_flow_regime: String,
    #[serde(default)]
    verify_q_balance: bool,
    #[serde(default)]
    deck_stations_m: Option<Vec<f64>>,
    #[serde(default)]
    deck_low_elevations_m: Option<Vec<f64>>,
    #[serde(default)]
    deck_high_elevations_m: Option<Vec<f64>>,
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
        guide_banks: None,
    };
    xs.generate_lookup_table(num_slices)
}

fn deck_for_case(case: &BenchmarkCase) -> Option<BridgeDeckProfile> {
    build_bridge_deck_profile(
        case.low_chord_m,
        case.high_chord_m,
        case.deck_stations_m.as_deref(),
        case.deck_low_elevations_m.as_deref(),
        case.deck_high_elevations_m.as_deref(),
        UnitSystem::Metric,
    )
}

fn coupling_for_case(case: &BenchmarkCase) -> BridgeCouplingParams {
    BridgeCouplingParams {
        low_flow_method: case.low_flow_method,
        high_flow_method: case.high_flow_method,
        length: case.bridge_length_m,
        ..Default::default()
    }
}

#[test]
fn bridge_high_flow_hecras_benchmarks() {
    let file: BenchmarkFile =
        serde_json::from_str(include_str!("../verification/fixtures/bridge_high_flow_hecras.json"))
            .expect("bridge high-flow benchmark JSON");
    assert!(
        file.cases.len() >= 5,
        "acceptance requires ≥5 HEC-RAS bridge regression cases, got {}",
        file.cases.len()
    );

    for case in &file.cases {
        let table = rectangular_table(case.channel_width_m, 0.0, 50);
        let deck = deck_for_case(case);
        let coupling = coupling_for_case(case);
        let result = solve_bridge_coupled(
            case.q_cms,
            case.low_chord_m,
            case.high_chord_m,
            0.0,
            0,
            0,
            case.weir_coeff,
            case.orifice_coeff,
            0.0,
            0.0,
            case.tw_m,
            UnitSystem::Metric,
            &table,
            &table,
            &coupling,
            case.bridge_length_m,
            deck.as_ref(),
            None,
        );

        assert!(
            (result.wsel_up - case.expected_wsel_up_m).abs() < file.tolerance_m,
            "{}: headwater {:.9} m vs golden {:.9} m (tol {:.3} mm)",
            case.name,
            result.wsel_up,
            case.expected_wsel_up_m,
            file.tolerance_m * 1000.0
        );
        assert_eq!(
            result.flow_regime, case.expected_flow_regime,
            "{}: flow_regime",
            case.name
        );
        let _ = case.verify_q_balance;
    }
}

#[test]
fn bridge_high_flow_hecras_case_count() {
    let file: BenchmarkFile =
        serde_json::from_str(include_str!("../verification/fixtures/bridge_high_flow_hecras.json"))
            .expect("bridge high-flow benchmark JSON");
    assert!(
        file.cases.len() >= 5,
        "Phase 4 acceptance: ≥5 HEC-RAS bridge high-flow regression cases"
    );
}
