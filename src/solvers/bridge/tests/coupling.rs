use super::*;

#[test]
fn deck_profile_stations_remapped_to_reach_frame() {
    let mut deck = BridgeDeckProfile {
        stations_m: vec![0.0, 30.0],
        low_elevations_m: vec![5.0, 5.0],
        high_elevations_m: vec![7.0, 7.0],
    };
    deck.remap_stations_to_reach(100.0, UnitSystem::Metric);
    assert!((deck.stations_m[0] - 100.0).abs() < 1e-9);
    assert!((deck.stations_m[1] - 130.0).abs() < 1e-9);
}
#[test]
fn test_solve_bridge_wsel_yarnell_integration() {
    let table_up = rectangular_table(10.0, 0.1, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 1,
        ..Default::default()
    };
    let wsel_up = solve_bridge_wsel(
        15.0,
        5.0,
        7.0,
        0.5,
        2,
        0,
        1.44,
        0.5,
        0.0,
        0.1,
        3.0,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        100.0,
        None,
        None,
    );
    assert!((wsel_up - 3.00247).abs() < 0.001);
}
#[test]
fn test_solve_bridge_wsel_energy_no_obstructions() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        ..Default::default()
    };
    let wsel_up = solve_bridge_wsel(
        20.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        2.5,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        None,
    );
    assert!(
        wsel_up > 2.5,
        "energy method should raise upstream WSEL above tailwater, got {wsel_up}"
    );
}
#[test]
fn test_energy_friction_weighting_segments_raise_hw() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let approach = CrossSection {
        station: 70.0,
        x: vec![0.0, 0.0, 20.0, 20.0],
        y: vec![5.0, 0.0, 0.0, 5.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        coeff_contraction: None,
        coeff_expansion: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    let departure = CrossSection {
        station: 30.0,
        x: vec![0.0, 0.0, 20.0, 20.0],
        y: vec![5.0, 0.0, 0.0, 5.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        coeff_contraction: None,
        coeff_expansion: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    let sections = BridgeSectionContext {
        friction_length_m: 20.0,
        friction_lengths: BridgeFrictionLengths {
            weighting: BridgeFrictionWeighting::HecRasSegments,
            opening_m: 20.0,
            approach_m: 30.0,
            departure_m: 25.0,
        },
        xs_approach: Some(approach),
        xs_departure: Some(departure),
        ..Default::default()
    };
    let coupling_opening = BridgeCouplingParams {
        low_flow_method: 3,
        friction_weighting: BridgeFrictionWeighting::OpeningOnly,
        ..Default::default()
    };
    let coupling_segments = BridgeCouplingParams {
        low_flow_method: 3,
        friction_weighting: BridgeFrictionWeighting::HecRasSegments,
        ..Default::default()
    };
    let args = |coupling: &BridgeCouplingParams, sections: Option<&BridgeSectionContext>| {
        solve_bridge_wsel(
            20.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            coupling,
            20.0,
            None,
            sections,
        )
    };
    let hw_opening = args(&coupling_opening, Some(&sections));
    let hw_segments = args(&coupling_segments, Some(&sections));
    assert!(
        hw_segments > hw_opening,
        "HEC-RAS segment friction should raise HW: opening={hw_opening}, segments={hw_segments}"
    );
}
#[test]
fn test_friction_weighting_default_equals_opening_only_at_same_q() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let approach = CrossSection {
        station: 70.0,
        x: vec![0.0, 0.0, 20.0, 20.0],
        y: vec![5.0, 0.0, 0.0, 5.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        coeff_contraction: None,
        coeff_expansion: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    let departure = CrossSection {
        station: 30.0,
        x: vec![0.0, 0.0, 20.0, 20.0],
        y: vec![5.0, 0.0, 0.0, 5.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        coeff_contraction: None,
        coeff_expansion: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    let sections = BridgeSectionContext {
        friction_length_m: 20.0,
        friction_lengths: BridgeFrictionLengths {
            weighting: BridgeFrictionWeighting::HecRasSegments,
            opening_m: 20.0,
            approach_m: 30.0,
            departure_m: 25.0,
        },
        xs_approach: Some(approach),
        xs_departure: Some(departure),
        ..Default::default()
    };
    let mut coupling_default = BridgeCouplingParams::default();
    coupling_default.low_flow_method = 3;
    let coupling_opening = BridgeCouplingParams {
        low_flow_method: 3,
        friction_weighting: BridgeFrictionWeighting::OpeningOnly,
        ..Default::default()
    };
    let args = |coupling: &BridgeCouplingParams| {
        solve_bridge_wsel(
            20.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            coupling,
            20.0,
            None,
            Some(&sections),
        )
    };
    let hw_default = args(&coupling_default);
    let hw_opening = args(&coupling_opening);
    assert!(
        (hw_default - hw_opening).abs() < 1e-9,
        "omitted/default weighting should match explicit 0: default={hw_default}, opening={hw_opening}"
    );
}
#[test]
fn test_wspro_friction_weighting_segments_raise_hw() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let approach = CrossSection {
        station: 70.0,
        x: vec![0.0, 0.0, 20.0, 20.0],
        y: vec![5.0, 0.0, 0.0, 5.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        coeff_contraction: None,
        coeff_expansion: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    let departure = CrossSection {
        station: 30.0,
        x: vec![0.0, 0.0, 20.0, 20.0],
        y: vec![5.0, 0.0, 0.0, 5.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        coeff_contraction: None,
        coeff_expansion: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    let sections = BridgeSectionContext {
        friction_length_m: 20.0,
        friction_lengths: BridgeFrictionLengths {
            weighting: BridgeFrictionWeighting::HecRasSegments,
            opening_m: 20.0,
            approach_m: 30.0,
            departure_m: 25.0,
        },
        xs_approach: Some(approach),
        xs_departure: Some(departure),
        ..Default::default()
    };
    let coupling_opening = BridgeCouplingParams {
        low_flow_method: 4,
        abutment: BridgeAbutmentUserInput {
            legacy_total_width: 2.0,
            ..Default::default()
        },
        friction_weighting: BridgeFrictionWeighting::OpeningOnly,
        ..Default::default()
    };
    let coupling_segments = BridgeCouplingParams {
        low_flow_method: 4,
        abutment: BridgeAbutmentUserInput {
            legacy_total_width: 2.0,
            ..Default::default()
        },
        friction_weighting: BridgeFrictionWeighting::HecRasSegments,
        ..Default::default()
    };
    let solve = |coupling: &BridgeCouplingParams| {
        solve_bridge_wsel(
            20.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            coupling,
            20.0,
            None,
            Some(&sections),
        )
    };
    let hw_opening = solve(&coupling_opening);
    let hw_segments = solve(&coupling_segments);
    assert!(
        hw_segments > hw_opening,
        "WSPRO segment friction should raise HW: opening={hw_opening}, segments={hw_segments}"
    );
}
#[test]
fn test_wspro_uses_min_bu_bd_opening_area() {
    let table_bu = rectangular_table(10.0, 0.0, 50);
    let table_bd = rectangular_table(5.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 4,
        ..Default::default()
    };
    let bu_xs = CrossSection {
        station: 0.0,
        x: vec![0.0, 0.0, 10.0, 10.0],
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
    };
    let bd_xs = CrossSection {
        station: 0.0,
        x: vec![0.0, 0.0, 5.0, 5.0],
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
    };
    let sections = BridgeSectionContext {
        xs_up: Some(bu_xs),
        xs_down: Some(bd_xs),
        ..Default::default()
    };
    let hw_asymmetric = solve_bridge_wsel(
        15.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        2.5,
        UnitSystem::Metric,
        &table_bu,
        &table_bd,
        &coupling,
        50.0,
        None,
        Some(&sections),
    );
    let bu_xs_wide = CrossSection {
        station: 0.0,
        x: vec![0.0, 0.0, 10.0, 10.0],
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
    };
    let sections_symmetric = BridgeSectionContext {
        xs_up: Some(bu_xs_wide.clone()),
        xs_down: Some(bu_xs_wide),
        ..Default::default()
    };
    let hw_symmetric_wide = solve_bridge_wsel(
        15.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        2.5,
        UnitSystem::Metric,
        &table_bu,
        &table_bu,
        &coupling,
        50.0,
        None,
        Some(&sections_symmetric),
    );
    assert!(
        hw_asymmetric > hw_symmetric_wide,
        "WSPRO should use min(BU,BD) opening; narrower BD raises headwater"
    );
}
#[test]
fn test_flat_deck_profile_matches_scalar_chords() {
    let deck = build_bridge_deck_profile(
        5.0,
        7.0,
        Some(&[0.0, 10.0, 20.0]),
        Some(&[5.0, 5.0, 5.0]),
        Some(&[7.0, 7.0, 7.0]),
        UnitSystem::Metric,
    )
    .expect("flat profile");
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams::default();
    let hw_scalar = solve_bridge_wsel(
        15.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        3.0,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        None,
    );
    let hw_profile = solve_bridge_wsel(
        15.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        3.0,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        Some(&deck),
        None,
    );
    assert!(
        (hw_scalar - hw_profile).abs() < 0.01,
        "flat profile should match scalar chords, scalar={hw_scalar}, profile={hw_profile}"
    );
}
#[test]
fn test_deck_profile_hump_raises_headwater_at_low_flow() {
    let deck = build_bridge_deck_profile(
        5.0,
        7.0,
        Some(&[0.0, 10.0, 20.0]),
        Some(&[5.0, 6.5, 5.0]),
        Some(&[7.0, 7.5, 7.0]),
        UnitSystem::Metric,
    )
    .expect("humped profile");
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams::default();
    let hw_flat = solve_bridge_wsel(
        25.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        3.0,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        None,
    );
    let hw_profile = solve_bridge_wsel(
        25.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        3.0,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        Some(&deck),
        None,
    );
    assert!(
        hw_profile >= hw_flat,
        "center deck hump should not reduce headwater, flat={hw_flat}, profile={hw_profile}"
    );
    assert_eq!(deck.low_elevations_m.iter().cloned().fold(f64::INFINITY, f64::min), 5.0);
    assert_eq!(deck.low_elevations_m.iter().cloned().fold(f64::NEG_INFINITY, f64::max), 6.5);
}
#[test]
fn test_ineffective_flow_raises_bridge_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        ..Default::default()
    };
    let xs = CrossSection {
        station: 100.0,
        x: vec![0.0, 0.0, 10.0, 10.0, 30.0, 30.0, 40.0, 40.0],
        y: vec![5.0, 0.0, 0.0, 5.0, 0.0, 5.0, 0.0, 5.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: Some(vec![false, false, false, false, true, true, true, true]),
        blocked_obstructions: None,
        coeff_contraction: None,
        coeff_expansion: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    let sections_none = BridgeSectionContext::default();
    let ineff =
        IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap();
    let sections_ineff = BridgeSectionContext {
        ineffective_up: Some(ineff.clone()),
        ineffective_down: Some(ineff),
        xs_up: Some(xs.clone()),
        xs_down: Some(xs),
        ..Default::default()
    };

    let hw_none = solve_bridge_wsel(
        20.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        2.5,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        Some(&sections_none),
    );
    let hw_ineff = solve_bridge_wsel(
        20.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        2.5,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        Some(&sections_ineff),
    );
    assert!(
        hw_ineff >= hw_none,
        "ineffective left overbank should raise or maintain headwater, none={hw_none}, ineff={hw_ineff}"
    );
}
#[test]
fn test_bu_section_ineffective_raises_bridge_headwater() {
    use crate::solvers::bridge_interior::{
        BridgeFaceSolveParams, BridgeInteriorInput, resolve_bridge_face_solve_geometry,
    };

    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        ..Default::default()
    };
    let reach = CrossSection {
        station: 100.0,
        x: vec![0.0, 0.0, 10.0, 10.0, 30.0, 30.0, 40.0, 40.0],
        y: vec![5.0, 0.0, 0.0, 5.0, 0.0, 5.0, 0.0, 5.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: Some(vec![false, false, false, false, true, true, true, true]),
        blocked_obstructions: None,
        coeff_contraction: None,
        coeff_expansion: None,
        ineffective_flow_areas: None,
    guide_banks: None,
    };
    let mut bu = reach.clone();
    bu.ineffective_flow_areas = Some(
        IneffectiveFlowAreas::from_block_pairs(&[], &[], &[30.0], &[3.0]).unwrap(),
    );
    let interior_none = BridgeInteriorInput {
        bu: Some(reach.clone()),
        bd: Some(reach.clone()),
        ..Default::default()
    };
    let geo_none = resolve_bridge_face_solve_geometry(BridgeFaceSolveParams {
        interior: &interior_none,
        reach_xs_up: Some(&reach),
        reach_xs_down: Some(&reach),
        reach_table_up: &table_up,
        reach_table_down: &table_down,
        ..BridgeFaceSolveParams::new(&interior_none, &table_up, &table_down)
    });
    let interior_bu_ineff = BridgeInteriorInput {
        bu: Some(bu),
        bd: Some(reach.clone()),
        ..Default::default()
    };
    let geo_bu_ineff = resolve_bridge_face_solve_geometry(BridgeFaceSolveParams {
        interior: &interior_bu_ineff,
        reach_xs_up: Some(&reach),
        reach_xs_down: Some(&reach),
        reach_table_up: &table_up,
        reach_table_down: &table_down,
        ..BridgeFaceSolveParams::new(&interior_bu_ineff, &table_up, &table_down)
    });

    let hw_none = solve_bridge_wsel(
        20.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        2.0,
        UnitSystem::Metric,
        &geo_none.table_up,
        &geo_none.table_down,
        &coupling,
        50.0,
        None,
        Some(&geo_none.sections),
    );
    let hw_bu = solve_bridge_wsel(
        20.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        2.0,
        UnitSystem::Metric,
        &geo_bu_ineff.table_up,
        &geo_bu_ineff.table_down,
        &coupling,
        50.0,
        None,
        Some(&geo_bu_ineff.sections),
    );
    assert!(
        hw_bu >= hw_none,
        "BU section ineffective should raise headwater, none={hw_none}, bu={hw_bu}"
    );
}
#[test]
fn test_longer_bu_bd_friction_length_raises_energy_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        ..Default::default()
    };
    let mut sections_short = BridgeSectionContext::default();
    sections_short.friction_length_m = 4.0;
    let mut sections_long = BridgeSectionContext::default();
    sections_long.friction_length_m = 40.0;

    let hw_short = solve_bridge_wsel(
        20.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        2.5,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        Some(&sections_short),
    );
    let hw_long = solve_bridge_wsel(
        20.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        2.5,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        Some(&sections_long),
    );
    assert!(
        hw_long > hw_short,
        "longer BUâ€“BD friction reach should raise energy headwater, short={hw_short}, long={hw_long}"
    );
}
#[test]
fn test_multi_block_ineffective_raises_bridge_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        ..Default::default()
    };
    let xs = CrossSection {
        station: 100.0,
        x: vec![0.0, 0.0, 10.0, 10.0, 20.0, 20.0, 30.0, 30.0, 40.0, 40.0],
        y: vec![5.0, 0.0, 0.0, 5.0, 0.0, 5.0, 0.0, 5.0, 0.0, 5.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: Some(vec![
            false, false, false, false, true, true, true, true, true, true,
        ]),
        blocked_obstructions: None,
        coeff_contraction: None,
        coeff_expansion: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    let single = BridgeSectionContext {
        ineffective_up: Some(
            IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap(),
        ),
        ineffective_down: Some(
            IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap(),
        ),
        xs_up: Some(xs.clone()),
        xs_down: Some(xs.clone()),
        ..Default::default()
    };
    let multi = BridgeSectionContext {
        ineffective_up: Some(
            IneffectiveFlowAreas::from_block_pairs(&[20.0, 30.0], &[2.0, 3.5], &[], &[])
                .unwrap(),
        ),
        ineffective_down: Some(
            IneffectiveFlowAreas::from_block_pairs(&[20.0, 30.0], &[2.0, 3.5], &[], &[])
                .unwrap(),
        ),
        xs_up: Some(xs.clone()),
        xs_down: Some(xs),
        ..Default::default()
    };

    let hw_single = solve_bridge_wsel(
        20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
        UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, Some(&single),
    );
    let hw_multi = solve_bridge_wsel(
        20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
        UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, Some(&multi),
    );
    assert!(
        hw_multi >= hw_single,
        "inner ineffective block should raise headwater, single={hw_single}, multi={hw_multi}"
    );
}
#[test]
fn test_separate_us_ds_ineffective_elevations() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        ..Default::default()
    };
    let xs = CrossSection {
        station: 100.0,
        x: vec![0.0, 0.0, 10.0, 10.0, 30.0, 30.0, 40.0, 40.0],
        y: vec![5.0, 0.0, 0.0, 5.0, 0.0, 5.0, 0.0, 5.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: Some(vec![false, false, false, false, true, true, true, true]),
        blocked_obstructions: None,
        coeff_contraction: None,
        coeff_expansion: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    let sections_split = BridgeSectionContext {
        ineffective_up: Some(
            IneffectiveFlowAreas::from_block_pairs(&[30.0], &[2.0], &[], &[]).unwrap(),
        ),
        ineffective_down: Some(
            IneffectiveFlowAreas::from_block_pairs(&[30.0], &[5.0], &[], &[]).unwrap(),
        ),
        xs_up: Some(xs.clone()),
        xs_down: Some(xs.clone()),
        ..Default::default()
    };
    let sections_shared = BridgeSectionContext {
        ineffective_up: Some(
            IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap(),
        ),
        ineffective_down: Some(
            IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap(),
        ),
        xs_up: Some(xs.clone()),
        xs_down: Some(xs),
        ..Default::default()
    };

    let hw_split = solve_bridge_wsel(
        20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
        UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, Some(&sections_split),
    );
    let hw_shared = solve_bridge_wsel(
        20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
        UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, Some(&sections_shared),
    );
    assert!(
        hw_split >= hw_shared,
        "lower upstream ineffective activation should raise headwater, split={hw_split}, shared={hw_shared}"
    );
}
#[test]
fn test_apply_bridge_skew_geometry() {
    let (w, l) = apply_bridge_skew(0.0, 20.0, 50.0);
    assert!((w - 20.0).abs() < 1e-6);
    assert!((l - 50.0).abs() < 1e-6);
    let (w30, l30) = apply_bridge_skew(30.0, 20.0, 50.0);
    assert!(w30 < w);
    assert!(l30 > l);
}
#[test]
fn test_bridge_skew_increases_low_flow_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        ..Default::default()
    };
    let plain = BridgeSectionContext::default();
    let skewed = BridgeSectionContext {
        skew_deg: 25.0,
        ..Default::default()
    };
    let hw_plain = solve_bridge_wsel(
        20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
        UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, Some(&plain),
    );
    let hw_skew = solve_bridge_wsel(
        20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
        UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, Some(&skewed),
    );
    assert!(
        hw_skew >= hw_plain,
        "skew should raise headwater via longer friction path, plain={hw_plain}, skew={hw_skew}"
    );
}
#[test]
fn test_explicit_pier_stations_increase_headwater_vs_even_spacing() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 1,
        ..Default::default()
    };
    let deck = build_bridge_deck_profile(
        5.0,
        7.0,
        Some(&[0.0, 20.0]),
        Some(&[5.0, 5.0]),
        Some(&[7.0, 7.0]),
        UnitSystem::Metric,
    )
    .unwrap();
    let two_piers = BridgeSectionContext {
        pier_stations: Some(vec![6.0, 14.0]),
        ..Default::default()
    };
    let three_piers = BridgeSectionContext {
        pier_stations: Some(vec![4.0, 10.0, 16.0]),
        ..Default::default()
    };
    let hw_two = solve_bridge_wsel(
        15.0, 5.0, 7.0, 0.5, 2, 0, 1.44, 0.5, 0.0, 0.0, 3.0,
        UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, Some(&deck), Some(&two_piers),
    );
    let hw_three = solve_bridge_wsel(
        15.0, 5.0, 7.0, 0.5, 3, 0, 1.44, 0.5, 0.0, 0.0, 3.0,
        UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, Some(&deck), Some(&three_piers),
    );
    assert!(
        hw_three > hw_two,
        "more pier stations should increase Yarnell headwater, two={hw_two}, three={hw_three}"
    );
}
#[test]
fn test_guide_banks_narrow_approach_raises_energy_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let sections = approach_sections_with_guide_banks(20.0, 8.0, 12.0);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        length: 50.0,
        ..Default::default()
    };
    let args = (
        20.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        2.5,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
    );
    let hw_plain = solve_bridge_wsel(
        args.0, args.1, args.2, args.3, args.4, args.5, args.6, args.7, args.8, args.9,
        args.10, args.11, args.12, args.13, args.14, args.15, args.16, None,
    );
    let hw_guided = solve_bridge_wsel(
        args.0, args.1, args.2, args.3, args.4, args.5, args.6, args.7, args.8, args.9,
        args.10, args.11, args.12, args.13, args.14, args.15, args.16, Some(&sections),
    );
    assert!(
        hw_guided > hw_plain + 0.001,
        "guide banks should raise headwater via contraction loss, plain={hw_plain}, guided={hw_guided}"
    );
}
#[test]
fn test_approach_narrowing_vs_reach_only_contraction_coefficient() {
    let q = 20.0;
    let tw = 2.5;
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let sections = approach_sections_with_guide_banks(20.0, 8.0, 12.0);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        coeff_contraction: 0.3,
        coeff_expansion: 0.0,
        length: 50.0,
        ..Default::default()
    };
    let solve = |sections: Option<&BridgeSectionContext>| {
        solve_bridge_wsel(
            q,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            tw,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            None,
            sections,
        )
    };
    let hw_reach_only = solve(None);
    let hw_guided = solve(Some(&sections));
    let geom_guided = build_bridge_geometry(
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        UnitSystem::Metric,
        &coupling,
        50.0,
        None,
        Some(&sections),
    );

    let wsel = hw_guided;
    let a_bu = obstructed_hydraulics(&table_up, wsel, geom_guided.z_up_m, &geom_guided, true).a_eff;
    let a_guided = reach_cut_flow_area(&geom_guided, true, wsel).expect("guided approach area");
    let opening_wsel = wsel.min(tw).min(geom_guided.low_chord_m);
    let (props_open, _) =
        obstructed_opening_at_wsel(&geom_guided, &table_up, &table_down, opening_wsel);
    let a_open = props_open.a_eff.max(1e-5);
    let k = coupling.coeff_contraction;

    let h_reach_only =
        k * (velocity_head(q, a_bu) - velocity_head(q, a_open)).max(0.0);
    let h_approach_narrowed =
        k * (velocity_head(q, a_guided) - velocity_head(q, a_open)).max(0.0);

    assert!(
        a_guided < a_bu - 5.0,
        "guided channel should be materially narrower than BU reach face: guided={a_guided}, bu={a_bu}"
    );
    assert!(
        h_approach_narrowed > h_reach_only + 1e-4,
        "approach narrowing should increase coeff_contraction loss: reach={h_reach_only}, guided={h_approach_narrowed}"
    );
    assert!(
        hw_guided > hw_reach_only + 1e-4,
        "narrowed approach should raise headwater vs reach-only contraction: reach_hw={hw_reach_only}, guided_hw={hw_guided}"
    );

    // At the reach-only solution, applying the narrowed approach loss would require more headwater.
    let a_guided_at_reach_hw =
        reach_cut_flow_area(&geom_guided, true, hw_reach_only).expect("guided area at reach HW");
    let h_extra = k
        * (velocity_head(q, a_guided_at_reach_hw) - velocity_head(q, a_bu)).max(0.0);
    assert!(
        h_extra > 1e-4,
        "incremental contraction at reach-only HW should be positive"
    );
    assert!(
        (hw_guided - hw_reach_only - h_extra).abs() < 0.02,
        "HW delta should track incremental approach contraction (delta={:.4}, h_extra={:.4})",
        hw_guided - hw_reach_only,
        h_extra
    );
}
#[test]
fn test_wspro_guide_banks_narrow_approach_raises_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let sections = approach_sections_with_guide_banks(20.0, 8.0, 12.0);
    let coupling = BridgeCouplingParams {
        low_flow_method: 4,
        abutment: BridgeAbutmentUserInput {
            legacy_total_width: 1.0,
            ..Default::default()
        },
        length: 50.0,
        ..Default::default()
    };
    let args = (
        20.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, 2.5,
        UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None,
    );
    let hw_plain = solve_bridge_wsel(
        args.0, args.1, args.2, args.3, args.4, args.5, args.6, args.7, args.8, args.9,
        args.10, args.11, args.12, args.13, args.14, args.15, args.16, None,
    );
    let hw_guided = solve_bridge_wsel(
        args.0, args.1, args.2, args.3, args.4, args.5, args.6, args.7, args.8, args.9,
        args.10, args.11, args.12, args.13, args.14, args.15, args.16, Some(&sections),
    );
    assert!(
        hw_guided > hw_plain + 0.001,
        "WSPRO with guide banks should raise HW, plain={hw_plain}, guided={hw_guided}"
    );
}
#[test]
fn approach_ineffective_raises_energy_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let sections_open = BridgeSectionContext {
        xs_approach: Some(compound_overbank_approach(false)),
        friction_length_m: 50.0,
        ..approach_sections_with_ineffective_overbank()
    };
    let sections_ineff = approach_sections_with_ineffective_overbank();
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        coeff_contraction: 0.3,
        length: 50.0,
        ..Default::default()
    };
    let solve = |sections: Option<&BridgeSectionContext>| {
        solve_bridge_wsel(
            20.0,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            None,
            sections,
        )
    };
    let hw_open = solve(Some(&sections_open));
    let hw_ineff = solve(Some(&sections_ineff));
    assert!(
        hw_ineff > hw_open + 1e-4,
        "approach overbank ineffective should raise HW via contraction loss: open={hw_open}, ineff={hw_ineff}"
    );
}
#[test]
fn test_tapered_vs_mean_constant_solve_yarnell_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 10.0],
        low_elevations_m: vec![4.0, 4.0],
        high_elevations_m: vec![6.0, 6.0],
    };
    let coupling = BridgeCouplingParams {
        low_flow_method: 1,
        ..Default::default()
    };
    let mean_sections = BridgeSectionContext {
        pier_stations: Some(vec![5.0]),
        ..Default::default()
    };
    let taper_sections = BridgeSectionContext {
        pier_stations: Some(vec![5.0]),
        pier_widths: Some(PierWidthUserInput {
            top_widths: Some(vec![1.0]),
            bottom_widths: Some(vec![2.0]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let solve = |sections: &BridgeSectionContext, pier_width: f64| {
        solve_bridge_wsel(
            15.0,
            4.0,
            6.0,
            pier_width,
            1,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            Some(&deck),
            Some(sections),
        )
    };
    let hw_mean = solve(&mean_sections, 1.5);
    let hw_taper = solve(&taper_sections, 1.5);
    assert!(
        hw_taper > hw_mean + 1e-4,
        "partial-depth Yarnell: taper HW {hw_taper} vs mean-width constant {hw_mean}"
    );
}
#[test]
fn test_tapered_pier_solve_yarnell_raises_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 10.0],
        low_elevations_m: vec![4.0, 4.0],
        high_elevations_m: vec![6.0, 6.0],
    };
    let coupling = BridgeCouplingParams {
        low_flow_method: 1,
        ..Default::default()
    };
    let rect_sections = BridgeSectionContext {
        pier_stations: Some(vec![5.0]),
        ..Default::default()
    };
    let taper_sections = BridgeSectionContext {
        pier_stations: Some(vec![5.0]),
        pier_widths: Some(PierWidthUserInput {
            top_widths: Some(vec![1.0]),
            bottom_widths: Some(vec![2.0]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let args = |sections: &BridgeSectionContext| {
        solve_bridge_wsel(
            15.0,
            4.0,
            6.0,
            1.0,
            1,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            Some(&deck),
            Some(sections),
        )
    };
    let hw_rect = args(&rect_sections);
    let hw_taper = args(&taper_sections);
    assert!(
        hw_taper > hw_rect + 1e-4,
        "tapered Yarnell HW {hw_taper} vs rectangular {hw_rect}"
    );
}
#[test]
fn test_profile_pier_solve_yarnell_raises_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 10.0],
        low_elevations_m: vec![4.0, 4.0],
        high_elevations_m: vec![6.0, 6.0],
    };
    let coupling = BridgeCouplingParams {
        low_flow_method: 1,
        ..Default::default()
    };
    let rect_sections = BridgeSectionContext {
        pier_stations: Some(vec![5.0]),
        ..Default::default()
    };
    let profile_sections = BridgeSectionContext {
        pier_stations: Some(vec![5.0]),
        pier_widths: Some(PierWidthUserInput {
            width_elevations: Some(vec![vec![0.0, 4.0]]),
            width_values: Some(vec![vec![2.0, 1.0]]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let solve = |sections: &BridgeSectionContext| {
        solve_bridge_wsel(
            15.0,
            4.0,
            6.0,
            1.0,
            1,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            Some(&deck),
            Some(sections),
        )
    };
    let hw_rect = solve(&rect_sections);
    let hw_profile = solve(&profile_sections);
    assert!(
        hw_profile > hw_rect + 1e-4,
        "profile Yarnell HW {hw_profile} vs rectangular {hw_rect}"
    );
}
#[test]
fn test_tapered_pier_exceeds_legacy_constant_headwater_in_solve() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 10.0],
        low_elevations_m: vec![4.0, 4.0],
        high_elevations_m: vec![6.0, 6.0],
    };
    let coupling = BridgeCouplingParams {
        low_flow_method: 1,
        ..Default::default()
    };
    let legacy_sections = BridgeSectionContext {
        pier_stations: Some(vec![5.0]),
        ..Default::default()
    };
    let tapered_sections = BridgeSectionContext {
        pier_stations: Some(vec![5.0]),
        pier_widths: Some(PierWidthUserInput {
            top_widths: Some(vec![1.0]),
            bottom_widths: Some(vec![2.0]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let solve = |sections: &BridgeSectionContext| {
        solve_bridge_wsel(
            15.0,
            4.0,
            6.0,
            1.5,
            1,
            0,
            1.44,
            0.5,
            0.0,
            0.0,
            2.5,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            Some(&deck),
            Some(sections),
        )
    };
    let hw_legacy = solve(&legacy_sections);
    let hw_tapered = solve(&tapered_sections);
    assert!(
        hw_tapered > hw_legacy + 1e-4,
        "tapered HW {hw_tapered} vs legacy mean-width HW {hw_legacy}"
    );
}

#[test]
fn bridge_flow_regime_label_covers_high_and_low_branches() {
    use crate::solvers::bridge::coupling::bridge_flow_regime_label;

    let table = rectangular_table(10.0, 0.0, 50);
    let coupling_energy = BridgeCouplingParams {
        high_flow_method: 1,
        ..Default::default()
    };
    assert_eq!(
        bridge_flow_regime_label(
            6.0, 6.5, 5.0, 7.0, UnitSystem::Metric, 20.0, &table, &table, &coupling_energy,
            50.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0,
        ),
        "energy"
    );

    let coupling_weir = BridgeCouplingParams::default();
    assert_eq!(
        bridge_flow_regime_label(
            6.0, 7.5, 5.0, 7.0, UnitSystem::Metric, 20.0, &table, &table, &coupling_weir,
            50.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0,
        ),
        "weir"
    );
    assert_eq!(
        bridge_flow_regime_label(
            6.0, 6.2, 5.0, 7.0, UnitSystem::Metric, 20.0, &table, &table, &coupling_weir,
            50.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0,
        ),
        "pressure"
    );

    let coupling_low = BridgeCouplingParams {
        low_flow_method: 1,
        ..Default::default()
    };
    let low_regime = bridge_flow_regime_label(
        2.5, 3.0, 5.0, 7.0, UnitSystem::Metric, 15.0, &table, &table, &coupling_low,
        50.0, 0.5, 1, 0, 1.44, 0.8, 0.0, 0.0,
    );
    assert!(
        matches!(low_regime.as_str(), "low_a" | "low_b" | "low_c"),
        "unexpected low-flow regime {low_regime}"
    );
}

#[test]
fn solve_bridge_coupled_us_customary_and_reverse_q() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        ..Default::default()
    };
    let forward = solve_bridge_coupled(
        100.0,
        16.0,
        20.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        18.0,
        UnitSystem::USCustomary,
        &table_up,
        &table_down,
        &coupling,
        150.0,
        None,
        None,
    );
    assert!(forward.wsel_up > forward.wsel_down);
    assert!(forward.head_loss >= 0.0);

    let reverse = solve_bridge_coupled(
        -100.0,
        16.0,
        20.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        18.0,
        UnitSystem::USCustomary,
        &table_up,
        &table_down,
        &coupling,
        150.0,
        None,
        None,
    );
    assert!(reverse.wsel_down > reverse.wsel_up || reverse.head_loss >= 0.0);
}

#[test]
fn solve_bridge_tailwater_reverse_q_low_flow() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 1,
        ..Default::default()
    };
    let tw = solve_bridge_tailwater(
        -15.0,
        4.0,
        6.0,
        0.5,
        1,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        3.2,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        None,
    );
    assert!(tw.is_finite());
}

#[test]
fn bridge_flow_regime_label_each_low_class_and_us_customary() {
    use crate::solvers::bridge::coupling::bridge_flow_regime_label;
    use crate::solvers::bridge::low_flow::classify_low_flow;
    use crate::solvers::bridge::types::LowFlowClass;

    let coupling = BridgeCouplingParams::default();
    let table = rectangular_table(10.0, 0.0, 50);

    assert_eq!(
        bridge_flow_regime_label(
            3.0, 3.5, 5.0, 7.0, UnitSystem::Metric, 15.0, &table, &table, &coupling,
            50.0, 0.5, 2, 0, 1.44, 0.8, 0.0, 0.0,
        ),
        "low_a"
    );

    let (q_b, tw_b, pier_w, num_piers, table_up, table_down) = class_b_energy_case();
    let geom_b = build_bridge_geometry(
        5.0, 7.0, pier_w, num_piers, 0, 1.44, 0.5, 0.0, 0.0,
        UnitSystem::Metric, &coupling, 15.0, None, None,
    );
    assert_eq!(classify_low_flow(q_b, tw_b, &geom_b, &table_up, &table_down), LowFlowClass::B);
    assert_eq!(
        bridge_flow_regime_label(
            tw_b, tw_b + 0.5, 5.0, 7.0, UnitSystem::Metric, q_b, &table_up, &table_down, &coupling,
            15.0, pier_w, num_piers, 0, 1.44, 0.5, 0.0, 0.0,
        ),
        "low_b"
    );

    let table_narrow = rectangular_table(4.0, 0.0, 50);
    let mut found_c = false;
    'search: for q in [40.0, 60.0, 80.0, 100.0] {
        for tw in [0.5, 0.75, 1.0, 1.25] {
            let geom_c = build_bridge_geometry(
                5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0,
                UnitSystem::Metric, &coupling, 50.0, None, None,
            );
            if classify_low_flow(q, tw, &geom_c, &table_narrow, &table_narrow) == LowFlowClass::C {
                assert_eq!(
                    bridge_flow_regime_label(
                        tw, tw + 0.2, 5.0, 7.0, UnitSystem::Metric, q, &table_narrow, &table_narrow,
                        &coupling, 50.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0,
                    ),
                    "low_c"
                );
                found_c = true;
                break 'search;
            }
        }
    }
    assert!(found_c, "expected a Class C low-flow case for regime label coverage");

    let us_label = bridge_flow_regime_label(
        8.0, 10.0, 16.0, 20.0, UnitSystem::USCustomary, 100.0, &table, &table, &coupling,
        150.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0,
    );
    assert!(matches!(us_label.as_str(), "low_a" | "low_b" | "low_c"));
}

#[test]
fn solve_bridge_tailwater_us_customary_high_flow_branch() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams::default();
    let tw = solve_bridge_tailwater(
        200.0,
        16.0,
        20.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        18.0,
        UnitSystem::USCustomary,
        &table_up,
        &table_down,
        &coupling,
        150.0,
        None,
        None,
    );
    assert!(tw.is_finite() && tw > 0.0);
}
