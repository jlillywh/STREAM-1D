//! Roadway embankment fill via `bridge_roadway_embankments` — no manual `blocked_obstructions` on BU/BD.
//!
//! Benchmark: `verification/fixtures/bridge_roadway_embankment.json`

use stream1d::geometry::CrossSection;
use stream1d::solvers::bridge_roadway_compose::{
    composed_steady_inputs, steady_composed_embankment_blocked, BridgeDeckInput,
    BridgeRoadwayEmbankment, EmbankmentPolyline, RoadwayEmbankmentSide,
};
use stream1d::solvers::bridge_interior::{
    interior_from_steady, resolve_bridge_face_solve_geometry, BridgeFaceSolveParams,
};
use stream1d::solvers::steady::bridge_ineffective_downstream_for;
use stream1d::solvers::steady::bridge_ineffective_upstream_for;
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
    opening_origin_m: f64,
    left_blocked_stations_m: Vec<f64>,
    left_blocked_elevations_m: Vec<f64>,
    right_blocked_stations_m: Vec<f64>,
    right_blocked_elevations_m: Vec<f64>,
    expected_upstream_wsel_m: f64,
}

/// 30 m trapezoid channel cut — intentionally has **no** `blocked_obstructions`.
fn channel_face(station: f64) -> CrossSection {
    CrossSection {
        station,
        x: vec![0.0, 0.0, 30.0, 30.0],
        y: vec![8.0, 0.0, 0.0, 8.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    }
}

fn typical_roadway_embankment() -> BridgeRoadwayEmbankment {
    BridgeRoadwayEmbankment {
        deck: BridgeDeckInput {
            stations: vec![0.0, 10.0],
            low_elevations: vec![5.0, 5.0],
            high_elevations: vec![6.5, 6.5],
        },
        left: Some(RoadwayEmbankmentSide {
            embankment_profile: Some(EmbankmentPolyline {
                stations: vec![-6.0, 0.0],
                elevations: vec![1.5, 6.5],
            }),
            ..Default::default()
        }),
        right: Some(RoadwayEmbankmentSide {
            embankment_profile: Some(EmbankmentPolyline {
                stations: vec![10.0, 16.0],
                elevations: vec![6.5, 1.5],
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn base_bridge_steady_inputs(
    bu: CrossSection,
    bd: CrossSection,
    embankment: Option<BridgeRoadwayEmbankment>,
) -> SteadyInputs {
    let mut inputs = SteadyInputs::default();
    inputs.cross_sections = vec![
        channel_face(200.0),
        channel_face(100.0),
        channel_face(0.0),
    ];
    inputs.flow_rate = 15.0;
    inputs.num_slices = Some(50);
    inputs.regime = 0;
    inputs.downstream_wsel = Some(2.5);
    inputs.bridge_stations = Some(vec![50.0]);
    inputs.bridge_lengths = Some(vec![4.0]);
    inputs.bridge_low_chords = Some(vec![5.0]);
    inputs.bridge_high_chords = Some(vec![6.5]);
    inputs.bridge_low_flow_methods = Some(vec![3]);
    inputs.bridge_opening_reach_station_origins = Some(vec![10.0]);
    inputs.bridge_upstream_cross_sections = Some(vec![bu]);
    inputs.bridge_downstream_cross_sections = Some(vec![bd]);
    inputs.bridge_roadway_embankments = embankment.map(|e| vec![Some(e)]);
    inputs
}

/// Flat-field equivalent of unified compose: composed deck/ineffective/blocked runtime cache,
/// without `bridge_roadway_embankments`. Explicit BU/BD stay clean; blocked merges at solve
/// (same as unified — not baked into reach layout nodes).
fn decomposed_parity_steady_inputs(
    bu: CrossSection,
    bd: CrossSection,
    embankment: BridgeRoadwayEmbankment,
) -> SteadyInputs {
    let mut manual = composed_steady_inputs(&base_bridge_steady_inputs(
        bu.clone(),
        bd.clone(),
        Some(embankment),
    ));
    manual.bridge_roadway_embankments = None;
    manual.bridge_upstream_cross_sections = Some(vec![bu]);
    manual.bridge_downstream_cross_sections = Some(vec![bd]);
    manual
}

#[test]
fn explicit_bu_has_no_manual_blocked_before_compose() {
    let bu = channel_face(52.0);
    assert!(bu.blocked_obstructions.is_none());
    let inputs = base_bridge_steady_inputs(
        bu,
        channel_face(48.0),
        Some(typical_roadway_embankment()),
    );
    assert!(
        inputs.bridge_upstream_cross_sections.as_ref().unwrap()[0]
            .blocked_obstructions
            .is_none()
    );
}

#[test]
fn compose_derives_blocked_profiles_without_manual_polylines() {
    let inputs = base_bridge_steady_inputs(
        channel_face(52.0),
        channel_face(48.0),
        Some(typical_roadway_embankment()),
    );
    let composed = composed_steady_inputs(&inputs);
    let blocked = steady_composed_embankment_blocked(&composed, 0).expect("composed blocked");
    let left = blocked.left.as_ref().expect("left profile");
    assert_eq!(left.stations, vec![-6.0, 0.0]);
    assert_eq!(left.elevations, vec![1.5, 6.5]);
}

#[test]
fn resolve_geometry_merges_embankment_blocked_onto_bu() {
    let bu = channel_face(52.0);
    let bd = channel_face(48.0);
    let inputs = composed_steady_inputs(&base_bridge_steady_inputs(
        bu.clone(),
        bd.clone(),
        Some(typical_roadway_embankment()),
    ));
    let interior = interior_from_steady(&inputs, 0);
    let reach = channel_face(100.0);
    let table = reach.generate_lookup_table(50);
    let blocked = steady_composed_embankment_blocked(&inputs, 0).expect("blocked");

    let geo = resolve_bridge_face_solve_geometry(BridgeFaceSolveParams {
        interior: &interior,
        reach_xs_up: Some(&reach),
        reach_xs_down: Some(&reach),
        reach_table_up: &table,
        reach_table_down: &table,
        raw_units: UnitSystem::Metric,
        num_slices: 50,
        ineffective_up: bridge_ineffective_upstream_for(&inputs, 0),
        ineffective_down: bridge_ineffective_downstream_for(&inputs, 0),
        interval_length_m: 4.0,
        embankment_blocked: Some(&blocked),
        ..BridgeFaceSolveParams::new(&interior, &table, &table)
    });

    let bu_resolved = geo.sections.xs_up.expect("BU xs");
    let blocks = bu_resolved
        .blocked_obstructions
        .as_ref()
        .expect("merged blocked on BU");
    assert_eq!(blocks.len(), 2);
    assert!((blocks[0].stations[0] - 4.0).abs() < 1e-9);
    assert!((blocks[0].stations[1] - 10.0).abs() < 1e-9);
    assert!((blocks[1].stations[0] - 20.0).abs() < 1e-9);
    assert!((blocks[1].stations[1] - 26.0).abs() < 1e-9);

    let wsel = 2.5_f64;
    let row_open = table.interpolate(wsel);
    let row_fill = geo.table_up.interpolate(wsel);
    assert!(
        row_fill.active_area < row_open.active_area,
        "roadway fill should reduce active area at TW (open={:.4}, fill={:.4})",
        row_open.active_area,
        row_fill.active_area
    );
}

#[test]
fn roadway_fill_raises_upstream_wsel_without_manual_blocked_input() {
    let bu = channel_face(52.0);
    let bd = channel_face(48.0);
    let open = base_bridge_steady_inputs(bu.clone(), bd.clone(), None);
    let filled = base_bridge_steady_inputs(bu, bd, Some(typical_roadway_embankment()));

    let w_open = solve_steady(&open).wsel[0];
    let w_fill = solve_steady(&filled).wsel[0];
    assert!(
        w_fill > w_open,
        "embankment fill should raise upstream WSEL (open={w_open:.4}, fill={w_fill:.4})"
    );
}

#[test]
fn unified_embankment_matches_manual_blocked_and_ineffective() {
    let bu = channel_face(52.0);
    let bd = channel_face(48.0);
    let unified = base_bridge_steady_inputs(
        bu.clone(),
        bd.clone(),
        Some(typical_roadway_embankment()),
    );

    let emb = typical_roadway_embankment();
    let manual = decomposed_parity_steady_inputs(bu, bd, emb);

    let w_unified = solve_steady(&unified).wsel[0];
    let w_manual = solve_steady(&manual).wsel[0];
    assert!(
        (w_unified - w_manual).abs() < 2e-3,
        "unified embankment should match decomposed flat fields + runtime blocked cache (unified={w_unified:.6}, manual={w_manual:.6})"
    );
}

#[test]
fn interpolated_bu_gets_embankment_fill_without_manual_polylines() {
    let mut open = SteadyInputs::default();
    open.cross_sections = vec![channel_face(200.0), channel_face(0.0)];
    open.flow_rate = 15.0;
    open.num_slices = Some(50);
    open.regime = 0;
    open.downstream_wsel = Some(2.5);
    open.bridge_stations = Some(vec![50.0]);
    open.bridge_lengths = Some(vec![4.0]);
    open.bridge_low_chords = Some(vec![5.0]);
    open.bridge_high_chords = Some(vec![6.5]);
    open.bridge_low_flow_methods = Some(vec![3]);
    open.bridge_opening_reach_station_origins = Some(vec![10.0]);
    let mut filled = open.clone();
    filled.bridge_roadway_embankments = Some(vec![Some(typical_roadway_embankment())]);

    let composed = composed_steady_inputs(&filled);
    assert!(
        steady_composed_embankment_blocked(&composed, 0).is_some(),
        "interpolated BU/BD should still compose embankment blocked profiles"
    );

    let w_open = solve_steady(&open).wsel[0];
    let w_fill = solve_steady(&filled).wsel[0];
    assert!(
        w_fill > w_open,
        "interpolated layout: embankment fill should raise upstream WSEL (open={w_open:.4}, fill={w_fill:.4})"
    );
}

#[test]
fn benchmark_json_typical_roadway_fill() {
    let file: BenchmarkFile =
        serde_json::from_str(include_str!("../verification/fixtures/bridge_roadway_embankment.json"))
            .expect("benchmark JSON");
    let case = file
        .cases
        .iter()
        .find(|c| c.name == "typical_overbank_fill")
        .expect("case");

    let bu = channel_face(52.0);
    let bd = channel_face(48.0);
    let emb = typical_roadway_embankment();
    let inputs = base_bridge_steady_inputs(bu.clone(), bd.clone(), Some(emb.clone()));
    let w_unified = solve_steady(&inputs).wsel[0];

    let blocked = steady_composed_embankment_blocked(&composed_steady_inputs(&inputs), 0)
        .expect("composed blocked");
    let left = blocked.left.expect("left blocked profile");
    let right = blocked.right.expect("right blocked profile");
    let origin = case.opening_origin_m;
    for (got, want) in left
        .stations
        .iter()
        .zip(case.left_blocked_stations_m.iter())
    {
        assert!((got + origin - want).abs() < 1e-9);
    }
    for (got, want) in right
        .stations
        .iter()
        .zip(case.right_blocked_stations_m.iter())
    {
        assert!((got + origin - want).abs() < 1e-9);
    }

    let w_manual = solve_steady(&decomposed_parity_steady_inputs(bu, bd, emb)).wsel[0];

    assert!(
        (w_unified - w_manual).abs() < file.tolerance_m,
        "benchmark parity unified vs manual (unified={w_unified:.6}, manual={w_manual:.6})"
    );
    assert!(
        w_unified > 2.5,
        "roadway fill should backwater above tailwater (wsel={w_unified:.6})"
    );
    if case.expected_upstream_wsel_m > 0.0 {
        assert!(
            (w_unified - case.expected_upstream_wsel_m).abs() < file.tolerance_m,
            "upstream WSEL {:.6} vs regression {:.6}",
            w_unified,
            case.expected_upstream_wsel_m
        );
    }
}
