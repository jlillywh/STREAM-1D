use super::*;

#[test]
fn test_sluice_gate_cd_increases_with_submergence() {
    let cd_min = sluice_gate_discharge_coeff(0.0, 0.0);
    let cd_mid = sluice_gate_discharge_coeff(0.5, 0.0);
    let cd_deep = sluice_gate_discharge_coeff(1.0, 0.0);
    assert!(cd_deep > cd_mid);
    assert!(cd_mid > cd_min);
    assert!((cd_min - 0.27).abs() < 0.01);
    assert!((cd_deep - 0.5).abs() < 0.05);
    assert!((sluice_gate_discharge_coeff(0.4, 0.65) - 0.65).abs() < 1e-6);
}
#[test]
fn test_bradley_submergence_reduces_weir_factor() {
    assert!((bradley_weir_submergence_factor(0.0) - 1.0).abs() < 1e-6);
    assert!(bradley_weir_submergence_factor(0.9) < bradley_weir_submergence_factor(0.5));
    assert!(bradley_weir_submergence_factor(0.95) < 0.3);
    assert!(bradley_weir_submergence_factor(0.99) <= 0.08);
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
        coeff_contraction: None,
        coeff_expansion: None,
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
#[test]
fn test_segment_weir_before_min_high_wsel() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 5.0, 10.0],
        low_elevations_m: vec![5.0, 5.0, 5.0],
        high_elevations_m: vec![6.5, 6.5, 7.0],
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
    let a_net = net_opening_area_at_low_chord(&geom, &table_up, &table_up);
    let wsel = 6.3;
    let tw = 5.4;
    let q = 250.0;
    let e_up = upstream_energy_grade(wsel, q, &geom, &table_up, geom.z_up_m, true);
    assert!(
        e_up > 6.5 + 1e-3 && wsel < geom.high_chord_m - 1e-3,
        "EGL should overtop the 6.5 m segment before WSEL reaches min high chord 7.0"
    );
    let parts = combined_high_flow_discharge(wsel, tw, q, &geom, &table_up, a_net, Some(10.0));
    assert!(
        parts.q_weir_m3s > 0.05,
        "segment-wise weir should flow when EGL clears local crest, got q_weir={}",
        parts.q_weir_m3s
    );
}
#[test]
fn test_weir_submergence_energy_fallback_at_cap() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        high_flow_method: 0,
        max_weir_submergence: 0.98,
        ..Default::default()
    };
    let result = solve_bridge_coupled(
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
        7.69,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        None,
    );
    assert_eq!(
        result.flow_regime, "energy",
        "near-fully submerged weir should fall back to energy, got {}",
        result.flow_regime
    );
}
#[test]
fn test_sluice_orifice_switch_uses_max_low_chord() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 10.0],
        low_elevations_m: vec![5.0, 5.5],
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
    assert!((geom.low_chord_m - 5.0).abs() < 1e-6);
    assert!((geom.low_chord_max_m - 5.5).abs() < 1e-6);
    let a_net = net_opening_area_at_low_chord(&geom, &table_up, &table_up);
    let wsel = 6.0;
    let tw = 5.2;
    assert!(tw >= geom.low_chord_m && tw < geom.low_chord_max_m);
    let q = 100.0;
    let q_sluice = main_pressure_flow_discharge(wsel, tw, q, &geom, &table_up, a_net);
    let mut geom_orifice_early = geom.clone();
    geom_orifice_early.low_chord_max_m = geom.low_chord_m;
    let q_orifice = main_pressure_flow_discharge(
        wsel,
        tw,
        q,
        &geom_orifice_early,
        &table_up,
        a_net,
    );
    assert!(
        (q_sluice - q_orifice).abs() > 0.5,
        "TW between min and max low chord should remain sluice (max trigger), sluice={q_sluice}, orifice={q_orifice}"
    );
}
#[test]
fn test_scalar_weir_discharge_without_deck_profile() {
    let table_up = rectangular_table(10.0, 0.0, 50);
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
        None,
        None,
    );
    let wsel = 7.8;
    let tw = 7.35;
    let q = 80.0;
    let q_weir = weir_flow_discharge(wsel, tw, q, &geom, &table_up, 10.0);
    assert!(
        q_weir > 0.05,
        "scalar crest weir should discharge when EGL clears high chord, got {q_weir}"
    );
    let e_up = upstream_energy_grade(wsel, q, &geom, &table_up, geom.z_up_m, true);
    assert!(e_up > geom.high_chord_m + 1e-3);
    let ratio = max_active_weir_submergence_ratio(tw, e_up, &geom);
    assert!(
        ratio > 0.05 && ratio < 0.98,
        "partially submerged scalar weir should have Bradley ratio in (0, cap), got {ratio}"
    );
    assert!(!weir_submergence_exceeds_cap(tw, e_up, &geom));
}
#[test]
fn test_segment_weir_returns_zero_without_deck() {
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
        None,
        None,
    );
    assert_eq!(segment_weir_discharge_m3s(5.0, 8.0, &geom), 0.0);
}
#[test]
fn test_reconcile_reports_pressure_when_egl_exceeds_deck_at_hw_tie() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        high_flow_method: 0,
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
        None,
        None,
    );
    let solved = solve_bridge_headwater_metric(200.0, 2.5, &geom, &table_up, &table_down);
    assert!(
        (solved.wsel_m - geom.low_chord_m).abs() < 0.01,
        "sluice reconcile should pin HW at low chord, got {}",
        solved.wsel_m
    );
    assert_eq!(solved.regime, BridgeFlowRegimeKind::Pressure);
}
#[test]
fn test_reconcile_preserves_low_flow_regime_when_egl_below_deck() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
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
        None,
        None,
    );
    let solved = solve_bridge_headwater_metric(12.0, 2.5, &geom, &table_up, &table_down);
    assert!(solved.wsel_m < geom.low_chord_m);
    assert!(
        matches!(
            solved.regime,
            BridgeFlowRegimeKind::LowA | BridgeFlowRegimeKind::LowB | BridgeFlowRegimeKind::LowC
        ),
        "expected low-flow regime, got {:?}",
        solved.regime
    );
}
#[test]
fn test_combined_weir_solve_beats_pressure_only_capacity() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 5.0, 10.0],
        low_elevations_m: vec![5.0, 5.0, 5.0],
        high_elevations_m: vec![6.5, 6.5, 7.0],
    };
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        high_flow_method: 0,
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
        None,
    );
    let q = 220.0;
    let tw = 5.4;
    let pressure_only_hw =
        solve_pressure_headwater(q, tw, &geom, &table_up, &table_down);
    let solved = solve_bridge_headwater_metric(q, tw, &geom, &table_up, &table_down);
    assert!(
        solved.wsel_m <= pressure_only_hw + 0.02,
        "segment weir should not require more HW than pressure-only, got {} vs {}",
        solved.wsel_m,
        pressure_only_hw
    );
    assert_eq!(solved.regime, BridgeFlowRegimeKind::Weir);
    let a_net = net_opening_area_at_low_chord(&geom, &table_up, &table_down);
    let e_up = upstream_energy_grade(solved.wsel_m, q, &geom, &table_up, geom.z_up_m, true);
    let l_weir = effective_weir_length_m(&geom, e_up, 10.0);
    let parts = combined_high_flow_discharge(
        solved.wsel_m,
        tw,
        q,
        &geom,
        &table_up,
        a_net,
        Some(l_weir),
    );
    assert!((parts.total_m3s() - q).abs() < 0.05);
    assert!(parts.q_weir_m3s > 0.01);
}
#[test]
fn test_high_flow_expands_upper_bound_for_large_discharge() {
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
    let solved = solve_bridge_headwater_metric(450.0, 5.5, &geom, &table_up, &table_down);
    assert!(
        solved.wsel_m > geom.high_chord_m + 0.5,
        "large Q should require expanded upper bound, got hw={}",
        solved.wsel_m
    );
    assert_eq!(solved.regime, BridgeFlowRegimeKind::Weir);
}
#[test]
fn test_pressure_only_high_flow_regime_without_weir() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        high_flow_method: 0,
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
        None,
        None,
    );
    let solved = solve_bridge_headwater_metric(100.0, 5.4, &geom, &table_up, &table_down);
    assert_eq!(solved.regime, BridgeFlowRegimeKind::Pressure);
    let a_net = net_opening_area_at_low_chord(&geom, &table_up, &table_down);
    let parts = combined_high_flow_discharge(
        solved.wsel_m,
        5.4,
        100.0,
        &geom,
        &table_up,
        a_net,
        Some(10.0),
    );
    assert!(parts.q_weir_m3s.abs() < 1e-6);
    assert!((parts.total_m3s() - 100.0).abs() < 0.05);
}
#[test]
fn test_weir_head_active_partial_deck_segments() {
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 5.0, 10.0],
        low_elevations_m: vec![5.0, 5.0, 5.0],
        high_elevations_m: vec![6.5, 6.5, 7.0],
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
    let table_up = rectangular_table(10.0, 0.0, 50);
    let wsel = 6.4;
    let q = 150.0;
    let e_up = upstream_energy_grade(wsel, q, &geom, &table_up, geom.z_up_m, true);
    assert!(
        weir_head_active_at_energy(e_up, &geom),
        "EGL {:.4} should clear the 6.5 m center crest before global high chord",
        e_up
    );
    let e_below = 6.2;
    assert!(
        !weir_head_active_at_energy(e_below, &geom),
        "EGL below all crests should not activate weir head"
    );
}
#[test]
fn test_high_flow_tailwater_pressure_only_roundtrip() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        high_flow_method: 0,
        ..Default::default()
    };
    let q = 100.0;
    let tw_target = 5.4;
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
        tw_target,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        None,
    );
    let tw_back = solve_bridge_tailwater(
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
        hw,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        None,
        None,
    );
    assert!(
        (tw_back - tw_target).abs() < 0.02,
        "pressure-only tailwater roundtrip: tw={tw_back}, expected={tw_target}, hw={hw}"
    );
}
#[test]
fn test_high_flow_tailwater_combined_weir_roundtrip() {
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
    let q = 280.0;
    let tw_target = 5.5;
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
        tw_target,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        Some(&deck),
        None,
    );
    let tw_back = solve_bridge_tailwater(
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
        hw,
        UnitSystem::Metric,
        &table_up,
        &table_down,
        &coupling,
        50.0,
        Some(&deck),
        None,
    );
    assert!(
        (tw_back - tw_target).abs() < 0.05,
        "combined weir tailwater roundtrip: tw={tw_back}, expected={tw_target}, hw={hw}"
    );
}
#[test]
fn test_high_flow_tailwater_submergence_energy_fallback() {
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
        max_weir_submergence: 0.98,
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
        None,
    );
    let q = 280.0;
    let hw = solve_bridge_headwater_metric(q, 5.5, &geom, &table_up, &table_down).wsel_m;
    let tw_metric = solve_high_flow_tailwater(q, &geom, hw, &table_up, &table_down);
    assert!(
        tw_metric > geom.z_down_m && tw_metric <= hw + 1e-3,
        "tailwater under weir head should be physical, got tw={tw_metric}, hw={hw}"
    );
    // Deep tailwater drives Bradley submergence past cap → energy fallback branch.
    let tw_energy_path = solve_high_flow_tailwater(15.0, &geom, 7.5, &table_up, &table_down);
    assert!(
        tw_energy_path > geom.z_down_m && tw_energy_path.is_finite(),
        "near-cap submergence should still return finite tailwater, got {tw_energy_path}"
    );
}
#[test]
fn test_solve_bridge_headwater_metric_low_vs_high_flow_path() {
    let table_up = rectangular_table(10.0, 0.0, 50);
    let table_down = rectangular_table(10.0, 0.0, 50);
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        high_flow_method: 0,
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
        None,
        None,
    );
    let low_tw = solve_bridge_headwater_metric(30.0, 2.5, &geom, &table_up, &table_down);
    let high_tw = solve_bridge_headwater_metric(100.0, 5.4, &geom, &table_up, &table_down);
    assert!(low_tw.wsel_m < geom.low_chord_m);
    assert!(high_tw.wsel_m >= geom.low_chord_m - 1e-6);
    assert_ne!(low_tw.regime, BridgeFlowRegimeKind::Pressure);
    assert_eq!(high_tw.regime, BridgeFlowRegimeKind::Pressure);
}
#[test]
fn test_deck_ice_lowers_weir_onset_energy() {
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 10.0],
        low_elevations_m: vec![5.0, 5.0],
        high_elevations_m: vec![7.0, 7.0],
    };
    let base = BridgeCouplingParams {
        low_flow_method: 3,
        high_flow_method: 0,
        ..Default::default()
    };
    let mut iced = base.clone();
    iced.ice_debris.deck_ice_thickness = 0.5;
    let geom_clear = build_bridge_geometry(
        5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, UnitSystem::Metric, &base, 50.0,
        Some(&deck), None,
    );
    let geom_iced = build_bridge_geometry(
        5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, UnitSystem::Metric, &iced, 50.0,
        Some(&deck), None,
    );
    // Deck ice lowers effective crest (7.0 -> 6.5 m), so weir overtopping engages below the nominal high chord.
    let tw = 5.5;
    let e_up = 6.75;
    let q_weir_clear = segment_weir_discharge_m3s(tw, e_up, &geom_clear);
    let q_weir_iced = segment_weir_discharge_m3s(tw, e_up, &geom_iced);
    assert!(
        q_weir_clear < 1e-6,
        "without deck ice, energy below high chord should not weir, got {q_weir_clear}"
    );
    assert!(
        q_weir_iced > 1e-3,
        "deck ice should lower weir onset energy: iced={q_weir_iced}"
    );
}

#[test]
fn internal_bridge_cuts_build_opening_friction_segments() {
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
    let sections = BridgeSectionContext {
        xs_up: Some(bu),
        internal_xs: vec![internal],
        xs_down: Some(bd),
        friction_length_m: 100.0,
        ..Default::default()
    };
    let geom = build_bridge_geometry(
        4.0,
        6.0,
        0.0,
        0,
        0,
        1.44,
        0.8,
        0.0,
        0.0,
        UnitSystem::Metric,
        &BridgeCouplingParams::default(),
        100.0,
        None,
        Some(&sections),
    );
    assert_eq!(geom.internal_opening_tables.len(), 1);
    assert_eq!(geom.internal_opening_segment_lengths_m.len(), 2);
    assert_eq!(geom.internal_opening_z_m.len(), 1);
}

#[test]
fn test_deck_obstructed_area_subtracted_in_obstructed_hydraulics() {
    use crate::solvers::bridge::opening::{obstructed_hydraulics, deck_obstructed_area_at_wsel, deck_obstructed_width_at_wsel};
    let table = rectangular_table(10.0, 0.0, 50); // 10m wide rectangular channel, bed at 0.0
    let deck = BridgeDeckProfile {
        stations_m: vec![0.0, 10.0],
        low_elevations_m: vec![5.0, 5.0],
        high_elevations_m: vec![7.0, 7.0],
    };
    let geom = build_bridge_geometry(
        5.0, 7.0, 0.0, 0, 0, 1.44, 0.8, 0.0, 0.0, UnitSystem::Metric,
        &BridgeCouplingParams::default(), 50.0, Some(&deck), None,
    );
    
    // 1. Water surface is below deck: wsel = 4.0
    let wsel_low = 4.0;
    let props_low = obstructed_hydraulics(&table, wsel_low, 0.0, &geom, true);
    // Unobstructed base area: 10 * 4.0 = 40.0
    assert!((props_low.a_eff - 40.0).abs() < 1e-4);
    assert!((props_low.top_width - 10.0).abs() < 1e-4);
    assert_eq!(deck_obstructed_area_at_wsel(&geom, wsel_low), 0.0);
    assert_eq!(deck_obstructed_width_at_wsel(&geom, wsel_low), 0.0);

    // 2. Water surface is within deck: wsel = 6.0
    let wsel_mid = 6.0;
    let props_mid = obstructed_hydraulics(&table, wsel_mid, 0.0, &geom, true);
    // Base area: 10 * 6.0 = 60.0. Blocked deck: 10 * (6.0 - 5.0) = 10.0. Expected: 50.0
    assert!((props_mid.a_eff - 50.0).abs() < 1e-4);
    // Top width base: 10.0. Blocked: 10.0. Expected: clamped to 1e-3
    assert!((props_mid.top_width - 1e-3).abs() < 1e-4);
    assert_eq!(deck_obstructed_area_at_wsel(&geom, wsel_mid), 10.0);
    assert_eq!(deck_obstructed_width_at_wsel(&geom, wsel_mid), 10.0);

    // 3. Water surface is above deck: wsel = 8.0
    let wsel_high = 8.0;
    let props_high = obstructed_hydraulics(&table, wsel_high, 0.0, &geom, true);
    // Base area: 10 * 8.0 = 80.0. Blocked deck: 10 * (7.0 - 5.0) = 20.0. Expected: 60.0
    assert!((props_high.a_eff - 60.0).abs() < 1e-4);
    // Top width base: 10.0. Blocked: 0.0 (since wsel > high_chord). Expected: 10.0
    assert!((props_high.top_width - 10.0).abs() < 1e-4);
    assert_eq!(deck_obstructed_area_at_wsel(&geom, wsel_high), 20.0);
    assert_eq!(deck_obstructed_width_at_wsel(&geom, wsel_high), 0.0);
}
