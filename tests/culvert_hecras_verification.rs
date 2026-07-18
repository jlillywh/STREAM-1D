//! HEC-RAS alignment tests for culvert hydraulics.

use stream1d::geometry::CrossSection;
use stream1d::solvers::culvert::{solve_culvert, CulvertSolveParams};
use stream1d::solvers::solve_steady;
use stream1d::solvers::steady::SteadyInputs;
use stream1d::utils::UnitSystem;

#[derive(serde::Deserialize)]
struct ConspanProfilesFile {
    tolerance_ft: f64,
    profiles: Vec<ConspanProfile>,
}

#[derive(serde::Deserialize)]
struct ConspanProfile {
    name: String,
    flow_rate_cfs: f64,
    downstream_wsel_ft: f64,
    expected_wsel_ft: std::collections::HashMap<String, f64>,
}

#[derive(serde::Deserialize)]
struct PointBenchmarksFile {
    tolerance_ft: f64,
    cases: Vec<PointBenchmarkCase>,
}

#[derive(serde::Deserialize)]
struct PointBenchmarkCase {
    name: String,
    shape_type: i32,
    inlet_type: i32,
    span: f64,
    rise: f64,
    roughness_n: f64,
    length: f64,
    entrance_loss_coeff: f64,
    exit_loss_coeff: f64,
    z_up: f64,
    z_down: f64,
    tw_wsel: f64,
    q_cfs: f64,
    num_barrels: i32,
    #[serde(default)]
    active_barrels: i32,
    expected_control: String,
    expected_wsel_ft: f64,
}

fn load_conspan_project() -> SteadyInputs {
    let project_json = include_str!("../verification/fixtures/conspan_project_12.json");
    let v: serde_json::Value = serde_json::from_str(project_json).unwrap();
    build_conspan_inputs_from_json(&v)
}

fn build_conspan_inputs_from_json(v: &serde_json::Value) -> SteadyInputs {
    let xs_raw = v["geometry_data"].as_array().unwrap();
    let mut cross_sections = Vec::new();
    for xs in xs_raw {
        cross_sections.push(CrossSection {
            station: xs["station"].as_f64().unwrap(),
            x: xs["x"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_f64().unwrap())
                .collect(),
            y: xs["y"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_f64().unwrap())
                .collect(),
            n_stations: xs["n_stations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_f64().unwrap())
                .collect(),
            n_values: xs["n_values"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_f64().unwrap())
                .collect(),
            unit_system: UnitSystem::USCustomary,
            is_overbank: xs
                .get("is_overbank")
                .and_then(|v| Some(v.as_array()?.iter().map(|b| b.as_bool().unwrap()).collect())),
            coeff_contraction: None,
            coeff_expansion: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        });
    }

    let culverts = v["culvert_stations"].as_array().unwrap();
    let mut culvert_stations = Vec::new();
    let mut culvert_shape_types = Vec::new();
    let mut culvert_spans = Vec::new();
    let mut culvert_rises = Vec::new();
    let mut culvert_roughness_ns = Vec::new();
    let mut culvert_lengths = Vec::new();
    let mut culvert_entrance_loss_coeffs = Vec::new();
    let mut culvert_exit_loss_coeffs = Vec::new();
    let mut culvert_barrels = Vec::new();
    let mut culvert_roughness_n_bottoms = Vec::new();
    let mut culvert_depth_bottom_ns = Vec::new();
    let mut culvert_depth_blockeds = Vec::new();

    for c in culverts {
        culvert_stations.push(c["station"].as_f64().unwrap());
        culvert_shape_types.push(c["shape_type"].as_i64().unwrap() as i32);
        culvert_spans.push(c["span"].as_f64().unwrap());
        culvert_rises.push(c["rise"].as_f64().unwrap());
        culvert_roughness_ns.push(c["roughness_n"].as_f64().unwrap());
        culvert_lengths.push(c["length"].as_f64().unwrap());
        culvert_entrance_loss_coeffs.push(c["entrance_loss_coeff"].as_f64().unwrap());
        culvert_exit_loss_coeffs.push(c["exit_loss_coeff"].as_f64().unwrap());
        culvert_barrels.push(c["num_barrels"].as_i64().unwrap_or(1) as i32);
        culvert_roughness_n_bottoms.push(
            c["roughness_n_bottom"]
                .as_f64()
                .unwrap_or(c["roughness_n"].as_f64().unwrap()),
        );
        culvert_depth_bottom_ns.push(c["depth_bottom_n"].as_f64().unwrap_or(0.0));
        culvert_depth_blockeds.push(c["depth_blocked"].as_f64().unwrap_or(0.0));
    }

    let params = &v["parameters"];
    let mut inputs = SteadyInputs::default();
    inputs.cross_sections = cross_sections;
    inputs.flow_rate = 1000.0;
    inputs.num_slices = Some(params["vertical_slices"].as_u64().unwrap_or(100) as usize);
    inputs.coeff_contraction = Some(0.1);
    inputs.coeff_expansion = Some(0.3);
    inputs.regime = params["flow_regime"].as_i64().unwrap_or(0) as u8;
    inputs.downstream_wsel = Some(30.51);
    inputs.max_spacing = Some(params["max_spacing"].as_f64().unwrap_or(100.0));
    inputs.downstream_bc_type = Some(0);
    inputs.culvert_stations = Some(culvert_stations);
    inputs.culvert_shape_types = Some(culvert_shape_types);
    inputs.culvert_spans = Some(culvert_spans);
    inputs.culvert_rises = Some(culvert_rises);
    inputs.culvert_roughness_ns = Some(culvert_roughness_ns);
    inputs.culvert_lengths = Some(culvert_lengths);
    inputs.culvert_entrance_loss_coeffs = Some(culvert_entrance_loss_coeffs);
    inputs.culvert_exit_loss_coeffs = Some(culvert_exit_loss_coeffs);
    inputs.culvert_barrels = Some(culvert_barrels);
    inputs.culvert_roughness_n_bottoms = Some(culvert_roughness_n_bottoms);
    inputs.culvert_depth_bottom_ns = Some(culvert_depth_bottom_ns);
    inputs.culvert_depth_blockeds = Some(culvert_depth_blockeds);
    inputs
}

#[test]
fn test_hecras_conspan_profiles() {
    let profiles_json = include_str!("../verification/fixtures/hecras_conspan_profiles.json");
    let profiles_file: ConspanProfilesFile = serde_json::from_str(profiles_json).unwrap();
    let mut base = load_conspan_project();
    let station_list: Vec<f64> = base.cross_sections.iter().map(|xs| xs.station).collect();

    for profile in &profiles_file.profiles {
        base.flow_rate = profile.flow_rate_cfs;
        base.downstream_wsel = Some(profile.downstream_wsel_ft);
        let result = solve_steady(&base);

        for (sta_key, expected) in &profile.expected_wsel_ft {
            let station: f64 = sta_key.parse().unwrap();
            let idx = station_list
                .iter()
                .position(|s| (*s - station).abs() < 0.5)
                .unwrap_or_else(|| panic!("station {} missing for {}", station, profile.name));
            let calc = result.wsel[idx];
            let diff = (calc - expected).abs();
            assert!(
                diff <= profiles_file.tolerance_ft,
                "{} STA {}: calc {:.3} vs HEC-RAS {:.3} (diff {:.3} ft)",
                profile.name,
                station,
                calc,
                expected,
                diff
            );
        }
    }
}

#[test]
#[ignore]
fn calibrate_point_benchmarks_print() {
    let json = include_str!("fixtures/culvert_point_benchmarks.json");
    let file: PointBenchmarksFile = serde_json::from_str(json).unwrap();
    for case in &file.cases {
        let result = solve_culvert(&CulvertSolveParams {
            q: case.q_cfs,
            shape_type: case.shape_type,
            inlet_type: case.inlet_type,
            span: case.span,
            rise: case.rise,
            roughness_n: case.roughness_n,
            length: case.length,
            entrance_loss_coeff: case.entrance_loss_coeff,
            exit_loss_coeff: case.exit_loss_coeff,
            z_down: case.z_down,
            z_up: case.z_up,
            tw_wsel: case.tw_wsel,
            units: UnitSystem::USCustomary,
            manning_n_bottom: case.roughness_n,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: case.num_barrels,
            active_barrels: case.active_barrels,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            ..Default::default()
        });
        eprintln!(
            "{}: control={} wsel={:.3}",
            case.name, result.control_type, result.wsel
        );
    }
}

#[test]
fn test_culvert_point_benchmarks() {
    let json = include_str!("fixtures/culvert_point_benchmarks.json");
    let file: PointBenchmarksFile = serde_json::from_str(json).unwrap();

    for case in &file.cases {
        let result = solve_culvert(&CulvertSolveParams {
            q: case.q_cfs,
            shape_type: case.shape_type,
            inlet_type: case.inlet_type,
            span: case.span,
            rise: case.rise,
            roughness_n: case.roughness_n,
            length: case.length,
            entrance_loss_coeff: case.entrance_loss_coeff,
            exit_loss_coeff: case.exit_loss_coeff,
            z_down: case.z_down,
            z_up: case.z_up,
            tw_wsel: case.tw_wsel,
            units: UnitSystem::USCustomary,
            manning_n_bottom: case.roughness_n,
            depth_bottom_n: 0.0,
            depth_blocked: 0.0,
            ds_velocity: 0.0,
            us_velocity: 0.0,
            crest_elev: None,
            weir_coeff: 0.0,
            weir_length: 0.0,
            num_barrels: case.num_barrels,
            active_barrels: case.active_barrels,
            skew_deg: 0.0,
            barrel_spans: None,
            barrel_rises: None,
            custom_shape_tbl_y: None,
            custom_shape_tbl_area: None,
            custom_shape_tbl_perimeter: None,
            custom_shape_tbl_top_width: None,
            roadway_stations: None,
            ..Default::default()
        });

        assert_eq!(
            result.control_type, case.expected_control,
            "{} control type",
            case.name
        );
        let diff = (result.wsel - case.expected_wsel_ft).abs();
        assert!(
            diff <= file.tolerance_ft,
            "{}: calc {:.3} vs ref {:.3} (diff {:.3} ft)",
            case.name,
            result.wsel,
            case.expected_wsel_ft,
            diff
        );
    }
}
