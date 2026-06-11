//! Steady-profile verification: approach guide-bank narrowing vs reach-only `coeff_contraction`.
//!
//! Benchmark: `verification/fixtures/bridge_guide_bank_contraction.json`

use stream1d::geometry::{CrossSection, GuideBankToe, GuideBanks};
use stream1d::solvers::{solve_steady, SteadyInputs};
use stream1d::utils::UnitSystem;

#[derive(serde::Deserialize)]
struct BenchmarkFile {
    tolerance_m: f64,
    cases: Vec<BenchmarkCase>,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct BenchmarkCase {
    name: String,
    #[allow(dead_code)]
    notes: String,
    flow_rate_cms: f64,
    downstream_wsel_m: f64,
    bridge_center_station_m: f64,
    bu_station_m: f64,
    bd_station_m: f64,
    channel_width_m: f64,
    approach_width_m: f64,
    guide_left_toe_m: f64,
    guide_right_toe_m: f64,
    coeff_contraction: f64,
    expected_hw_reach_only_m: f64,
    expected_hw_guided_m: f64,
}

fn channel_xs(station: f64, bed: f64, width: f64) -> CrossSection {
    CrossSection {
        station,
        x: vec![0.0, 0.0, width, width],
        y: vec![bed + 10.0, bed, bed, bed + 10.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    }
}

fn steady_inputs(case: &BenchmarkCase, with_guide_banks: bool) -> SteadyInputs {
    let bu = channel_xs(case.bu_station_m, 0.05, case.channel_width_m);
    let bd = channel_xs(case.bd_station_m, 0.0, case.channel_width_m);
    let approach = channel_xs(case.bu_station_m + 10.0, 0.1, case.approach_width_m);
    let mut inputs = SteadyInputs::default();
    inputs.cross_sections = vec![
        channel_xs(200.0, 0.2, case.approach_width_m),
        channel_xs(100.0, 0.1, case.approach_width_m),
        channel_xs(0.0, 0.0, case.channel_width_m),
    ];
    inputs.flow_rate = case.flow_rate_cms;
    inputs.num_slices = Some(50);
    inputs.regime = 0;
    inputs.downstream_wsel = Some(case.downstream_wsel_m);
    inputs.bridge_stations = Some(vec![case.bridge_center_station_m]);
    inputs.bridge_low_chords = Some(vec![5.0]);
    inputs.bridge_high_chords = Some(vec![7.0]);
    inputs.bridge_weir_coeffs = Some(vec![1.44]);
    inputs.bridge_orifice_coeffs = Some(vec![0.5]);
    inputs.bridge_low_flow_methods = Some(vec![3]);
    inputs.coeff_contraction = Some(case.coeff_contraction);
    inputs.coeff_expansion = Some(0.0);
    inputs.bridge_upstream_cross_sections = Some(vec![bu]);
    inputs.bridge_downstream_cross_sections = Some(vec![bd]);
    inputs.bridge_approach_cross_sections = Some(vec![approach]);
    inputs.bridge_approach_guide_banks = if with_guide_banks {
        Some(vec![GuideBanks {
            left_toe: Some(GuideBankToe {
                station: case.guide_left_toe_m,
                elevation: 0.0,
            }),
            right_toe: Some(GuideBankToe {
                station: case.guide_right_toe_m,
                elevation: 0.0,
            }),
            ..Default::default()
        }])
    } else {
        None
    };
    inputs
}

/// Upstream approach WSEL on the densified profile (node immediately above BU).
fn upstream_approach_wsel(result: &stream1d::solvers::steady::SteadyResult) -> f64 {
    assert!(result.wsel.len() >= 2, "expected ≥2 profile nodes, got {}", result.wsel.len());
    result.wsel[1]
}

#[test]
#[ignore = "run manually to refresh verification/fixtures/bridge_guide_bank_contraction.json"]
fn capture_guide_bank_contraction_golden() {
    let file: BenchmarkFile = serde_json::from_str(include_str!(
        "../verification/fixtures/bridge_guide_bank_contraction.json"
    ))
    .unwrap();
    for case in &file.cases {
        let reach_only = solve_steady(&steady_inputs(case, false));
        let guided = solve_steady(&steady_inputs(case, true));
        eprintln!(
            "{}: reach_only={:.9} guided={:.9} len={}",
            case.name,
            upstream_approach_wsel(&reach_only),
            upstream_approach_wsel(&guided),
            reach_only.wsel.len()
        );
    }
}

#[test]
fn bridge_guide_bank_contraction_benchmarks() {
    let file: BenchmarkFile = serde_json::from_str(include_str!(
        "../verification/fixtures/bridge_guide_bank_contraction.json"
    ))
    .expect("guide-bank contraction benchmark JSON");

    for case in &file.cases {
        let reach_only = solve_steady(&steady_inputs(case, false));
        let guided = solve_steady(&steady_inputs(case, true));
        let hw_reach = upstream_approach_wsel(&reach_only);
        let hw_guided = upstream_approach_wsel(&guided);

        assert!(
            (hw_reach - case.expected_hw_reach_only_m).abs() < file.tolerance_m,
            "{}: reach-only HW {:.6} vs expected {:.6}",
            case.name,
            hw_reach,
            case.expected_hw_reach_only_m
        );
        assert!(
            (hw_guided - case.expected_hw_guided_m).abs() < file.tolerance_m,
            "{}: guided HW {:.6} vs expected {:.6}",
            case.name,
            hw_guided,
            case.expected_hw_guided_m
        );
        assert!(
            hw_guided > hw_reach + file.tolerance_m,
            "{}: guide banks should raise approach HW (reach={hw_reach:.6}, guided={hw_guided:.6})",
            case.name
        );
    }
}
