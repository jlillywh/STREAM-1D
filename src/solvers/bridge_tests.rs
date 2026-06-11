//! Unit tests for bridge hydraulics (see `bridge.rs`).

use super::*;
use crate::geometry::{CrossSection, GuideBankToe, GuideBanks, IneffectiveFlowAreas, row_at_elevation};
use crate::solvers::deck_vent_geometry::DeckVentUserInput;
use crate::solvers::pier_geometry::{
    resolve_pier_width_specs, PierAttachmentsUserInput, PierWidthSpec, PierWidthUserInput,
    ResolvedPier,
};

fn compound_overbank_approach(ineffective: bool) -> CrossSection {
    CrossSection {
        station: 60.0,
        x: vec![0.0, 0.0, 10.0, 10.0, 30.0, 30.0, 40.0, 40.0],
        y: vec![5.0, 0.0, 0.0, 5.0, 0.0, 5.0, 0.0, 5.0],
        n_stations: vec![0.0, 10.0],
        n_values: vec![0.03, 0.05],
        unit_system: UnitSystem::Metric,
        is_overbank: Some(vec![
            false, false, false, false, true, true, true, true,
        ]),
        blocked_obstructions: None,
        ineffective_flow_areas: if ineffective {
            Some(
                IneffectiveFlowAreas::from_block_pairs(&[30.0], &[3.0], &[], &[]).unwrap(),
            )
        } else {
            // Inactive block keeps the reach-cut energy path without clipping conveyance.
            Some(
                IneffectiveFlowAreas::from_block_pairs(&[30.0], &[-100.0], &[], &[]).unwrap(),
            )
        },
        guide_banks: None,
    }
}

fn approach_sections_with_ineffective_overbank() -> BridgeSectionContext {
    BridgeSectionContext {
        friction_length_m: 50.0,
        xs_approach: Some(compound_overbank_approach(true)),
        ..Default::default()
    }
}

fn approach_sections_with_guide_banks(
    approach_width: f64,
    left_toe: f64,
    right_toe: f64,
) -> BridgeSectionContext {
    let approach = CrossSection {
        station: 60.0,
        x: vec![0.0, 0.0, approach_width, approach_width],
        y: vec![10.0, 0.0, 0.0, 10.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    BridgeSectionContext {
        friction_length_m: 50.0,
        xs_approach: Some(approach),
        guide_banks_approach: Some(GuideBanks {
            left_toe: Some(GuideBankToe {
                station: left_toe,
                elevation: 0.0,
            }),
            right_toe: Some(GuideBankToe {
                station: right_toe,
                elevation: 0.0,
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
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
fn test_yarnell_pier_head_loss_hec_ras() {
    let hl = yarnell_pier_head_loss(15.0, 3.0, 0.0, 0.5, 2, PierShape::Square, 30.0);
    assert!(
        (hl - 0.00247).abs() < 1e-4,
        "Yarnell head loss should match HEC-RAS formula, got {hl}"
    );
}

#[test]
fn test_yarnell_zero_piers_no_loss() {
    let hl = yarnell_pier_head_loss(15.0, 3.0, 0.0, 0.5, 0, PierShape::Square, 30.0);
    assert_eq!(hl, 0.0);
}

#[test]
fn pier_shape_coefficients_match_hecras_table() {
    const CASES: &[(i32, f64, f64)] = &[
        (0, 1.25, 2.00),
        (1, 0.90, 1.20),
        (2, 0.95, 1.33),
        (3, 1.05, 1.60),
        (4, 1.05, 1.33),
        (5, 2.50, 2.00),
        (6, 0.90, 0.60),
        (7, 0.90, 0.32),
        (8, 0.90, 0.29),
        (9, 1.05, 1.00),
        (10, 1.05, 1.39),
        (11, 1.05, 1.72),
    ];
    for &(code, k, cd) in CASES {
        let shape = PierShape::from_i32(code);
        assert!(
            (shape.yarnell_coefficient() - k).abs() < 1e-9,
            "K mismatch for code {code}"
        );
        assert!(
            (shape.drag_coefficient() - cd).abs() < 1e-9,
            "C_D mismatch for code {code}"
        );
    }
}

fn momentum_pier_geometry(shape: PierShape) -> BridgeGeometry {
    BridgeGeometry {
        low_chord_m: 5.0,
        low_chord_max_m: 5.0,
        high_chord_m: 7.0,
        high_chord_max_m: 7.0,
        pier_width_m: 0.5,
        num_piers: 2,
        pier_shape: shape,
        abutments: BridgeAbutments::default(),
        weir_coeff_m: 1.44,
        orifice_coeff: 0.5,
        z_up_m: 0.0,
        z_down_m: 0.0,
        low_flow_method: LowFlowMethod::Momentum,
        high_flow_method: HighFlowMethod::PressureWeir,
        length_m: 50.0,
        wspro_coeff_c: 0.8,
        coeff_contraction: 0.1,
        coeff_expansion: 0.3,
        pressure_coeff_inlet: 0.0,
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: 0.98,
        deck: None,
        pier_stations_m: vec![],
        pier_specs: vec![],
        skew_deg: 0.0,
        skew_cos: 1.0,
        ineffective_up: None,
        ineffective_down: None,
        xs_up: None,
        xs_down: None,
        xs_approach: None,
        xs_departure: None,
        guide_banks_approach: None,
        guide_banks_departure: None,
        table_approach: None,
        table_departure: None,
        deck_vents: Vec::new(),
    }
}

fn momentum_drag_for_shape(shape: PierShape) -> f64 {
    let table = rectangular_table(10.0, 0.0, 50);
    let geom = momentum_pier_geometry(shape);
    pier_drag_momentum_with_table(&table, 15.0, 3.0, geom.z_up_m, &geom, true)
}

/// API v29 pier shapes `4`–`11`: one behavioral case each (see `extended_pier_shape_catalog.md`).

#[test]
fn test_pier_shape_4_twin_no_diaphragm_lower_momentum_drag_than_triangular_90() {
    let twin_open = momentum_drag_for_shape(PierShape::TwinCylinderNoDiaphragm);
    let triangular = momentum_drag_for_shape(PierShape::Triangular);
    let yarnell_twin = yarnell_pier_head_loss(15.0, 3.0, 0.0, 0.5, 2, PierShape::TwinCylinderNoDiaphragm, 30.0);
    let yarnell_tri = yarnell_pier_head_loss(15.0, 3.0, 0.0, 0.5, 2, PierShape::Triangular, 30.0);
    assert!((yarnell_twin - yarnell_tri).abs() < 1e-9, "same Yarnell K=1.05");
    assert!(
        twin_open < triangular - 1e-6,
        "C_D 1.33 vs 1.60: twin_open={twin_open}, triangular={triangular}"
    );
}

#[test]
fn test_pier_shape_5_trestle_yarnell_exceeds_square() {
    let square = yarnell_pier_head_loss(15.0, 3.0, 0.0, 0.5, 2, PierShape::Square, 30.0);
    let trestle = yarnell_pier_head_loss(15.0, 3.0, 0.0, 0.5, 2, PierShape::TenPileTrestle, 30.0);
    assert!(
        trestle > square + 1e-4,
        "K=2.50 vs 1.25: trestle={trestle}, square={square}"
    );
}

#[test]
fn test_pier_shape_6_elliptical_2to1_lower_momentum_drag_than_semicircular() {
    let elliptical = momentum_drag_for_shape(PierShape::Elliptical2to1);
    let circular = momentum_drag_for_shape(PierShape::Semicircular);
    assert!(
        elliptical < circular - 1e-6,
        "C_D 0.60 vs 1.20: elliptical={elliptical}, circular={circular}"
    );
}

#[test]
fn test_pier_shape_7_elliptical_4to1_lower_drag_than_2to1() {
    let e4 = momentum_drag_for_shape(PierShape::Elliptical4to1);
    let e2 = momentum_drag_for_shape(PierShape::Elliptical2to1);
    assert!(e4 < e2 - 1e-6, "C_D 0.32 vs 0.60: e4={e4}, e2={e2}");
}

#[test]
fn test_pier_shape_8_elliptical_8to1_lowest_elliptical_drag() {
    let e8 = momentum_drag_for_shape(PierShape::Elliptical8to1);
    let e4 = momentum_drag_for_shape(PierShape::Elliptical4to1);
    assert!(e8 < e4 - 1e-6, "C_D 0.29 vs 0.32: e8={e8}, e4={e4}");
}

#[test]
fn test_pier_shape_9_triangular_30_lower_drag_than_90() {
    let t30 = momentum_drag_for_shape(PierShape::Triangular30);
    let t90 = momentum_drag_for_shape(PierShape::Triangular);
    assert!(t30 < t90 - 1e-6, "C_D 1.00 vs 1.60: t30={t30}, t90={t90}");
}

#[test]
fn test_pier_shape_10_triangular_60_between_30_and_90() {
    let t30 = momentum_drag_for_shape(PierShape::Triangular30);
    let t60 = momentum_drag_for_shape(PierShape::Triangular60);
    let t90 = momentum_drag_for_shape(PierShape::Triangular);
    assert!(t60 > t30 + 1e-6 && t60 < t90 - 1e-6, "C_D 1.39 between 1.00 and 1.60");
}

#[test]
fn test_pier_shape_11_triangular_120_highest_triangular_drag() {
    let t120 = momentum_drag_for_shape(PierShape::Triangular120);
    let t90 = momentum_drag_for_shape(PierShape::Triangular);
    assert!(t120 > t90 + 1e-6, "C_D 1.72 vs 1.60: t120={t120}, t90={t90}");
}

#[test]
fn test_yarnell_square_pier_loss_exceeds_semicircular() {
    let square = yarnell_pier_head_loss(15.0, 3.0, 0.0, 0.5, 2, PierShape::Square, 30.0);
    let semi = yarnell_pier_head_loss(15.0, 3.0, 0.0, 0.5, 2, PierShape::Semicircular, 30.0);
    assert!(square > semi);
}

#[test]
fn test_classify_low_flow_subcritical_is_class_a() {
    let table = rectangular_table(10.0, 0.0, 50);
    let geom = BridgeGeometry {
        low_chord_m: 5.0,
        low_chord_max_m: 5.0,
        high_chord_m: 7.0,
        high_chord_max_m: 7.0,
        pier_width_m: 0.5,
        num_piers: 2,
        pier_shape: PierShape::Square,
        abutments: BridgeAbutments::default(),
        weir_coeff_m: 1.44,
        orifice_coeff: 0.5,
        z_up_m: 0.1,
        z_down_m: 0.0,
        low_flow_method: LowFlowMethod::Auto,
        high_flow_method: HighFlowMethod::PressureWeir,
        length_m: 100.0,
        wspro_coeff_c: 0.8,
        coeff_contraction: 0.1,
        coeff_expansion: 0.3,
        pressure_coeff_inlet: 0.0,
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: 0.98,
        deck: None,
        pier_stations_m: vec![],
        pier_specs: vec![],
        skew_deg: 0.0,
        skew_cos: 1.0,
        ineffective_up: None,
        ineffective_down: None,
        xs_up: None,
        xs_down: None,
        xs_approach: None,
        xs_departure: None,
        guide_banks_approach: None,
        guide_banks_departure: None,
        table_approach: None,
        table_departure: None,
        deck_vents: Vec::new(),
    };
    assert_eq!(
        classify_low_flow(15.0, 3.0, &geom, &table, &table),
        LowFlowClass::A
    );
}

#[test]
fn test_asymmetric_abutments_reduce_area_more_on_wide_side() {
    let table = rectangular_table(10.0, 0.0, 50);
    let base = BridgeGeometry {
        low_chord_m: 5.0,
        low_chord_max_m: 5.0,
        high_chord_m: 7.0,
        high_chord_max_m: 7.0,
        pier_width_m: 0.0,
        num_piers: 0,
        pier_shape: PierShape::Square,
        abutments: BridgeAbutments::default(),
        weir_coeff_m: 1.44,
        orifice_coeff: 0.5,
        z_up_m: 0.0,
        z_down_m: 0.0,
        low_flow_method: LowFlowMethod::Momentum,
        high_flow_method: HighFlowMethod::PressureWeir,
        length_m: 100.0,
        wspro_coeff_c: 0.8,
        coeff_contraction: 0.1,
        coeff_expansion: 0.3,
        pressure_coeff_inlet: 0.0,
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: 0.98,
        deck: None,
        pier_stations_m: vec![],
        pier_specs: vec![],
        skew_deg: 0.0,
        skew_cos: 1.0,
        ineffective_up: None,
        ineffective_down: None,
        xs_up: None,
        xs_down: None,
        xs_approach: None,
        xs_departure: None,
        guide_banks_approach: None,
        guide_banks_departure: None,
        table_approach: None,
        table_departure: None,
        deck_vents: Vec::new(),
    };
    let narrow_left = BridgeGeometry {
        abutments: resolve_abutments(
            &BridgeAbutmentUserInput {
                left_width: Some(1.0),
                right_width: Some(3.0),
                ..Default::default()
            },
            0.0,
            10.0,
            1.0,
            UnitSystem::Metric,
        ),
        ..base.clone()
    };
    let symmetric = BridgeGeometry {
        abutments: BridgeAbutments::symmetric_total_width_m(4.0, 0.0, 10.0),
        ..base
    };
    let props_asym = obstructed_hydraulics(&table, 3.0, 0.0, &narrow_left, false);
    let props_sym = obstructed_hydraulics(&table, 3.0, 0.0, &symmetric, false);
    assert!(
        (props_asym.a_eff - props_sym.a_eff).abs() < 1e-6,
        "same total width should yield same effective area"
    );
    assert!((narrow_left.abutments.left_width_m() - 1.0).abs() < 1e-9);
    assert!((narrow_left.abutments.right_width_m() - 3.0).abs() < 1e-9);
}

#[test]
fn test_per_side_abutment_tops_affect_obstructed_hydraulics() {
    let table = rectangular_table(10.0, 0.0, 50);
    let base = BridgeGeometry {
        low_chord_m: 5.0,
        low_chord_max_m: 5.0,
        high_chord_m: 7.0,
        high_chord_max_m: 7.0,
        pier_width_m: 0.0,
        num_piers: 0,
        pier_shape: PierShape::Square,
        abutments: BridgeAbutments::default(),
        weir_coeff_m: 1.44,
        orifice_coeff: 0.5,
        z_up_m: 0.0,
        z_down_m: 0.0,
        low_flow_method: LowFlowMethod::Momentum,
        high_flow_method: HighFlowMethod::PressureWeir,
        length_m: 100.0,
        wspro_coeff_c: 0.8,
        coeff_contraction: 0.1,
        coeff_expansion: 0.3,
        pressure_coeff_inlet: 0.0,
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: 0.98,
        deck: None,
        pier_stations_m: vec![],
        pier_specs: vec![],
        skew_deg: 0.0,
        skew_cos: 1.0,
        ineffective_up: None,
        ineffective_down: None,
        xs_up: None,
        xs_down: None,
        xs_approach: None,
        xs_departure: None,
        guide_banks_approach: None,
        guide_banks_departure: None,
        table_approach: None,
        table_departure: None,
        deck_vents: Vec::new(),
    };
    let geom_both = BridgeGeometry {
        abutments: resolve_abutments(
            &BridgeAbutmentUserInput {
                left_width: Some(2.0),
                right_width: Some(2.0),
                left_top_elevation: Some(0.0),
                right_top_elevation: Some(0.0),
                ..Default::default()
            },
            0.0,
            10.0,
            1.0,
            UnitSystem::Metric,
        ),
        ..base.clone()
    };
    let geom_right_only = BridgeGeometry {
        abutments: resolve_abutments(
            &BridgeAbutmentUserInput {
                left_width: Some(2.0),
                right_width: Some(2.0),
                left_top_elevation: Some(3.5),
                right_top_elevation: Some(0.0),
                ..Default::default()
            },
            0.0,
            10.0,
            1.0,
            UnitSystem::Metric,
        ),
        ..base
    };
    let props_both = obstructed_hydraulics(&table, 3.0, 0.0, &geom_both, false);
    let props_right = obstructed_hydraulics(&table, 3.0, 0.0, &geom_right_only, false);
    assert!(props_right.a_eff > props_both.a_eff);
    assert!(props_right.top_width > props_both.top_width);
}

#[test]
fn test_abutment_reduces_opening_area() {
    let table = rectangular_table(10.0, 0.0, 50);
    let geom_no_abut = BridgeGeometry {
        low_chord_m: 5.0,
        low_chord_max_m: 5.0,
        high_chord_m: 7.0,
        high_chord_max_m: 7.0,
        pier_width_m: 0.0,
        num_piers: 0,
        pier_shape: PierShape::Square,
        abutments: BridgeAbutments::default(),
        weir_coeff_m: 1.44,
        orifice_coeff: 0.5,
        z_up_m: 0.0,
        z_down_m: 0.0,
        low_flow_method: LowFlowMethod::Momentum,
        high_flow_method: HighFlowMethod::PressureWeir,
        length_m: 100.0,
        wspro_coeff_c: 0.8,
        coeff_contraction: 0.1,
        coeff_expansion: 0.3,
        pressure_coeff_inlet: 0.0,
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: 0.98,
        deck: None,
        pier_stations_m: vec![],
        pier_specs: vec![],
        skew_deg: 0.0,
        skew_cos: 1.0,
        ineffective_up: None,
        ineffective_down: None,
        xs_up: None,
        xs_down: None,
        xs_approach: None,
        xs_departure: None,
        guide_banks_approach: None,
        guide_banks_departure: None,
        table_approach: None,
        table_departure: None,
        deck_vents: Vec::new(),
    };
    let props_no = obstructed_hydraulics(&table, 3.0, 0.0, &geom_no_abut, false);
    let geom_abut = BridgeGeometry {
        abutments: BridgeAbutments::symmetric_total_width_m(2.0, 0.0, 10.0),
        ..geom_no_abut.clone()
    };
    let props_abut = obstructed_hydraulics(&table, 3.0, 0.0, &geom_abut, false);
    assert!(props_abut.a_eff < props_no.a_eff);
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
fn test_wspro_higher_c_lowers_head_loss() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let base = BridgeCouplingParams {
        low_flow_method: 4,
        abutment: BridgeAbutmentUserInput {
            legacy_total_width: 2.0,
            ..Default::default()
        },
        length: 50.0,
        ..Default::default()
    };
    let mut coupling_low_c = base.clone();
    coupling_low_c.wspro_coeff = 0.6;
    let mut coupling_high_c = base;
    coupling_high_c.wspro_coeff = 0.95;
    let hw_low = solve_bridge_wsel(
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
        &table_up,
        &table_down,
        &coupling_low_c,
        50.0,
        None,
        None,
    );
    let hw_high = solve_bridge_wsel(
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
        &table_up,
        &table_down,
        &coupling_high_c,
        50.0,
        None,
        None,
    );
    assert!(
        hw_high < hw_low,
        "higher WSPRO C should reduce upstream head, low_c={hw_low}, high_c={hw_high}"
    );
}

#[test]
fn test_auto_low_flow_uses_wspro_with_abutments() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let auto_coupling = BridgeCouplingParams {
        low_flow_method: 0,
        abutment: BridgeAbutmentUserInput {
            legacy_total_width: 1.5,
            ..Default::default()
        },
        length: 50.0,
        ..Default::default()
    };
    let wspro_coupling = BridgeCouplingParams {
        low_flow_method: 4,
        abutment: BridgeAbutmentUserInput {
            legacy_total_width: 1.5,
            ..Default::default()
        },
        length: 50.0,
        ..Default::default()
    };
    let hw_auto = solve_bridge_wsel(
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
        &table_up,
        &table_down,
        &auto_coupling,
        50.0,
        None,
        None,
    );
    let hw_wspro = solve_bridge_wsel(
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
        &table_up,
        &table_down,
        &wspro_coupling,
        50.0,
        None,
        None,
    );
    assert!(
        (hw_auto - hw_wspro).abs() < 0.01,
        "auto with abutments should match explicit WSPRO, auto={hw_auto}, wspro={hw_wspro}"
    );
}

#[test]
fn test_sluice_gate_cd_increases_with_submergence() {
    let cd_min = sluice_gate_discharge_coeff(0.0, 0.0);
    let cd_mid = sluice_gate_discharge_coeff(0.5, 0.0);
    let cd_deep = sluice_gate_discharge_coeff(1.0, 0.0);
    assert!(cd_deep > cd_mid);
    assert!(cd_mid > cd_min);
    assert!((cd_min - 0.27).abs() < 0.01);
    assert!((cd_deep - 0.5).abs() < 0.05);
}

#[test]
fn test_bradley_submergence_reduces_weir_factor() {
    assert!((bradley_weir_submergence_factor(0.0) - 1.0).abs() < 1e-6);
    assert!(bradley_weir_submergence_factor(0.9) < bradley_weir_submergence_factor(0.5));
    assert!(bradley_weir_submergence_factor(0.95) < 0.3);
}

#[test]
fn test_bu_bd_min_opening_weighting_for_pressure_flow() {
    let table_bu_wide = rectangular_table(10.0, 0.0, 50);
    let table_bd_narrow = rectangular_table(4.0, 0.0, 50);
    let table_both_wide = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams::default();
    let q = 20.0;
    let tw = 5.2;
    let hw_both_wide = solve_bridge_wsel(
        q,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        tw,
        UnitSystem::Metric,
        &table_both_wide,
        &table_both_wide,
        &coupling,
        50.0,
        None,
        None,
    );
    let bu_xs = CrossSection {
        station: 0.0,
        x: vec![0.0, 0.0, 10.0, 10.0],
        y: vec![10.0, 0.0, 0.0, 10.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    let bd_xs = CrossSection {
        station: 0.0,
        x: vec![0.0, 0.0, 4.0, 4.0],
        y: vec![10.0, 0.0, 0.0, 10.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    let sections = BridgeSectionContext {
        xs_up: Some(bu_xs),
        xs_down: Some(bd_xs),
        ..Default::default()
    };
    let hw_bd_constricts = solve_bridge_wsel(
        q,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        tw,
        UnitSystem::Metric,
        &table_bu_wide,
        &table_bd_narrow,
        &coupling,
        50.0,
        None,
        Some(&sections),
    );
    assert!(
        hw_bd_constricts > hw_both_wide,
        "BD constriction should raise submerged pressure headwater: wide={hw_both_wide}, constricted={hw_bd_constricts}"
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
fn test_submerged_orifice_constant_driving_head() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams::default();
    let q = 35.0;
    let hw_mild = solve_bridge_wsel(
        q,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        5.05,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        None,
    );
    let hw_deep = solve_bridge_wsel(
        q,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        5.8,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        None,
    );
    // Fully submerged orifice: E_up - TW â‰ˆ constant for a given Q, so WSEL rises with tailwater.
    assert!(
        hw_deep > hw_mild,
        "deeper submergence should raise upstream WSEL, mild={hw_mild}, deep={hw_deep}"
    );
    let drive_mild = hw_mild - 5.05;
    let drive_deep = hw_deep - 5.8;
    assert!(
        (drive_mild - drive_deep).abs() < 0.05,
        "driving head should be similar, mild={drive_mild}, deep={drive_deep}"
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

fn abutment_coupling(left_w: f64, right_w: f64, left_top: f64, right_top: f64) -> BridgeCouplingParams {
    BridgeCouplingParams {
        abutment: BridgeAbutmentUserInput {
            left_width: Some(left_w),
            right_width: Some(right_w),
            left_top_elevation: Some(left_top),
            right_top_elevation: Some(right_top),
            ..Default::default()
        },
        length: 50.0,
        ..Default::default()
    }
}

#[test]
fn test_per_side_abutments_affect_energy_wspro_momentum_pressure() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let symmetric = abutment_coupling(2.5, 2.5, 0.0, 0.0);
    let asymmetric = abutment_coupling(1.0, 4.0, 0.0, 2.5);
    let q = 15.0;
    let tw = 2.5;

    let mut energy_sym = symmetric.clone();
    energy_sym.low_flow_method = 3;
    let mut energy_asym = asymmetric.clone();
    energy_asym.low_flow_method = 3;
    let hw_energy_sym = solve_bridge_wsel(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw,
        UnitSystem::Metric, &table_up, &table_down, &energy_sym, 50.0, None, None,
    );
    let hw_energy_asym = solve_bridge_wsel(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw,
        UnitSystem::Metric, &table_up, &table_down, &energy_asym, 50.0, None, None,
    );
    assert!(
        (hw_energy_sym - hw_energy_asym).abs() > 0.01,
        "energy method should reflect per-side abutment tops"
    );

    let mut wspro_sym = symmetric.clone();
    wspro_sym.low_flow_method = 4;
    let mut wspro_asym = asymmetric.clone();
    wspro_asym.low_flow_method = 4;
    let hw_wspro_sym = solve_bridge_wsel(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw,
        UnitSystem::Metric, &table_up, &table_down, &wspro_sym, 50.0, None, None,
    );
    let hw_wspro_asym = solve_bridge_wsel(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw,
        UnitSystem::Metric, &table_up, &table_down, &wspro_asym, 50.0, None, None,
    );
    assert!(
        (hw_wspro_sym - hw_wspro_asym).abs() > 0.01,
        "WSPRO should reflect per-side abutment tops"
    );

    let mut momentum = asymmetric.clone();
    momentum.low_flow_method = 2;
    let hw_momentum = solve_bridge_wsel(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw,
        UnitSystem::Metric, &table_up, &table_down, &momentum, 50.0, None, None,
    );
    assert!(hw_momentum > tw);

    let mut pressure = asymmetric.clone();
    pressure.low_flow_method = 3;
    let hw_pressure = solve_bridge_wsel(
        35.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, 5.8,
        UnitSystem::Metric, &table_up, &table_down, &pressure, 50.0, None, None,
    );
    let mut pressure_sym = symmetric.clone();
    pressure_sym.low_flow_method = 3;
    let hw_pressure_sym = solve_bridge_wsel(
        35.0, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, 5.8,
        UnitSystem::Metric, &table_up, &table_down, &pressure_sym, 50.0, None, None,
    );
    assert!(
        (hw_pressure - hw_pressure_sym).abs() > 0.01,
        "pressure flow should reflect per-side abutment obstruction"
    );

    let mut yarnell_sym = symmetric.clone();
    yarnell_sym.low_flow_method = 1;
    let mut yarnell_asym = asymmetric.clone();
    yarnell_asym.low_flow_method = 1;
    let hw_yarnell_sym = solve_bridge_wsel(
        q, 5.0, 7.0, 0.5, 2, 0, 1.44, 0.5, 0.0, 0.0, tw,
        UnitSystem::Metric, &table_up, &table_down, &yarnell_sym, 50.0, None, None,
    );
    let hw_yarnell_asym = solve_bridge_wsel(
        q, 5.0, 7.0, 0.5, 2, 0, 1.44, 0.5, 0.0, 0.0, tw,
        UnitSystem::Metric, &table_up, &table_down, &yarnell_asym, 50.0, None, None,
    );
    assert!(
        (hw_yarnell_sym - hw_yarnell_asym).abs() > 0.001,
        "Yarnell should use per-side abutment area in pier alpha"
    );

    let q_weir = 50.0;
    let tw_weir = 5.5;
    let weir_sym = solve_bridge_coupled(
        q_weir, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, tw_weir,
        UnitSystem::Metric, &table_up, &table_down, &symmetric, 50.0, None, None,
    );
    let weir_asym = solve_bridge_coupled(
        q_weir, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, tw_weir,
        UnitSystem::Metric, &table_up, &table_down, &asymmetric, 50.0, None, None,
    );
    assert!(
        weir_sym.flow_regime == "weir" || weir_sym.flow_regime == "pressure",
        "expected high-flow regime, got {}",
        weir_sym.flow_regime
    );
    assert!(
        (weir_sym.wsel_up - weir_asym.wsel_up).abs() > 0.01,
        "weir/pressure EGL should reflect per-side abutment obstruction"
    );
}

#[test]
fn test_bridge_rating_curve() {
    let inputs = BridgeRatingCurveInputs {
        q_values: vec![10.0, 20.0, 30.0],
        bridge: BridgeSolveParams {
            low_chord: 5.0,
            high_chord: 7.0,
            z_down: 0.0,
            z_up: 0.0,
            tw_wsel: 2.5,
            low_flow_method: 3,
            channel_width: 10.0,
            manning_n: 0.03,
            ..Default::default()
        },
    };
    let curve = compute_bridge_rating_curve(&inputs);
    assert_eq!(curve.q.len(), 3);
    assert!(curve.wsel[1] > curve.wsel[0]);
    assert!(curve.wsel[2] > curve.wsel[1]);
    assert_eq!(curve.wsel_down.len(), 3);
    assert_eq!(curve.flow_regimes.len(), 3);
    assert!(!curve.flow_regimes[0].is_empty());
}

fn hand_rectangular_a_eff(
    channel_width_m: f64,
    wsel_m: f64,
    z_bed_m: f64,
    geom: &BridgeGeometry,
) -> f64 {
    let a_base = channel_width_m * (wsel_m - z_bed_m).max(0.0);
    (a_base - geom.abutments.submerged_area_m2(wsel_m, z_bed_m)).max(1e-5)
}

#[test]
fn test_obstructed_area_hand_calc_asymmetric_and_one_sided() {
    let table = rectangular_table(10.0, 0.0, 50);
    let base = BridgeGeometry {
        low_chord_m: 5.0,
        low_chord_max_m: 5.0,
        high_chord_m: 7.0,
        high_chord_max_m: 7.0,
        pier_width_m: 0.0,
        num_piers: 0,
        pier_shape: PierShape::Square,
        abutments: BridgeAbutments::default(),
        weir_coeff_m: 1.44,
        orifice_coeff: 0.5,
        z_up_m: 0.0,
        z_down_m: 0.0,
        low_flow_method: LowFlowMethod::Wspro,
        high_flow_method: HighFlowMethod::PressureWeir,
        length_m: 50.0,
        wspro_coeff_c: 0.8,
        coeff_contraction: 0.1,
        coeff_expansion: 0.3,
        pressure_coeff_inlet: 0.0,
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: 0.98,
        deck: None,
        pier_stations_m: vec![],
        pier_specs: vec![],
        skew_deg: 0.0,
        skew_cos: 1.0,
        ineffective_up: None,
        ineffective_down: None,
        xs_up: None,
        xs_down: None,
        xs_approach: None,
        xs_departure: None,
        guide_banks_approach: None,
        guide_banks_departure: None,
        table_approach: None,
        table_departure: None,
        deck_vents: Vec::new(),
    };

    let asymmetric = BridgeGeometry {
        abutments: resolve_abutments(
            &BridgeAbutmentUserInput {
                left_width: Some(1.0),
                right_width: Some(4.0),
                left_top_elevation: Some(0.0),
                right_top_elevation: Some(2.5),
                ..Default::default()
            },
            0.0,
            10.0,
            1.0,
            UnitSystem::Metric,
        ),
        ..base.clone()
    };
    let props_asym = obstructed_hydraulics(&table, 2.5, 0.0, &asymmetric, false);
    let hand_asym = hand_rectangular_a_eff(10.0, 2.5, 0.0, &asymmetric);
    assert!((hand_asym - 22.5).abs() < 1e-6, "hand A_eff@2.5 = {hand_asym}");
    assert!(
        (props_asym.a_eff - hand_asym).abs() < 1e-3,
        "obstructed_hydraulics {:.4} vs hand {:.4}",
        props_asym.a_eff,
        hand_asym
    );

    let left_only = BridgeGeometry {
        abutments: resolve_abutments(
            &BridgeAbutmentUserInput {
                left_width: Some(3.0),
                left_top_elevation: Some(0.0),
                ..Default::default()
            },
            0.0,
            10.0,
            1.0,
            UnitSystem::Metric,
        ),
        ..base.clone()
    };
    let props_left = obstructed_hydraulics(&table, 2.5, 0.0, &left_only, false);
    assert!((hand_rectangular_a_eff(10.0, 2.5, 0.0, &left_only) - 17.5).abs() < 1e-6);
    assert!((props_left.a_eff - 17.5).abs() < 1e-3);

    let right_only = BridgeGeometry {
        abutments: resolve_abutments(
            &BridgeAbutmentUserInput {
                right_width: Some(3.0),
                right_top_elevation: Some(2.0),
                ..Default::default()
            },
            0.0,
            10.0,
            1.0,
            UnitSystem::Metric,
        ),
        ..base
    };
    let props_right = obstructed_hydraulics(&table, 2.5, 0.0, &right_only, false);
    assert!((hand_rectangular_a_eff(10.0, 2.5, 0.0, &right_only) - 23.5).abs() < 1e-6);
    assert!((props_right.a_eff - 23.5).abs() < 1e-3);
}

#[test]
fn test_wspro_headwater_hand_calc_reference_cases() {
    let table = rectangular_table(10.0, 0.0, 50);
    let q = 15.0;
    let tw = 2.5;

    let cases: [(&str, BridgeCouplingParams, f64, f64); 3] = [
        (
            "asymmetric_per_side",
            abutment_coupling(1.0, 4.0, 0.0, 2.5),
            22.5,
            2.511_630_058_288_574,
        ),
        (
            "one_sided_left",
            BridgeCouplingParams {
                abutment: BridgeAbutmentUserInput {
                    left_width: Some(3.0),
                    left_top_elevation: Some(0.0),
                    ..Default::default()
                },
                length: 50.0,
                low_flow_method: 4,
                ..Default::default()
            },
            17.5,
            2.519_601_583_480_835,
        ),
        (
            "symmetric_full_height",
            abutment_coupling(2.5, 2.5, 0.0, 0.0),
            12.5,
            2.539_474_964_141_846,
        ),
    ];

    let base = BridgeGeometry {
        low_chord_m: 5.0,
        low_chord_max_m: 5.0,
        high_chord_m: 7.0,
        high_chord_max_m: 7.0,
        pier_width_m: 0.0,
        num_piers: 0,
        pier_shape: PierShape::Square,
        abutments: BridgeAbutments::default(),
        weir_coeff_m: 1.44,
        orifice_coeff: 0.5,
        z_up_m: 0.0,
        z_down_m: 0.0,
        low_flow_method: LowFlowMethod::Wspro,
        high_flow_method: HighFlowMethod::PressureWeir,
        length_m: 50.0,
        wspro_coeff_c: 0.8,
        coeff_contraction: 0.1,
        coeff_expansion: 0.3,
        pressure_coeff_inlet: 0.0,
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: 0.98,
        deck: None,
        pier_stations_m: vec![],
        pier_specs: vec![],
        skew_deg: 0.0,
        skew_cos: 1.0,
        ineffective_up: None,
        ineffective_down: None,
        xs_up: None,
        xs_down: None,
        xs_approach: None,
        xs_departure: None,
        guide_banks_approach: None,
        guide_banks_departure: None,
        table_approach: None,
        table_departure: None,
        deck_vents: Vec::new(),
    };

    for (name, mut coupling, a_eff_tw, expected_hw) in cases {
        coupling.low_flow_method = 4;
        let geom = BridgeGeometry {
            abutments: resolve_abutments(
                &coupling.abutment,
                0.0,
                10.0,
                1.0,
                UnitSystem::Metric,
            ),
            length_m: coupling.length,
            wspro_coeff_c: coupling.wspro_coeff,
            coeff_contraction: coupling.coeff_contraction,
            coeff_expansion: coupling.coeff_expansion,
            ..base.clone()
        };
        assert!(
            (hand_rectangular_a_eff(10.0, tw, 0.0, &geom) - a_eff_tw).abs() < 1e-3,
            "{name}: hand A_eff@TW"
        );

        let hw = solve_bridge_wsel(
            q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw,
            UnitSystem::Metric, &table, &table, &coupling, 50.0, None, None,
        );
        assert!(
            (hw - expected_hw).abs() < 0.002,
            "{name}: WSPRO hw {hw:.4} vs reference {expected_hw:.4}"
        );

        let props_down = obstructed_hydraulics(&table, tw, 0.0, &geom, false);
        let props_up = obstructed_hydraulics(&table, hw, 0.0, &geom, true);
        let opening_wsel = hw.min(tw).min(geom.low_chord_m);
        let props_open = obstructed_hydraulics(&table, opening_wsel, 0.0, &geom, true);
        let e_down = tw + velocity_head(q, props_down.a_eff);
        let e_up = hw + velocity_head(q, props_up.a_eff);
        let hf = friction_loss(
            q,
            obstructed_conveyance(&table, tw, 0.0, &geom, false),
            obstructed_conveyance(&table, hw, 0.0, &geom, true),
            geom.length_m,
        );
        let h_wspro = wspro_contraction_loss(
            q,
            props_up.a_eff,
            props_open.a_eff.max(1e-5),
            geom.wspro_coeff_c,
        );
        assert!(
            (e_up - e_down - hf - h_wspro).abs() < 1e-4,
            "{name}: WSPRO energy balance residual"
        );
    }
}

#[test]
fn test_bridge_rating_curve_per_side_abutments() {
    let base = BridgeSolveParams {
        low_chord: 5.0,
        high_chord: 7.0,
        z_down: 0.0,
        z_up: 0.0,
        tw_wsel: 2.5,
        low_flow_method: 4,
        channel_width: 10.0,
        manning_n: 0.03,
        ..Default::default()
    };
    let asymmetric = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![15.0, 25.0],
        bridge: BridgeSolveParams {
            abutment_left_width: Some(1.0),
            abutment_right_width: Some(4.0),
            abutment_right_top_elevation: Some(2.5),
            ..base.clone()
        },
    });
    let legacy_symmetric = compute_bridge_rating_curve(&BridgeRatingCurveInputs {
        q_values: vec![15.0, 25.0],
        bridge: BridgeSolveParams {
            abutment_block_width: 5.0,
            ..base
        },
    });
    assert!(
        (asymmetric.wsel[0] - legacy_symmetric.wsel[0]).abs() > 0.01,
        "rating curve should honor per-side abutment geometry"
    );
    assert!(
        asymmetric.wsel[1] > asymmetric.wsel[0],
        "headwater should increase with discharge"
    );
}

#[test]
fn test_explicit_high_flow_energy_method() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let pressure_coupling = BridgeCouplingParams::default();
    let energy_coupling = BridgeCouplingParams {
        high_flow_method: 1,
        low_flow_method: 3,
        ..Default::default()
    };
    let q = 35.0;
    let tw = 5.8;

    let pressure = solve_bridge_coupled(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, tw,
        UnitSystem::Metric, &table_up, &table_down, &pressure_coupling, 50.0, None, None,
    );
    let energy = solve_bridge_coupled(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, tw,
        UnitSystem::Metric, &table_up, &table_down, &energy_coupling, 50.0, None, None,
    );

    assert_eq!(pressure.flow_regime, "pressure");
    assert_eq!(energy.flow_regime, "energy");
    assert!(energy.wsel_up > tw);
    assert!(
        (pressure.wsel_up - energy.wsel_up).abs() > 0.01,
        "explicit energy should differ from pressure/weir, pressure={}, energy={}",
        pressure.wsel_up,
        energy.wsel_up
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

/// Approach guide banks narrow the contraction area below the BU reach face; loss uses
/// `coeff_contraction` on velocity-head difference (guided approach â†’ opening), not BU area alone.
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
fn test_high_flow_energy_supercritical_roundtrip() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        high_flow_method: 1,
        low_flow_method: 3,
        ..Default::default()
    };
    let q = 30.0;
    let hw = solve_bridge_wsel(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, 5.5,
        UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, None,
    );
    let tw = solve_bridge_tailwater(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, hw,
        UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, None,
    );
    let hw_back = solve_bridge_wsel(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, tw,
        UnitSystem::Metric, &table_up, &table_down, &coupling, 50.0, None, None,
    );
    assert!((hw_back - hw).abs() < 0.05, "roundtrip hw={hw}, hw_back={hw_back}, tw={tw}");
}

#[test]
fn approach_overbank_ineffective_splits_storage_and_conveyance() {
    let approach = compound_overbank_approach(true);
    let table = approach.generate_lookup_table(50);
    let wsel = 2.5;
    let row = row_at_elevation(&table, &approach, wsel, None, None);
    let plain = approach.compute_properties_at_elevation(wsel);
    assert!(
        (row.area - plain.area).abs() < 1e-2,
        "ineffective should retain ponded storage on approach cut"
    );
    assert!(
        row.active_area < row.area - 1.0,
        "left overbank ineffective should clip conveyance below activation"
    );
    assert!(row.conveyance < plain.conveyance);
}

#[test]
fn reach_cut_flow_area_uses_approach_ineffective_without_guide_banks() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let sections = approach_sections_with_ineffective_overbank();
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        coeff_contraction: 0.3,
        length: 50.0,
        ..Default::default()
    };
    let geom = build_bridge_geometry(
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
    let wsel = 2.5;
    let a_approach = reach_cut_flow_area(&geom, true, wsel).expect("approach ineffective area");
    let table = geom.table_approach.as_ref().expect("approach table");
    let xs = geom.xs_approach.as_ref().expect("approach xs");
    let row = row_at_elevation(table, xs, wsel, None, None);
    assert!((a_approach - row.active_area).abs() < 1e-2);
    let a_bu = obstructed_hydraulics(&table_up, wsel, geom.z_up_m, &geom, true).a_eff;
    assert!(
        a_approach < a_bu - 1.0,
        "ineffective approach should convey less than BU reach face"
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

/// Taper top=1 / bottom=2 vs constant prism at mean width 1.5 (same pier height 0â†’4 m).
fn tapered_vs_mean_constant_pier_geometries() -> (BridgeGeometry, BridgeGeometry) {
    let base = BridgeGeometry {
        pier_width_m: 1.5,
        num_piers: 1,
        pier_stations_m: vec![5.0],
        pier_specs: vec![],
        low_chord_m: 4.0,
        low_chord_max_m: 4.0,
        high_chord_m: 6.0,
        high_chord_max_m: 6.0,
        pier_shape: PierShape::Square,
        abutments: BridgeAbutments::default(),
        weir_coeff_m: 1.44,
        orifice_coeff: 0.5,
        z_up_m: 0.0,
        z_down_m: 0.0,
        low_flow_method: LowFlowMethod::Yarnell,
        high_flow_method: HighFlowMethod::PressureWeir,
        length_m: 50.0,
        wspro_coeff_c: 0.8,
        coeff_contraction: 0.1,
        coeff_expansion: 0.3,
        pressure_coeff_inlet: 0.0,
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: 0.98,
        deck: None,
        skew_deg: 0.0,
        skew_cos: 1.0,
        ineffective_up: None,
        ineffective_down: None,
        xs_up: None,
        xs_down: None,
        xs_approach: None,
        xs_departure: None,
        guide_banks_approach: None,
        guide_banks_departure: None,
        table_approach: None,
        table_departure: None,
        deck_vents: Vec::new(),
    };
    let mean_constant = BridgeGeometry {
        pier_specs: vec![ResolvedPier {
            station_m: 5.0,
            spec: PierWidthSpec::Constant { width_perp_m: 1.5 },
            nosing: None,
        }],
        ..base.clone()
    };
    let tapered = BridgeGeometry {
        pier_specs: vec![ResolvedPier {
            station_m: 5.0,
            spec: PierWidthSpec::Tapered {
                top_width_perp_m: 1.0,
                bottom_width_perp_m: 2.0,
                z_base_m: 0.0,
                z_top_m: 4.0,
            },
            nosing: None,
        }],
        ..base
    };
    (mean_constant, tapered)
}

fn tapered_vs_rectangular_pier_geometries() -> (BridgeGeometry, BridgeGeometry) {
    let rectangular = BridgeGeometry {
        pier_width_m: 1.0,
        num_piers: 1,
        pier_stations_m: vec![5.0],
        pier_specs: vec![ResolvedPier {
            station_m: 5.0,
            spec: PierWidthSpec::Constant { width_perp_m: 1.0 },
            nosing: None,
        }],
        low_chord_m: 4.0,
        low_chord_max_m: 4.0,
        high_chord_m: 6.0,
        high_chord_max_m: 6.0,
        pier_shape: PierShape::Square,
        abutments: BridgeAbutments::default(),
        weir_coeff_m: 1.44,
        orifice_coeff: 0.5,
        z_up_m: 0.0,
        z_down_m: 0.0,
        low_flow_method: LowFlowMethod::Momentum,
        high_flow_method: HighFlowMethod::PressureWeir,
        length_m: 50.0,
        wspro_coeff_c: 0.8,
        coeff_contraction: 0.1,
        coeff_expansion: 0.3,
        pressure_coeff_inlet: 0.0,
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: 0.98,
        deck: None,
        skew_deg: 0.0,
        skew_cos: 1.0,
        ineffective_up: None,
        ineffective_down: None,
        xs_up: None,
        xs_down: None,
        xs_approach: None,
        xs_departure: None,
        guide_banks_approach: None,
        guide_banks_departure: None,
        table_approach: None,
        table_departure: None,
        deck_vents: Vec::new(),
    };
    let tapered = BridgeGeometry {
        pier_specs: vec![ResolvedPier {
            station_m: 5.0,
            spec: PierWidthSpec::Tapered {
                top_width_perp_m: 1.0,
                bottom_width_perp_m: 2.0,
                z_base_m: 0.0,
                z_top_m: 4.0,
            },
            nosing: None,
        }],
        ..rectangular.clone()
    };
    (rectangular, tapered)
}

#[test]
fn test_tapered_vs_mean_constant_equal_net_area_at_low_chord() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (mean_const, taper) = tapered_vs_mean_constant_pier_geometries();
    let a_mean = net_opening_area_at_low_chord(&mean_const, &table, &table);
    let a_taper = net_opening_area_at_low_chord(&taper, &table, &table);
    assert!((a_mean - 34.0).abs() < 0.1, "mean constant net: {a_mean}");
    assert!((a_taper - a_mean).abs() < 0.1, "full pier: taper {a_taper} vs mean {a_mean}");
}

#[test]
fn test_tapered_vs_mean_constant_partial_wsel_more_blockage() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (mean_const, taper) = tapered_vs_mean_constant_pier_geometries();
    let wsel = 2.5;
    let props_mean = obstructed_hydraulics(&table, wsel, 0.0, &mean_const, false);
    let props_taper = obstructed_hydraulics(&table, wsel, 0.0, &taper, false);
    assert!(props_taper.a_eff < props_mean.a_eff);
    // WSEL=2.5: taper surface width 1.375 m < mean constant 1.5 m â†’ more conveyance top width.
    assert!(props_taper.top_width > props_mean.top_width);
}

#[test]
fn test_tapered_vs_mean_constant_yarnell_higher_at_partial_depth() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (mean_const, taper) = tapered_vs_mean_constant_pier_geometries();
    let q = 15.0;
    let tw = 2.5;
    let flow_mean = yarnell_downstream_flow_area_m2(&table, tw, mean_const.z_down_m, &mean_const);
    let flow_taper = yarnell_downstream_flow_area_m2(&table, tw, taper.z_down_m, &taper);
    let hl_mean = yarnell_pier_head_loss_integrated(q, tw, mean_const.z_down_m, &mean_const, flow_mean);
    let hl_taper = yarnell_pier_head_loss_integrated(q, tw, taper.z_down_m, &taper, flow_taper);
    assert!(hl_taper > hl_mean + 1e-6);
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
fn test_tapered_pier_yarnell_head_loss_exceeds_rectangular() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (rect, taper) = tapered_vs_rectangular_pier_geometries();
    let q = 15.0;
    let tw = 2.5;
    let flow_rect = yarnell_downstream_flow_area_m2(&table, tw, rect.z_down_m, &rect);
    let flow_taper = yarnell_downstream_flow_area_m2(&table, tw, taper.z_down_m, &taper);
    let hl_rect = yarnell_pier_head_loss_integrated(q, tw, rect.z_down_m, &rect, flow_rect);
    let hl_taper = yarnell_pier_head_loss_integrated(q, tw, taper.z_down_m, &taper, flow_taper);
    assert!(hl_taper > hl_rect + 1e-6);
}

#[test]
fn test_tapered_pier_momentum_drag_exceeds_rectangular() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (rect, taper) = tapered_vs_rectangular_pier_geometries();
    let q = 15.0;
    let wsel = 2.5;
    let drag_rect = pier_drag_momentum_with_table(&table, q, wsel, rect.z_up_m, &rect, true);
    let drag_taper = pier_drag_momentum_with_table(&table, q, wsel, taper.z_up_m, &taper, true);
    assert!(drag_taper > drag_rect + 1e-6);
}

#[test]
fn test_tapered_pier_pressure_net_area_less_than_rectangular() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (rect, taper) = tapered_vs_rectangular_pier_geometries();
    let a_rect = net_opening_area_at_low_chord(&rect, &table, &table);
    let a_taper = net_opening_area_at_low_chord(&taper, &table, &table);
    assert!(a_taper < a_rect);
    assert!((a_rect - 36.0).abs() < 0.1, "rect net @ low chord: {a_rect}");
    // Trapezoid pier: 0.5 * (2 + 1) * 4 m height = 6 mÂ² â†’ 40 âˆ’ 6 = 34 mÂ²
    assert!((a_taper - 34.0).abs() < 0.1, "taper net @ low chord: {a_taper}");
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
fn test_tapered_pier_obstructed_area_exceeds_rectangular() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (rectangular, tapered) = tapered_vs_rectangular_pier_geometries();
    let wsel = 2.0;
    let props_rect = obstructed_hydraulics(&table, wsel, 0.0, &rectangular, false);
    let props_taper = obstructed_hydraulics(&table, wsel, 0.0, &tapered, false);
    assert!(props_taper.a_eff < props_rect.a_eff);
    // Mid-depth tapered submerged area 0.5*(2+1.5)*2 = 3.5 vs rectangular 1*2 = 2
    let a_rect_piers = 2.0;
    let a_taper_piers = 3.5;
    let base = 10.0 * wsel;
    assert!((props_rect.a_eff - (base - a_rect_piers)).abs() < 0.05);
    assert!((props_taper.a_eff - (base - a_taper_piers)).abs() < 0.05);
}

#[test]
fn test_legacy_constant_pier_area_unchanged_via_empty_specs() {
    let table = rectangular_table(10.0, 0.0, 50);
    let geom = BridgeGeometry {
        pier_width_m: 0.5,
        num_piers: 2,
        pier_stations_m: vec![],
        pier_specs: vec![],
        low_chord_m: 5.0,
        low_chord_max_m: 5.0,
        high_chord_m: 7.0,
        high_chord_max_m: 7.0,
        pier_shape: PierShape::Square,
        abutments: BridgeAbutments::default(),
        weir_coeff_m: 1.44,
        orifice_coeff: 0.5,
        z_up_m: 0.0,
        z_down_m: 0.0,
        low_flow_method: LowFlowMethod::Momentum,
        high_flow_method: HighFlowMethod::PressureWeir,
        length_m: 50.0,
        wspro_coeff_c: 0.8,
        coeff_contraction: 0.1,
        coeff_expansion: 0.3,
        pressure_coeff_inlet: 0.0,
        pressure_coeff_submerged: 0.8,
        max_weir_submergence: 0.98,
        deck: None,
        skew_deg: 0.0,
        skew_cos: 1.0,
        ineffective_up: None,
        ineffective_down: None,
        xs_up: None,
        xs_down: None,
        xs_approach: None,
        xs_departure: None,
        guide_banks_approach: None,
        guide_banks_departure: None,
        table_approach: None,
        table_departure: None,
        deck_vents: Vec::new(),
    };
    let wsel = 3.0;
    let props = obstructed_hydraulics(&table, wsel, 0.0, &geom, false);
    let a_piers_legacy = 2.0 * 0.5 * wsel;
    let base = 10.0 * wsel;
    assert!((props.a_eff - (base - a_piers_legacy)).abs() < 0.1);
}

fn profile_pier_geometry() -> BridgeGeometry {
    let (_, tapered) = tapered_vs_rectangular_pier_geometries();
    BridgeGeometry {
        pier_specs: vec![ResolvedPier {
            station_m: 5.0,
            spec: PierWidthSpec::Profile {
                elevations_m: vec![0.0, 4.0],
                widths_perp_m: vec![2.0, 1.0],
            },
            nosing: None,
        }],
        ..tapered
    }
}

#[test]
fn test_profile_pier_obstructed_area_matches_tapered_trapezoid() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (_, tapered) = tapered_vs_rectangular_pier_geometries();
    let profile = profile_pier_geometry();
    let wsel = 2.0;
    let a_taper = obstructed_hydraulics(&table, wsel, 0.0, &tapered, false).a_eff;
    let a_profile = obstructed_hydraulics(&table, wsel, 0.0, &profile, false).a_eff;
    assert!((a_taper - a_profile).abs() < 1e-6, "taper={a_taper}, profile={a_profile}");
}

#[test]
fn test_tapered_pier_skew_increases_opening_plane_blockage() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (_, mut geom) = tapered_vs_rectangular_pier_geometries();
    let wsel = 2.0;
    geom.skew_cos = 1.0;
    geom.skew_deg = 0.0;
    let props_normal = obstructed_hydraulics(&table, wsel, 0.0, &geom, false);
    geom.skew_cos = 0.5;
    geom.skew_deg = 60.0;
    let props_skew = obstructed_hydraulics(&table, wsel, 0.0, &geom, false);
    assert!(props_skew.a_eff < props_normal.a_eff);
    let pier_block_normal = 10.0 * wsel - props_normal.a_eff;
    let pier_block_skew = 10.0 * wsel - props_skew.a_eff;
    assert!((pier_block_skew - 2.0 * pier_block_normal).abs() < 0.1);
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

fn pier_with_footing_geometry() -> BridgeGeometry {
    let (_, base) = tapered_vs_rectangular_pier_geometries();
    BridgeGeometry {
        pier_specs: resolve_pier_width_specs(
            1.0,
            &[5.0],
            0.0,
            &[4.0],
            Some(&PierWidthUserInput {
                top_widths: Some(vec![1.0]),
                bottom_widths: Some(vec![2.0]),
                base_elevations: Some(vec![0.0]),
                ..Default::default()
            }),
            Some(&PierAttachmentsUserInput {
                footing_top_elevations: Some(vec![0.0]),
                footing_widths: Some(vec![3.0]),
                footing_bottom_elevations: Some(vec![-1.0]),
                ..Default::default()
            }),
        ),
        ..base
    }
}

fn pier_with_nosing_geometry() -> BridgeGeometry {
    let (_, base) = tapered_vs_rectangular_pier_geometries();
    BridgeGeometry {
        pier_specs: resolve_pier_width_specs(
            1.0,
            &[5.0],
            0.0,
            &[4.0],
            None,
            Some(&PierAttachmentsUserInput {
                nosing_lengths: Some(vec![0.5]),
                ..Default::default()
            }),
        ),
        ..base
    }
}

#[test]
fn test_footing_increases_obstructed_area_below_shaft() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (rectangular, _) = tapered_vs_rectangular_pier_geometries();
    let footing = pier_with_footing_geometry();
    let wsel = 2.0;
    let a_shaft = obstructed_hydraulics(&table, wsel, 0.0, &rectangular, false).a_eff;
    let a_footing = obstructed_hydraulics(&table, wsel, 0.0, &footing, false).a_eff;
    assert!(a_footing < a_shaft);
}

/// Submerged footing band (bed â†’ shaft base) widens pier plan area and lowers contracted opening `a_eff`.
#[test]
fn test_submerged_footing_reduces_opening_area() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (rectangular, _) = tapered_vs_rectangular_pier_geometries();
    let wsel = 2.0;
    let channel_area = 10.0 * wsel;
    let shaft_user = PierWidthUserInput {
        top_widths: Some(vec![1.0]),
        bottom_widths: Some(vec![1.0]),
        base_elevations: Some(vec![1.0]),
        top_elevations: Some(vec![4.0]),
        ..Default::default()
    };
    let footing_attach = PierAttachmentsUserInput {
        footing_top_elevations: Some(vec![1.0]),
        footing_widths: Some(vec![3.0]),
        footing_bottom_elevations: Some(vec![0.0]),
        ..Default::default()
    };
    let shaft_only = BridgeGeometry {
        pier_specs: resolve_pier_width_specs(
            1.0,
            &[5.0],
            0.0,
            &[4.0],
            Some(&shaft_user),
            None,
        ),
        ..rectangular.clone()
    };
    let with_footing = BridgeGeometry {
        pier_specs: resolve_pier_width_specs(
            1.0,
            &[5.0],
            0.0,
            &[4.0],
            Some(&shaft_user),
            Some(&footing_attach),
        ),
        ..rectangular
    };
    let props_shaft = obstructed_hydraulics(&table, wsel, 0.0, &shaft_only, false);
    let props_footing = obstructed_hydraulics(&table, wsel, 0.0, &with_footing, false);
    // Shaft wet 1â†’2 m at 1 m wide â†’ 1 mÂ² pier; footing 0â†’1 m tapers 3â†’1 m â†’ +2 mÂ².
    let a_pier_shaft = 1.0;
    let a_pier_footing = 3.0;
    let delta_opening = props_shaft.a_eff - props_footing.a_eff;
    assert!(
        delta_opening > 1.5,
        "footing must reduce opening area: shaft {:.4} footing {:.4}",
        props_shaft.a_eff,
        props_footing.a_eff,
    );
    assert!(
        (delta_opening - (a_pier_footing - a_pier_shaft)).abs() < 0.05,
        "opening reduction {:.4} vs extra pier area {:.4}",
        delta_opening,
        a_pier_footing - a_pier_shaft,
    );
    assert!(
        (props_shaft.a_eff - (channel_area - a_pier_shaft)).abs() < 0.05,
        "shaft opening {:.4} vs hand {:.4}",
        props_shaft.a_eff,
        channel_area - a_pier_shaft,
    );
    assert!(
        (props_footing.a_eff - (channel_area - a_pier_footing)).abs() < 0.05,
        "footing opening {:.4} vs hand {:.4}",
        props_footing.a_eff,
        channel_area - a_pier_footing,
    );
}

#[test]
fn test_nosing_increases_momentum_drag() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (rectangular, _) = tapered_vs_rectangular_pier_geometries();
    let nosing = pier_with_nosing_geometry();
    let wsel = 2.0;
    let q = 15.0;
    let drag_shaft = pier_drag_momentum_with_table(&table, q, wsel, 0.0, &rectangular, true);
    let drag_nosing = pier_drag_momentum_with_table(&table, q, wsel, 0.0, &nosing, true);
    assert!(drag_nosing > drag_shaft + 1e-6);
}

#[test]
fn test_nosing_reduces_obstructed_top_width() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (rectangular, _) = tapered_vs_rectangular_pier_geometries();
    let nosing = pier_with_nosing_geometry();
    let wsel = 2.0;
    let tw_shaft = obstructed_hydraulics(&table, wsel, 0.0, &rectangular, false).top_width;
    let tw_nosing = obstructed_hydraulics(&table, wsel, 0.0, &nosing, false).top_width;
    assert!(
        tw_nosing + 0.5 < tw_shaft + 1e-6,
        "nosing top_width {tw_nosing} vs shaft {tw_shaft}"
    );
}

#[test]
fn test_yarnell_integrated_loss_increases_with_footing() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (rectangular, _) = tapered_vs_rectangular_pier_geometries();
    let footing = pier_with_footing_geometry();
    let wsel = 2.5;
    let q = 15.0;
    let a_shaft = obstructed_hydraulics(&table, wsel, 0.0, &rectangular, false).a_eff;
    let a_footing = obstructed_hydraulics(&table, wsel, 0.0, &footing, false).a_eff;
    let hl_shaft = yarnell_pier_head_loss_integrated(q, wsel, 0.0, &rectangular, a_shaft);
    let hl_footing = yarnell_pier_head_loss_integrated(q, wsel, 0.0, &footing, a_footing);
    assert!(hl_footing > hl_shaft + 1e-6);
}

fn pier_shaft_only_sections() -> BridgeSectionContext {
    BridgeSectionContext {
        pier_stations: Some(vec![5.0]),
        pier_widths: Some(PierWidthUserInput {
            top_widths: Some(vec![1.0]),
            bottom_widths: Some(vec![1.0]),
            base_elevations: Some(vec![1.0]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn pier_footing_nosing_sections() -> BridgeSectionContext {
    BridgeSectionContext {
        pier_stations: Some(vec![5.0]),
        pier_widths: Some(PierWidthUserInput {
            top_widths: Some(vec![1.0]),
            bottom_widths: Some(vec![1.0]),
            base_elevations: Some(vec![1.0]),
            ..Default::default()
        }),
        pier_attachments: Some(PierAttachmentsUserInput {
            footing_top_elevations: Some(vec![1.0]),
            footing_widths: Some(vec![3.0]),
            footing_bottom_elevations: Some(vec![0.0]),
            nosing_lengths: Some(vec![0.5]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn test_footing_nosing_exceed_shaft_only_headwater_in_solve() {
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
    let hw_shaft = solve(&pier_shaft_only_sections());
    let hw_attach = solve(&pier_footing_nosing_sections());
    assert!(
        hw_attach > hw_shaft + 1e-4,
        "footing+nosing HW {hw_attach} vs shaft-only {hw_shaft}"
    );
}

#[test]
fn test_partially_submerged_deck_with_vents() {
    use crate::solvers::deck_vent_geometry::{resolve_deck_vents, total_deck_vent_discharge_m3s};

    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 10.0],
        low_elevations_m: vec![5.0, 5.0],
        high_elevations_m: vec![7.0, 7.0],
    };
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        high_flow_method: 0,
        ..Default::default()
    };
    let sections = BridgeSectionContext {
        deck_vents: Some(DeckVentUserInput {
            left_stations: Some(vec![2.0]),
            right_stations: Some(vec![4.0]),
            invert_elevations: Some(vec![5.2]),
            soffit_elevations: Some(vec![5.9]),
            discharge_coefficients: Some(vec![0.8]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let geom = build_bridge_geometry(
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        UnitSystem::Metric,
        &coupling,
        50.0,
        Some(&deck),
        Some(&sections),
    );
    let tw = 5.3;
    let wsel = 5.55;
    assert!(
        wsel > geom.low_chord_m && wsel < geom.high_chord_m,
        "deck in pressure regime only"
    );

    let vents = resolve_deck_vents(sections.deck_vents.as_ref().unwrap(), 1.0, UnitSystem::Metric, 0.8);
    let h_sub = wsel - vents[0].invert_m;
    assert!(
        h_sub < vents[0].slot_height_m - 1e-6,
        "vent should be partly submerged, not full slot"
    );
    assert!((vents[0].submerged_area_m2(wsel) - 2.0 * h_sub).abs() < 1e-9);

    let q_metric = 100.0;
    let a_net = net_opening_area_at_low_chord(&geom, &table_up, &table_down);
    let parts = combined_high_flow_discharge(wsel, tw, q_metric, &geom, &table_up, a_net, Some(10.0));
    assert!(parts.q_opening_m3s > 1.0, "main submerged orifice active");
    assert!(parts.q_vents_m3s > 0.05, "vents active above invert");
    assert!(
        parts.q_weir_m3s.abs() < 1e-6,
        "no roadway weir below high chord"
    );

    let e_up = upstream_energy_grade(wsel, q_metric, &geom, &table_up, geom.z_up_m, true);
    let q_vent_hand =
        total_deck_vent_discharge_m3s(&vents, wsel, e_up, tw);
    assert!((parts.q_vents_m3s - q_vent_hand).abs() < 1e-4);

    let q = 100.0;
    let solve = |sections: Option<&BridgeSectionContext>| {
        solve_bridge_wsel(
            q,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.8,
            0.0,
            0.0,
            tw,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            Some(&deck),
            sections,
        )
    };
    let hw = solve(Some(&sections));
    assert!(
        hw > vents[0].invert_m + 1e-3 && hw < vents[0].soffit_m - 1e-3,
        "solved HW should partly fill vent, got hw={hw}"
    );
    assert!(hw < geom.high_chord_m - 1e-3, "no deck overtopping");

    let parts_solved =
        combined_high_flow_discharge(hw, tw, q, &geom, &table_up, a_net, Some(10.0));
    assert!(
        (parts_solved.total_m3s() - q).abs() < 0.05,
        "opening={} vents={} weir={} sum={}",
        parts_solved.q_opening_m3s,
        parts_solved.q_vents_m3s,
        parts_solved.q_weir_m3s,
        parts_solved.total_m3s()
    );

    let hw_no_vents = solve(None);
    assert!(
        hw < hw_no_vents - 1e-4,
        "vents should lower HW in partial submergence: with={hw}, without={hw_no_vents}"
    );
}

#[test]
fn test_deck_vents_reduce_headwater_when_main_opening_submerged() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 10.0],
        low_elevations_m: vec![5.0, 5.0],
        high_elevations_m: vec![7.0, 7.0],
    };
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        high_flow_method: 0,
        ..Default::default()
    };
    let q = 80.0;
    let tw = 5.2;
    let solve = |sections: Option<&BridgeSectionContext>| {
        solve_bridge_wsel(
            q,
            5.0,
            7.0,
            0.0,
            0,
            0,
            1.44,
            0.8,
            0.0,
            0.0,
            tw,
            UnitSystem::Metric,
            &table_up,
            &table_down,
            &coupling,
            50.0,
            Some(&deck),
            sections,
        )
    };
    let hw_main_only = solve(None);
    let sections_vents = BridgeSectionContext {
        deck_vents: Some(DeckVentUserInput {
            left_stations: Some(vec![3.0, 6.0]),
            right_stations: Some(vec![4.0, 7.0]),
            invert_elevations: Some(vec![5.0, 5.0]),
            soffit_elevations: Some(vec![5.6, 5.6]),
            discharge_coefficients: Some(vec![0.75, 0.75]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let hw_with_vents = solve(Some(&sections_vents));
    assert!(
        hw_with_vents < hw_main_only - 1e-4,
        "vents should lower HW under submerged deck: main={hw_main_only}, vents={hw_with_vents}"
    );
}

#[test]
fn test_deck_vents_parallel_orifice_matches_hand_calc() {
    use crate::solvers::deck_vent_geometry::{resolve_deck_vents, total_deck_vent_discharge_m3s};

    let user = DeckVentUserInput {
        left_stations: Some(vec![0.0]),
        right_stations: Some(vec![2.0]),
        invert_elevations: Some(vec![10.0]),
        soffit_elevations: Some(vec![12.0]),
        discharge_coefficients: Some(vec![0.8]),
        ..Default::default()
    };
    let vents = resolve_deck_vents(&user, 1.0, UnitSystem::Metric, 0.8);
    let wsel = 11.0;
    let e_up = 11.5;
    let tw = 8.0;
    let q = total_deck_vent_discharge_m3s(&vents, wsel, e_up, tw);
    let a = 2.0;
    let head = 3.5;
    let expected = 0.8 * a * (2.0 * crate::utils::G_METRIC * head).sqrt();
    assert!((q - expected).abs() < 1e-6);
}

#[test]
fn test_combined_high_flow_opening_vents_weir_sum_to_q() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 10.0],
        low_elevations_m: vec![5.0, 5.0],
        high_elevations_m: vec![7.0, 7.0],
    };
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        high_flow_method: 0,
        ..Default::default()
    };
    let sections = BridgeSectionContext {
        deck_vents: Some(DeckVentUserInput {
            left_stations: Some(vec![2.0, 6.0]),
            right_stations: Some(vec![3.0, 7.0]),
            invert_elevations: Some(vec![5.1, 5.1]),
            soffit_elevations: Some(vec![5.7, 5.7]),
            discharge_coefficients: Some(vec![0.75, 0.75]),
            ..Default::default()
        }),
        ..Default::default()
    };
    // Q must exceed pressure-only capacity (opening + vents) so weir overtopping engages.
    let q = 300.0;
    let tw = 5.5;
    let hw = solve_bridge_wsel(
        q,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        tw,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        Some(&deck),
        Some(&sections),
    );
    assert!(
        hw > 7.0 + 1e-4,
        "combined overtopping expected above high chord, got hw={hw}"
    );

    let geom = build_bridge_geometry(
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        UnitSystem::Metric,
        &coupling,
        50.0,
        Some(&deck),
        Some(&sections),
    );
    let a_net = net_opening_area_at_low_chord(&geom, &table_up, &table_down);
    let e_up = upstream_energy_grade(hw, q, &geom, &table_up, geom.z_up_m, true);
    let l_weir = effective_weir_length_m(&geom, e_up, 10.0);
    let parts = combined_high_flow_discharge(hw, tw, q, &geom, &table_up, a_net, Some(l_weir));

    assert!(
        (parts.total_m3s() - q).abs() < 1e-2,
        "Q_opening={} Q_vents={} Q_weir={} sum={} target={}",
        parts.q_opening_m3s,
        parts.q_vents_m3s,
        parts.q_weir_m3s,
        parts.total_m3s(),
        q
    );
    assert!(parts.q_opening_m3s > 1.0, "main opening should carry flow");
    assert!(parts.q_vents_m3s > 0.1, "vents should carry flow");
    assert!(parts.q_weir_m3s > 0.1, "weir should carry flow");
}

#[test]
fn test_combined_high_flow_weir_only_when_below_high_chord() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 10.0],
        low_elevations_m: vec![5.0, 5.0],
        high_elevations_m: vec![7.0, 7.0],
    };
    let coupling = BridgeCouplingParams::default();
    let geom = build_bridge_geometry(
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        UnitSystem::Metric,
        &coupling,
        50.0,
        Some(&deck),
        None,
    );
    let a_net = net_opening_area_at_low_chord(&geom, &table_up, &table_down);
    let wsel = 6.0;
    let tw = 5.5;
    let q = 50.0;
    let pressure_only =
        combined_high_flow_discharge(wsel, tw, q, &geom, &table_up, a_net, None);
    let with_weir = combined_high_flow_discharge(wsel, tw, q, &geom, &table_up, a_net, Some(10.0));
    assert!((pressure_only.q_weir_m3s).abs() < 1e-9);
    assert!((with_weir.q_weir_m3s).abs() < 1e-9);
    assert!(
        (pressure_only.total_m3s() - with_weir.total_m3s()).abs() < 1e-9,
        "below high chord weir term should be zero"
    );
}
