use super::*;

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
        friction_opening_m: 100.0,
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
        ..Default::default()
    };
    assert_eq!(
        classify_low_flow(15.0, 3.0, &geom, &table, &table),
        LowFlowClass::A
    );
}
#[test]
fn test_class_b_energy_friction_segments_raise_hw() {
    let (q, tw, pier_w, num_piers, table_up, table_down) = class_b_energy_case();
    let sections = class_b_friction_sections();
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
    let geom_opening = build_bridge_geometry(
        5.0,
        7.0,
        pier_w,
        num_piers,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        UnitSystem::Metric,
        &coupling_opening,
        20.0,
        None,
        Some(&sections),
    );
    let geom_segments = build_bridge_geometry(
        5.0,
        7.0,
        pier_w,
        num_piers,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        UnitSystem::Metric,
        &coupling_segments,
        20.0,
        None,
        Some(&sections),
    );
    let geom_classify = build_bridge_geometry(
        5.0,
        7.0,
        pier_w,
        num_piers,
        0,
        1.44,
        0.5,
        0.0,
        0.0,
        UnitSystem::Metric,
        &coupling_segments,
        15.0,
        None,
        None,
    );
    assert_eq!(
        classify_low_flow(q, tw, &geom_classify, &table_up, &table_down),
        LowFlowClass::B
    );

    let wsel_sample = (tw + 1.5).min(geom_segments.low_chord_m - 0.05);
    let hf_opening = bridge_energy_friction_loss(
        q,
        wsel_sample,
        tw,
        &geom_opening,
        &table_up,
        &table_down,
    );
    let hf_segments = bridge_energy_friction_loss(
        q,
        wsel_sample,
        tw,
        &geom_segments,
        &table_up,
        &table_down,
    );
    assert!(
        hf_segments > hf_opening + 1e-6,
        "Class B energy friction: segments={hf_segments} should exceed opening={hf_opening}"
    );

    let hw_opening = solve_low_flow_class_b(q, tw, &geom_opening, &table_up, &table_down);
    let hw_segments = solve_low_flow_class_b(q, tw, &geom_segments, &table_up, &table_down);
    let ceiling = geom_segments.low_chord_m + 20.0;
    if hw_opening < ceiling - 0.5 && hw_segments < ceiling - 0.5 {
        assert!(
            hw_segments > hw_opening,
            "Class B energy HW with converged solve: opening={hw_opening}, segments={hw_segments}"
        );
    }
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
fn test_combined_regime_label_from_solver() {
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
    let result = solve_bridge_coupled(
        300.0,
        5.0,
        7.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        5.5,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        Some(&deck),
        None,
    );
    assert_eq!(
        result.flow_regime, "weir",
        "combined overtopping solve should report weir regime, got {}",
        result.flow_regime
    );
}
