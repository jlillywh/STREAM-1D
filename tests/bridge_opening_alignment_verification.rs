//! §1.3 — skewed bridge with offset opening origin (anchor + preprocessor + steady solve).

use stream1d::geometry::CrossSection;
use stream1d::solvers::bridge::{solve_bridge_wsel, BridgeCouplingParams};
use stream1d::solvers::bridge_interior::{
    interior_from_steady, opening_station_to_reach_x, resolve_bridge_face_solve_geometry,
    BridgeFaceSolveParams,
};
use stream1d::solvers::{solve_steady, validate_steady_inputs, SteadyInputs};
use stream1d::utils::UnitSystem;

const ORIGIN_OFFSET_M: f64 = 95.0;
const BU_X0_M: f64 = 110.0;
const OPENING_WIDTH_M: f64 = 25.0;
const SKEW_DEG: f64 = 30.0;

fn approach_xs(station: f64) -> CrossSection {
    CrossSection {
        station,
        x: vec![80.0, 80.0, 150.0, 150.0],
        y: vec![10.0, 0.0, 0.0, 10.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        coeff_contraction: None,
        coeff_expansion: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    }
}

fn bu_face(station: f64, bed: f64) -> CrossSection {
    CrossSection {
        station,
        x: vec![BU_X0_M, BU_X0_M, BU_X0_M + 30.0, BU_X0_M + 30.0],
        y: vec![bed + 10.0, bed, bed, bed + 10.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        coeff_contraction: None,
        coeff_expansion: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    }
}

fn bd_face(station: f64, bed: f64) -> CrossSection {
    bu_face(station, bed)
}

fn skew_offset_inputs(
    opening_origin: Option<f64>,
    skew_deg: f64,
    pier_opening_stations: Option<Vec<f64>>,
) -> SteadyInputs {
    let mut inputs = SteadyInputs::default();
    inputs.cross_sections = vec![approach_xs(100.0), approach_xs(0.0)];
    inputs.flow_rate = 15.0;
    inputs.num_slices = Some(50);
    inputs.regime = 0;
    inputs.downstream_wsel = Some(3.0);
    inputs.bridge_stations = Some(vec![50.0]);
    inputs.bridge_low_chords = Some(vec![5.0]);
    inputs.bridge_high_chords = Some(vec![7.0]);
    inputs.bridge_pier_widths = Some(vec![0.5]);
    inputs.bridge_num_piers = Some(vec![2]);
    inputs.bridge_pier_shapes = Some(vec![0]);
    inputs.bridge_weir_coeffs = Some(vec![1.44]);
    inputs.bridge_orifice_coeffs = Some(vec![0.5]);
    inputs.bridge_low_flow_methods = Some(vec![1]);
    inputs.bridge_deck_stations = Some(vec![vec![0.0, OPENING_WIDTH_M]]);
    inputs.bridge_deck_low_elevations = Some(vec![vec![5.0, 5.0]]);
    inputs.bridge_deck_high_elevations = Some(vec![vec![7.0, 7.0]]);
    inputs.bridge_opening_reach_station_origins = opening_origin.map(|o| vec![o]);
    inputs.bridge_skew_angles = Some(vec![skew_deg]);
    inputs.bridge_pier_stations = pier_opening_stations.map(|v| vec![v]);
    inputs.bridge_upstream_cross_sections = Some(vec![bu_face(52.0, 0.05)]);
    inputs.bridge_downstream_cross_sections = Some(vec![bd_face(48.0, 0.0)]);
    inputs
}

fn face_geometry_for(inputs: &SteadyInputs) -> stream1d::solvers::bridge_interior::BridgeFaceSolveGeometry {
    let interior = interior_from_steady(inputs, 0);
    let bu = interior.bu.as_ref().unwrap();
    let bd = interior.bd.as_ref().unwrap();
    let table_up = bu.to_metric().generate_lookup_table(50);
    let table_down = bd.to_metric().generate_lookup_table(50);
    let skew = inputs
        .bridge_skew_angles
        .as_ref()
        .and_then(|v| v.get(0))
        .copied()
        .unwrap_or(0.0);
    resolve_bridge_face_solve_geometry(BridgeFaceSolveParams {
        interior: &interior,
        reach_table_up: &table_up,
        reach_table_down: &table_down,
        reach_z_up_user: 0.05,
        raw_units: UnitSystem::Metric,
        num_slices: 50,
        skew_deg: skew,
        pier_stations: inputs
            .bridge_pier_stations
            .as_ref()
            .and_then(|v| v.get(0))
            .cloned(),
        interval_length_m: 4.0,
        ..BridgeFaceSolveParams::new(&interior, &table_up, &table_down)
    })
}

#[test]
fn offset_origin_remaps_pier_stations_to_reach_frame() {
    let inputs = skew_offset_inputs(Some(ORIGIN_OFFSET_M), 0.0, Some(vec![10.0, 18.0]));
    let geo = face_geometry_for(&inputs);
    assert_eq!(geo.sections.opening_reach_station_origin, Some(ORIGIN_OFFSET_M));
    let piers = geo
        .sections
        .pier_stations
        .as_ref()
        .expect("pier stations remapped");
    assert!((piers[0] - opening_station_to_reach_x(10.0, ORIGIN_OFFSET_M)).abs() < 1e-9);
    assert!((piers[1] - opening_station_to_reach_x(18.0, ORIGIN_OFFSET_M)).abs() < 1e-9);
    assert!(
        (piers[0] - 105.0).abs() < 1e-9,
        "pier 0 should sit at reach x=105, got {}",
        piers[0]
    );
}

#[test]
fn explicit_offset_origin_pier_positions_differ_from_bu_left_inference() {
    let offset_inputs = skew_offset_inputs(Some(ORIGIN_OFFSET_M), 0.0, Some(vec![10.0, 18.0]));
    let inferred_inputs = skew_offset_inputs(None, 0.0, Some(vec![10.0, 18.0]));
    let geo_offset = face_geometry_for(&offset_inputs);
    let geo_inferred = face_geometry_for(&inferred_inputs);

    assert_eq!(
        geo_inferred.sections.opening_reach_station_origin,
        Some(BU_X0_M),
        "BU left inference should use min(BU.x)"
    );
    let piers_offset = geo_offset.sections.pier_stations.as_ref().unwrap();
    let piers_inferred = geo_inferred.sections.pier_stations.as_ref().unwrap();
    assert!((piers_offset[0] - 105.0).abs() < 1e-9);
    assert!((piers_inferred[0] - 120.0).abs() < 1e-9);
    assert!((piers_offset[0] - piers_inferred[0]).abs() > 10.0);

    let steady_offset = solve_steady(&offset_inputs);
    let steady_inferred = solve_steady(&inferred_inputs);
    assert!(steady_offset.wsel[0].is_finite());
    assert!(steady_inferred.wsel[0].is_finite());
}

#[test]
fn skew_with_offset_origin_increases_headwater_vs_no_skew() {
    let plain = solve_steady(&skew_offset_inputs(Some(ORIGIN_OFFSET_M), 0.0, Some(vec![10.0, 18.0])));
    let skewed = solve_steady(&skew_offset_inputs(
        Some(ORIGIN_OFFSET_M),
        SKEW_DEG,
        Some(vec![10.0, 18.0]),
    ));
    assert_eq!(plain.wsel.len(), 2);
    assert!(
        skewed.wsel[0] >= plain.wsel[0],
        "30° skew should raise upstream headwater with offset origin (plain={:.4}, skew={:.4})",
        plain.wsel[0],
        skewed.wsel[0]
    );
}

#[test]
fn offset_origin_equivalent_pier_reach_position_matches_headwater() {
    let coupling = BridgeCouplingParams {
        low_flow_method: 1,
        ..Default::default()
    };
    // Pier at reach x=105: origin 95 + opening 10, or origin 100 + opening 5.
    let a = skew_offset_inputs(Some(95.0), 0.0, Some(vec![10.0, 18.0]));
    let mut b = skew_offset_inputs(Some(100.0), 0.0, Some(vec![5.0, 13.0]));
    b.bridge_upstream_cross_sections = a.bridge_upstream_cross_sections.clone();
    b.bridge_downstream_cross_sections = a.bridge_downstream_cross_sections.clone();

    let geo_a = face_geometry_for(&a);
    let geo_b = face_geometry_for(&b);
    let piers_a = geo_a.sections.pier_stations.as_ref().unwrap();
    let piers_b = geo_b.sections.pier_stations.as_ref().unwrap();
    assert!((piers_a[0] - piers_b[0]).abs() < 1e-9);
    assert!((piers_a[1] - piers_b[1]).abs() < 1e-9);

    let hw_a = solve_bridge_wsel(
        15.0,
        5.0,
        7.0,
        0.5,
        2,
        0,
        1.44,
        0.5,
        geo_a.z_down_user,
        geo_a.z_up_user,
        3.0,
        UnitSystem::Metric,
        &geo_a.table_up,
        &geo_a.table_down,
        &coupling,
        4.0,
        None,
        Some(&geo_a.sections),
    );
    let hw_b = solve_bridge_wsel(
        15.0,
        5.0,
        7.0,
        0.5,
        2,
        0,
        1.44,
        0.5,
        geo_b.z_down_user,
        geo_b.z_up_user,
        3.0,
        UnitSystem::Metric,
        &geo_b.table_up,
        &geo_b.table_down,
        &coupling,
        4.0,
        None,
        Some(&geo_b.sections),
    );
    assert!(
        (hw_a - hw_b).abs() < 1e-6,
        "equivalent reach pier positions should yield same headwater (a={hw_a}, b={hw_b})"
    );
}

#[test]
fn skew_offset_validation_passes_when_opening_inside_parent_bu() {
    // Deck-only span: opening 20–35 → reach 115–130 inside BU x 110–140 (no pier extent in validation).
    let mut inputs = SteadyInputs::default();
    inputs.cross_sections = vec![approach_xs(100.0), approach_xs(0.0)];
    inputs.flow_rate = 15.0;
    inputs.bridge_stations = Some(vec![50.0]);
    inputs.bridge_low_chords = Some(vec![5.0]);
    inputs.bridge_high_chords = Some(vec![7.0]);
    inputs.bridge_deck_stations = Some(vec![vec![20.0, 35.0]]);
    inputs.bridge_deck_low_elevations = Some(vec![vec![5.0, 5.0]]);
    inputs.bridge_deck_high_elevations = Some(vec![vec![7.0, 7.0]]);
    inputs.bridge_opening_reach_station_origins = Some(vec![ORIGIN_OFFSET_M]);
    inputs.bridge_skew_angles = Some(vec![SKEW_DEG]);
    inputs.bridge_upstream_cross_sections = Some(vec![bu_face(52.0, 0.05)]);
    let result = validate_steady_inputs(&inputs);
    assert!(
        result.warnings.is_empty(),
        "expected no warnings, got: {:?}",
        result.warnings
    );
}

#[test]
fn skew_offset_validation_warns_when_deck_exceeds_parent_bu() {
    let mut inputs = skew_offset_inputs(Some(ORIGIN_OFFSET_M), SKEW_DEG, Some(vec![10.0, 18.0]));
    inputs.bridge_deck_stations = Some(vec![vec![0.0, 40.0]]);
    inputs.bridge_deck_low_elevations = Some(vec![vec![5.0, 5.0]]);
    inputs.bridge_deck_high_elevations = Some(vec![vec![7.0, 7.0]]);
    let result = validate_steady_inputs(&inputs);
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("exceeds parent"));
}
