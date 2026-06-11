//! Steady-profile verification: bridge friction weighting 0 (HEC-RAS default) vs 1 (three segments).
//!
//! Benchmark: `verification/fixtures/bridge_friction_weighting_hecras.json`

use stream1d::geometry::CrossSection;
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
    approach_station_m: f64,
    departure_station_m: f64,
    channel_width_m: f64,
    approach_width_m: f64,
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

fn steady_inputs(case: &BenchmarkCase, friction_weighting: Option<Vec<i32>>) -> SteadyInputs {
    let bu = channel_xs(case.bu_station_m, 0.05, case.channel_width_m);
    let bd = channel_xs(case.bd_station_m, 0.0, case.channel_width_m);
    let approach = channel_xs(case.approach_station_m, 0.1, case.approach_width_m);
    let departure = channel_xs(case.departure_station_m, 0.0, case.approach_width_m);
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
    inputs.coeff_contraction = Some(0.0);
    inputs.coeff_expansion = Some(0.0);
    inputs.bridge_upstream_cross_sections = Some(vec![bu]);
    inputs.bridge_downstream_cross_sections = Some(vec![bd]);
    inputs.bridge_approach_cross_sections = Some(vec![approach]);
    inputs.bridge_departure_cross_sections = Some(vec![departure]);
    inputs.bridge_friction_weighting = friction_weighting;
    inputs
}

/// Upstream approach WSEL on the densified profile (node immediately above BU).
fn upstream_approach_wsel(result: &stream1d::solvers::steady::SteadyResult) -> f64 {
    assert!(
        result.wsel.len() >= 2,
        "expected ≥2 profile nodes, got {}",
        result.wsel.len()
    );
    result.wsel[1]
}

#[test]
#[ignore = "run manually to refresh verification/fixtures/bridge_friction_weighting_hecras.json"]
fn capture_friction_weighting_golden() {
    let file: BenchmarkFile = serde_json::from_str(include_str!(
        "../verification/fixtures/bridge_friction_weighting_hecras.json"
    ))
    .unwrap();
    for case in &file.cases {
        let omitted = solve_steady(&steady_inputs(case, None));
        let opening_only = solve_steady(&steady_inputs(case, Some(vec![0])));
        let segments = solve_steady(&steady_inputs(case, Some(vec![1])));
        eprintln!(
            "{}: omitted={:.9} opening_only={:.9} segments={:.9} len={}",
            case.name,
            upstream_approach_wsel(&omitted),
            upstream_approach_wsel(&opening_only),
            upstream_approach_wsel(&segments),
            omitted.wsel.len()
        );
    }
}

#[test]
fn bridge_friction_weighting_hecras_benchmarks() {
    let file: BenchmarkFile = serde_json::from_str(include_str!(
        "../verification/fixtures/bridge_friction_weighting_hecras.json"
    ))
    .expect("friction weighting benchmark JSON");

    for case in &file.cases {
        let omitted = solve_steady(&steady_inputs(case, None));
        let opening_only = solve_steady(&steady_inputs(case, Some(vec![0])));
        let segments = solve_steady(&steady_inputs(case, Some(vec![1])));
        let hw_omitted = upstream_approach_wsel(&omitted);
        let hw_opening = upstream_approach_wsel(&opening_only);
        let hw_segments = upstream_approach_wsel(&segments);

        assert!(
            (hw_omitted - hw_opening).abs() < file.tolerance_m,
            "{}: omitted (HEC-RAS default) HW {:.6} should match explicit 0 {:.6}",
            case.name,
            hw_omitted,
            hw_opening
        );
        assert!(
            hw_segments > hw_opening,
            "{}: weighting 1 should raise HW vs 0 at same Q (opening={hw_opening:.6}, segments={hw_segments:.6})",
            case.name
        );
        assert!(
            hw_segments > hw_omitted,
            "{}: weighting 1 should raise HW vs omitted default (omitted={hw_omitted:.6}, segments={hw_segments:.6})",
            case.name
        );
    }
}
