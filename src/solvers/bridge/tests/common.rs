//! Shared bridge test fixtures and helpers.

use super::*;
use crate::geometry::{CrossSection, GuideBankToe, GuideBanks, IneffectiveFlowAreas, row_at_elevation};
use crate::solvers::deck_vent_geometry::DeckVentUserInput;
use crate::solvers::pier_geometry::{
    resolve_pier_width_specs, PierAttachmentsUserInput, PierWidthSpec, PierWidthUserInput,
    ResolvedPier,
};


pub(crate) fn compound_overbank_approach(ineffective: bool) -> CrossSection {
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

pub(crate) fn approach_sections_with_ineffective_overbank() -> BridgeSectionContext {
    BridgeSectionContext {
        friction_length_m: 50.0,
        xs_approach: Some(compound_overbank_approach(true)),
        ..Default::default()
    }
}

pub(crate) fn approach_sections_with_guide_banks(
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

pub(crate) fn rectangular_table(width: f64, z_bed: f64, num_slices: usize) -> GeometryTable {
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

pub(crate) fn momentum_pier_geometry(shape: PierShape) -> BridgeGeometry {
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
    }
}

pub(crate) fn momentum_drag_for_shape(shape: PierShape) -> f64 {
    let table = rectangular_table(10.0, 0.0, 50);
    let geom = momentum_pier_geometry(shape);
    pier_drag_momentum_with_table(&table, 15.0, 3.0, geom.z_up_m, &geom, true)
}

pub(crate) fn class_b_friction_sections() -> BridgeSectionContext {
    let approach = CrossSection {
        station: 70.0,
        x: vec![0.0, 0.0, 20.0, 20.0],
        y: vec![5.0, 0.0, 0.0, 5.0],
        n_stations: vec![0.0],
        n_values: vec![0.03],
        unit_system: UnitSystem::Metric,
        is_overbank: None,
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
        blocked_obstructions: None,
        ineffective_flow_areas: None,
        guide_banks: None,
    };
    BridgeSectionContext {
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
    }
}

/// Class B with energy method (search uses opening-only geometry; sections added in test).
pub(crate) fn class_b_energy_case() -> (f64, f64, f64, i32, GeometryTable, GeometryTable) {
    let coupling = BridgeCouplingParams {
        low_flow_method: 3,
        ..Default::default()
    };
    for &width in &[5.0, 6.0, 7.0, 8.0, 10.0] {
        let table_up = rectangular_table(width, 0.0, 50);
        let table_down = rectangular_table(width, 0.0, 50);
        for &(pier_w, num_piers) in &[(1.5, 2), (2.0, 2), (2.0, 3), (2.5, 2)] {
            let geom = build_bridge_geometry(
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
                &coupling,
                15.0,
                None,
                None,
            );
            let mut q = 10.0;
            while q <= 65.0 {
                let mut tw = 0.85;
                while tw <= 2.5 {
                    if classify_low_flow(q, tw, &geom, &table_up, &table_down) == LowFlowClass::B {
                        return (q, tw, pier_w, num_piers, table_up, table_down);
                    }
                    tw += 0.05;
                }
                q += 1.0;
            }
        }
    }
    panic!("no Class B case found for energy friction test");
}

pub(crate) fn abutment_coupling(left_w: f64, right_w: f64, left_top: f64, right_top: f64) -> BridgeCouplingParams {
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

pub(crate) fn symmetric_rating_bridge_params() -> BridgeSolveParams {
    BridgeSolveParams {
        low_chord: 5.0,
        high_chord: 7.0,
        z_down: 0.0,
        z_up: 0.0,
        tw_wsel: 2.5,
        low_flow_method: 3,
        channel_width: 10.0,
        manning_n: 0.03,
        ..Default::default()
    }
}

pub(crate) fn hand_rectangular_a_eff(
    channel_width_m: f64,
    wsel_m: f64,
    z_bed_m: f64,
    geom: &BridgeGeometry,
) -> f64 {
    let a_base = channel_width_m * (wsel_m - z_bed_m).max(0.0);
    (a_base - geom.abutments.submerged_area_m2(wsel_m, z_bed_m)).max(1e-5)
}

/// Taper top=1 / bottom=2 vs constant prism at mean width 1.5 (same pier height 0â†’4 m).
pub(crate) fn tapered_vs_mean_constant_pier_geometries() -> (BridgeGeometry, BridgeGeometry) {
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

pub(crate) fn tapered_vs_rectangular_pier_geometries() -> (BridgeGeometry, BridgeGeometry) {
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

pub(crate) fn profile_pier_geometry() -> BridgeGeometry {
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

pub(crate) fn pier_with_footing_geometry() -> BridgeGeometry {
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

pub(crate) fn pier_with_nosing_geometry() -> BridgeGeometry {
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

pub(crate) fn pier_shaft_only_sections() -> BridgeSectionContext {
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

pub(crate) fn pier_footing_nosing_sections() -> BridgeSectionContext {
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
