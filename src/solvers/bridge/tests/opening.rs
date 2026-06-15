use super::*;

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
    let props_no = obstructed_hydraulics(&table, 3.0, 0.0, &geom_no_abut, false);
    let geom_abut = BridgeGeometry {
        abutments: BridgeAbutments::symmetric_total_width_m(2.0, 0.0, 10.0),
        ..geom_no_abut.clone()
    };
    let props_abut = obstructed_hydraulics(&table, 3.0, 0.0, &geom_abut, false);
    assert!(props_abut.a_eff < props_no.a_eff);
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
        friction_opening_m: 50.0,
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
        friction_opening_m: 50.0,
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
        friction_opening_m: 50.0,
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
        ..Default::default()
    };
    let wsel = 3.0;
    let props = obstructed_hydraulics(&table, wsel, 0.0, &geom, false);
    let a_piers_legacy = 2.0 * 0.5 * wsel;
    let base = 10.0 * wsel;
    assert!((props.a_eff - (base - a_piers_legacy)).abs() < 0.1);
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
fn test_footing_increases_obstructed_area_below_shaft() {
    let table = rectangular_table(10.0, 0.0, 50);
    let (rectangular, _) = tapered_vs_rectangular_pier_geometries();
    let footing = pier_with_footing_geometry();
    let wsel = 2.0;
    let a_shaft = obstructed_hydraulics(&table, wsel, 0.0, &rectangular, false).a_eff;
    let a_footing = obstructed_hydraulics(&table, wsel, 0.0, &footing, false).a_eff;
    assert!(a_footing < a_shaft);
}
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
fn test_opening_blockage_factor_raises_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let base = BridgeCouplingParams {
        low_flow_method: 3,
        length: 50.0,
        ..Default::default()
    };
    let mut blocked = base.clone();
    blocked.ice_debris.opening_blockage_factor = 0.65;
    let q = 25.0;
    let tw = 2.5;
    let hw_clear = solve_bridge_wsel(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw, UnitSystem::Metric, &table_up,
        &table_down, &base, 50.0, None, None,
    );
    let hw_blocked = solve_bridge_wsel(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw, UnitSystem::Metric, &table_up,
        &table_down, &blocked, 50.0, None, None,
    );
    assert!(
        hw_blocked > hw_clear + 1e-4,
        "blockage factor should raise HW: clear={hw_clear}, blocked={hw_blocked}"
    );
}
#[test]
fn test_pier_debris_raises_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let base = BridgeCouplingParams {
        low_flow_method: 3,
        length: 50.0,
        ..Default::default()
    };
    let mut debris = base.clone();
    debris.ice_debris.pier_debris_widths = vec![4.0];
    debris.ice_debris.pier_debris_heights = vec![2.0];
    let sections = BridgeSectionContext {
        pier_stations: Some(vec![5.0]),
        ..Default::default()
    };
    let q = 20.0;
    let tw = 2.5;
    let hw_clear = solve_bridge_wsel(
        q, 5.0, 7.0, 1.0, 1, 0, 1.44, 0.5, 0.0, 0.0, tw, UnitSystem::Metric, &table_up,
        &table_down, &base, 50.0, None, Some(&sections),
    );
    let hw_debris = solve_bridge_wsel(
        q, 5.0, 7.0, 1.0, 1, 0, 1.44, 0.5, 0.0, 0.0, tw, UnitSystem::Metric, &table_up,
        &table_down, &debris, 50.0, None, Some(&sections),
    );
    assert!(
        hw_debris > hw_clear + 1e-4,
        "pier debris should raise HW: clear={hw_clear}, debris={hw_debris}"
    );
}
#[test]
fn test_ice_thickness_raises_headwater() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let base = BridgeCouplingParams {
        low_flow_method: 3,
        length: 50.0,
        ..Default::default()
    };
    let mut iced = base.clone();
    iced.ice_debris.ice_mode = 1;
    iced.ice_debris.ice_thickness = 0.75;
    let q = 22.0;
    let tw = 2.5;
    let hw_clear = solve_bridge_wsel(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw, UnitSystem::Metric, &table_up,
        &table_down, &base, 50.0, None, None,
    );
    let hw_iced = solve_bridge_wsel(
        q, 5.0, 7.0, 0.0, 0, 0, 1.44, 0.5, 0.0, 0.0, tw, UnitSystem::Metric, &table_up,
        &table_down, &iced, 50.0, None, None,
    );
    assert!(
        hw_iced > hw_clear + 1e-4,
        "ice thickness should raise HW: clear={hw_clear}, iced={hw_iced}"
    );
}

#[test]
fn internal_opening_friction_segments_direct_and_edge_cases() {
    use crate::geometry::CrossSection;
    use crate::solvers::bridge::geometry::internal_opening_friction_segments;

    let empty = internal_opening_friction_segments(None, &[], None, 10.0);
    assert!(empty.0.is_empty() && empty.1.is_empty() && empty.2.is_empty());

    let bu = CrossSection {
        station: 100.0,
        x: vec![0.0, 0.0, 10.0, 10.0],
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
    let internal = CrossSection {
        station: 50.0,
        x: vec![0.0, 0.0, 10.0, 10.0],
        y: vec![4.5, 0.0, 0.0, 4.5],
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
    let bd = CrossSection {
        station: 0.0,
        x: vec![0.0, 0.0, 10.0, 10.0],
        y: vec![4.0, 0.0, 0.0, 4.0],
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

    let (tables, lengths, z_m) =
        internal_opening_friction_segments(Some(&bu), &[internal], Some(&bd), 20.0);
    assert_eq!(tables.len(), 1);
    assert_eq!(lengths.len(), 2);
    assert_eq!(z_m.len(), 1);
    assert!(lengths[0] > 45.0, "skew should lengthen segments, got {}", lengths[0]);

    let too_few_nodes =
        internal_opening_friction_segments(Some(&bu), &[], None, 0.0);
    assert!(too_few_nodes.0.is_empty());
}
