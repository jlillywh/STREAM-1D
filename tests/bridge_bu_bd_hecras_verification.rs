//! HEC-RAS alignment for explicit BU/BD bridge faces (API v22).
//!
//! Benchmarks: `python/verification/bridge_bu_bd_hecras.json`
//! Layout regression: 3-section reach (BU + internal + BD) vs 2-face baseline.

use stream1d::geometry::CrossSection;
use stream1d::solvers::bridge::{solve_bridge_wsel, BridgeCouplingParams};
use stream1d::solvers::bridge_interior::{
    friction_path_from_interior, interior_from_steady, layout_cuts_for_bridge,
    resolve_bridge_face_solve_geometry, resolve_bridge_face_stations_metric,
    resolve_bridge_friction_length_metric,
};
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
    low_chord_m: f64,
    high_chord_m: f64,
    low_flow_method: i32,
    pier_width_m: f64,
    num_piers: i32,
    #[serde(default = "default_true")]
    use_explicit_faces: bool,
    #[serde(default)]
    opening_width_m: Option<f64>,
    expected_wsel_upstream_m: f64,
    #[serde(default)]
    expected_friction_length_m: Option<f64>,
}

fn default_true() -> bool {
    true
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

fn face_xs(station: f64, bed: f64, width: f64) -> CrossSection {
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

fn yarnell_reach_inputs(
    bu: CrossSection,
    bd: CrossSection,
    internal: Option<Vec<CrossSection>>,
    low_flow_method: i32,
    use_explicit_faces: bool,
) -> SteadyInputs {
    SteadyInputs {
        cross_sections: vec![
            channel_xs(200.0, 0.2, 10.0),
            channel_xs(100.0, 0.1, 10.0),
            channel_xs(0.0, 0.0, 10.0),
        ],
        flow_rate: 15.0,
        num_slices: Some(50),
        regime: 0,
        downstream_wsel: Some(3.0),
        bridge_stations: Some(vec![50.0]),
        bridge_low_chords: Some(vec![5.0]),
        bridge_high_chords: Some(vec![7.0]),
        bridge_pier_widths: Some(vec![0.5]),
        bridge_num_piers: Some(vec![2]),
        bridge_pier_shapes: Some(vec![0]),
        bridge_weir_coeffs: Some(vec![1.44]),
        bridge_orifice_coeffs: Some(vec![0.5]),
        bridge_low_flow_methods: Some(vec![low_flow_method]),
        bridge_upstream_cross_sections: if use_explicit_faces {
            Some(vec![bu])
        } else {
            None
        },
        bridge_downstream_cross_sections: if use_explicit_faces {
            Some(vec![bd])
        } else {
            None
        },
        bridge_internal_cross_sections: internal.map(|v| vec![v]),
        ..Default::default()
    }
}

#[test]
fn bridge_bu_bd_hecras_benchmarks() {
    let file: BenchmarkFile =
        serde_json::from_str(include_str!("../python/verification/bridge_bu_bd_hecras.json"))
            .expect("bridge BU/BD benchmark JSON");

    for case in &file.cases {
        let width = case.opening_width_m.unwrap_or(case.channel_width_m);
        let bu = face_xs(case.bu_station_m, 0.05, width);
        let bd = face_xs(case.bd_station_m, 0.0, width);
        let mut inputs =
            yarnell_reach_inputs(bu, bd, None, case.low_flow_method, case.use_explicit_faces);
        inputs.flow_rate = case.flow_rate_cms;
        inputs.downstream_wsel = Some(case.downstream_wsel_m);
        inputs.bridge_stations = Some(vec![case.bridge_center_station_m]);
        inputs.bridge_pier_widths = Some(vec![case.pier_width_m]);
        inputs.bridge_num_piers = Some(vec![case.num_piers]);

        let interior = interior_from_steady(&inputs, 0);
        if case.use_explicit_faces {
            if let Some(expected_l) = case.expected_friction_length_m {
            let l = resolve_bridge_friction_length_metric(&interior, 0.0, 0.0, UnitSystem::Metric);
            assert!(
                (l - expected_l).abs() < 1e-6,
                "{}: friction length {:.4} m vs expected {:.4} m",
                case.name,
                l,
                expected_l
            );
            }
        }

        let result = solve_steady(&inputs);
        assert_eq!(result.wsel.len(), 3);
        assert!(
            (result.wsel[2] - case.downstream_wsel_m).abs() < 1e-9,
            "{}: downstream BC",
            case.name
        );
        assert!(
            (result.wsel[1] - case.expected_wsel_upstream_m).abs() < file.tolerance_m,
            "{}: upstream approach WSEL calc {:.6} m vs HEC-RAS reference {:.6} m",
            case.name,
            result.wsel[1],
            case.expected_wsel_upstream_m
        );
    }
}

#[test]
fn three_section_bridge_reach_matches_two_face_baseline() {
    let bu = face_xs(52.0, 0.05, 10.0);
    let bd = face_xs(48.0, 0.0, 10.0);
    let internal = face_xs(50.0, 0.025, 10.0);
    let table = |xs: &CrossSection| xs.to_metric().generate_lookup_table(50);
    let table_up = table(&bu);
    let table_down = table(&bd);

    let two_face = yarnell_reach_inputs(bu.clone(), bd.clone(), None, 1, true);
    let three_section = yarnell_reach_inputs(bu, bd, Some(vec![internal.clone()]), 1, true);

    let interior_two = interior_from_steady(&two_face, 0);
    let interior_three = interior_from_steady(&three_section, 0);
    let faces = resolve_bridge_face_stations_metric(
        50.0,
        UnitSystem::Metric,
        interior_two.bu.as_ref(),
        interior_two.bd.as_ref(),
        0.0,
    );

    let cuts_two = layout_cuts_for_bridge(&interior_two, faces, UnitSystem::Metric, None, None);
    let cuts_three = layout_cuts_for_bridge(&interior_three, faces, UnitSystem::Metric, None, None);
    assert_eq!(cuts_two.len(), 2, "2-face layout: BU + BD only");
    assert_eq!(cuts_three.len(), 3, "3-section layout: BU + internal + BD");

    let path_two = friction_path_from_interior(&interior_two, UnitSystem::Metric).unwrap();
    let path_three = friction_path_from_interior(&interior_three, UnitSystem::Metric).unwrap();
    assert!(
        (path_two - 4.0).abs() < 1e-9 && (path_three - 4.0).abs() < 1e-9,
        "collinear internal should preserve BU–BD path length"
    );

    let coupling = BridgeCouplingParams {
        low_flow_method: 1,
        ..Default::default()
    };
    let geo_two = resolve_bridge_face_solve_geometry(
        &interior_two,
        None,
        None,
        None,
        &table_up,
        &table_down,
        0.05,
        0.0,
        UnitSystem::Metric,
        50,
        None,
        None,
        0.0,
        None,
        4.0,
        0.0,
        None,
        None,
        None,
        None,
    );
    let geo_three = resolve_bridge_face_solve_geometry(
        &interior_three,
        None,
        None,
        None,
        &table_up,
        &table_down,
        0.05,
        0.0,
        UnitSystem::Metric,
        50,
        None,
        None,
        0.0,
        None,
        4.0,
        0.0,
        None,
        None,
        None,
        None,
    );
    let hw_two = solve_bridge_wsel(
        15.0,
        5.0,
        7.0,
        0.5,
        2,
        0,
        1.44,
        0.5,
        0.0,
        0.05,
        3.0,
        UnitSystem::Metric,
        &geo_two.table_up,
        &geo_two.table_down,
        &coupling,
        4.0,
        None,
        Some(&geo_two.sections),
    );
    let hw_three = solve_bridge_wsel(
        15.0,
        5.0,
        7.0,
        0.5,
        2,
        0,
        1.44,
        0.5,
        0.0,
        0.05,
        3.0,
        UnitSystem::Metric,
        &geo_three.table_up,
        &geo_three.table_down,
        &coupling,
        4.0,
        None,
        Some(&geo_three.sections),
    );
    assert!(
        (hw_two - hw_three).abs() < 1e-6,
        "BU/BD bridge headwater should match: 2-face {hw_two:.6} vs 3-section {hw_three:.6}"
    );

    let result_two = solve_steady(&two_face);
    let result_three = solve_steady(&three_section);
    assert_eq!(result_two.wsel[2], result_three.wsel[2]);
    assert!(
        (result_two.wsel[1] - result_three.wsel[1]).abs() < 0.005,
        "reach profile may differ slightly with interior node; @100 m: {:.6} vs {:.6}",
        result_two.wsel[1],
        result_three.wsel[1]
    );

    let l_two = resolve_bridge_friction_length_metric(&interior_two, 4.0, 0.0, UnitSystem::Metric);
    let l_three =
        resolve_bridge_friction_length_metric(&interior_three, 4.0, 0.0, UnitSystem::Metric);
    assert!((l_two - l_three).abs() < 1e-9);
}
