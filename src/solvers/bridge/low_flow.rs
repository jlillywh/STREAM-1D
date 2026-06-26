use crate::geometry::GeometryTable;
use crate::utils::G_METRIC;

use super::geometry::{
    apply_opening_blockage, effective_z_bed_m, profile_opening_area_factor, scale_base_area_for_ice,
    BridgeGeometry,
};
use super::opening::{
    active_resolved_piers, approach_departure_cut_modifiers_active, bridge_energy_friction_loss,
    gross_projected_opening_width_m, ineffective_for_side, lookup_row, obstructed_hydraulics,
    obstructed_opening_at_wsel, pier_floating_debris_obstruction_m2, pier_submerged_area_at_wsel,
    reach_cut_flow_area, section_xs, specific_force, velocity_head, wspro_contraction_loss,
    yarnell_downstream_flow_area_m2,
};
use super::high_flow::solve_high_flow;
use super::section::BridgeFrictionWeighting;
use super::opening::base_flow_area;
use super::types::{BridgeHeadwaterSolve, LowFlowClass, LowFlowMethod, PierShape};

pub(crate) fn solve_low_flow_energy_or_wspro(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
    use_wspro: bool,
    subtract_deck: bool,
) -> f64 {
    let props_down = obstructed_hydraulics(table_down, tw_m, geom.z_down_m, geom, false, subtract_deck);
    if props_down.a_eff < 1e-6 {
        return tw_m;
    }
    let e_down = tw_m + velocity_head(q_metric, props_down.a_eff);

    let residual = |wsel_up: f64| -> f64 {
        let props_up = obstructed_hydraulics(table_up, wsel_up, geom.z_up_m, geom, true, subtract_deck);
        if props_up.a_eff < 1e-6 {
            return 1e6;
        }
        let e_up = wsel_up + velocity_head(q_metric, props_up.a_eff);
        let hf = bridge_energy_friction_loss(q_metric, wsel_up, tw_m, geom, table_up, table_down);
        let h_other = if use_wspro {
            let opening_wsel = wsel_up.min(tw_m).min(geom.low_chord_m);
            let (props_opening, _) =
                obstructed_opening_at_wsel(geom, table_up, table_down, opening_wsel);
            let a_approach = reach_cut_flow_area(geom, true, wsel_up).unwrap_or(props_up.a_eff);
            wspro_contraction_loss(
                q_metric,
                a_approach,
                props_opening.a_eff.max(1e-5),
                geom.wspro_coeff_c,
            )
            .max(0.0)
        } else if approach_departure_cut_modifiers_active(geom, true)
            || approach_departure_cut_modifiers_active(geom, false)
        {
            let opening_wsel = wsel_up.min(tw_m).min(geom.low_chord_m);
            let (props_opening, _) =
                obstructed_opening_at_wsel(geom, table_up, table_down, opening_wsel);
            let a_bridge = props_opening.a_eff.max(1e-5);
            let a_approach = reach_cut_flow_area(geom, true, wsel_up).unwrap_or(props_up.a_eff);
            let a_departure =
                reach_cut_flow_area(geom, false, tw_m).unwrap_or(props_down.a_eff);
            let hv_approach = velocity_head(q_metric, a_approach);
            let hv_bridge = velocity_head(q_metric, a_bridge);
            let hv_departure = velocity_head(q_metric, a_departure);
            let h_contract = if hv_approach > hv_bridge {
                geom.coeff_contraction * (hv_approach - hv_bridge)
            } else {
                0.0
            };
            let h_expand = if hv_bridge > hv_departure {
                geom.coeff_expansion * (hv_bridge - hv_departure)
            } else {
                0.0
            };
            h_contract + h_expand
        } else {
            let hv_up = velocity_head(q_metric, props_up.a_eff);
            let hv_down = velocity_head(q_metric, props_down.a_eff);
            if hv_up > hv_down {
                geom.coeff_contraction * (hv_up - hv_down)
            } else {
                geom.coeff_expansion * (hv_down - hv_up)
            }
        };
        e_up - e_down - hf - h_other
    };

    let mut low = tw_m;
    let mut high = geom.low_chord_m;
    let mut best = low;
    let res_low = residual(low);
    let mut res_high = residual(high);
    if res_low * res_high > 0.0 {
        high = geom.low_chord_m + 20.0;
        res_high = residual(high);
    }
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let res = residual(mid);
        if res.abs() < 1e-6 {
            return mid;
        }
        if res_low * res_high <= 0.0 {
            if res < 0.0 {
                low = mid;
            } else {
                high = mid;
            }
        } else if res < 0.0 {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

pub(crate) fn auto_class_a_method(geom: &BridgeGeometry) -> LowFlowMethod {
    if geom.num_piers > 0 {
        LowFlowMethod::Yarnell
    } else if geom.abutments.is_configured() {
        LowFlowMethod::Wspro
    } else {
        LowFlowMethod::Energy
    }
}

pub(crate) fn pier_drag_momentum_with_table(
    table: &GeometryTable,
    q: f64,
    wsel: f64,
    z_bed: f64,
    geom: &BridgeGeometry,
    is_upstream: bool,
) -> f64 {
    let depth = (wsel - z_bed).max(0.0);
    let a_pier = pier_submerged_area_at_wsel(geom, wsel, z_bed);
    if a_pier <= 1e-6 {
        return 0.0;
    }
    let props = obstructed_hydraulics(table, wsel, z_bed, geom, is_upstream, false);
    let v = q / props.a_eff.max(1e-5);
    let y_pier = depth * 0.5;
    let cd = geom.pier_shape.drag_coefficient();
    a_pier * y_pier + 0.5 * cd * a_pier * (v * v) / G_METRIC
}

pub(crate) fn solve_critical_depth_obstructed(
    table: &GeometryTable,
    z_bed: f64,
    q: f64,
    geom: &BridgeGeometry,
    is_upstream: bool,
) -> f64 {
    if table.rows.is_empty() || q <= 1e-5 {
        return 0.0;
    }
    let y_min = table.rows[0].elevation;
    let y_max = table.rows[table.rows.len() - 1].elevation;
    let mut low = 0.0;
    let mut high = (y_max - y_min).max(10.0);
    let mut best_yc = 0.0;

    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let elev = y_min + mid;
        let props = obstructed_hydraulics(table, elev, z_bed, geom, is_upstream, false);
        if props.a_eff < 1e-6 {
            low = mid;
            continue;
        }
        let fr_sq = (q * q * props.top_width) / (G_METRIC * props.a_eff.powi(3));
        let f_val = 1.0 - fr_sq;
        if f_val.abs() < 1e-6 {
            best_yc = mid;
            break;
        }
        if f_val < 0.0 {
            low = mid;
        } else {
            high = mid;
        }
        best_yc = mid;
    }
    best_yc
}

pub(crate) fn critical_specific_force(
    table: &GeometryTable,
    z_bed: f64,
    q: f64,
    geom: &BridgeGeometry,
    is_upstream: bool,
) -> (f64, f64) {
    let yc = solve_critical_depth_obstructed(table, z_bed, q, geom, is_upstream);
    let wsel_crit = z_bed + yc;
    (wsel_crit, specific_force(table, wsel_crit, z_bed, q, geom, is_upstream))
}

/// Classify low flow per HEC-RAS: compare downstream momentum to critical momentum in the bridge.
pub fn classify_low_flow(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> LowFlowClass {
    let (_, m_crit_up) = critical_specific_force(table_up, geom.z_up_m, q_metric, geom, true);
    let (_, m_crit_down) = critical_specific_force(table_down, geom.z_down_m, q_metric, geom, false);
    let (wsel_crit, m_crit) = if m_crit_up >= m_crit_down {
        (
            geom.z_up_m + solve_critical_depth_obstructed(table_up, geom.z_up_m, q_metric, geom, true),
            m_crit_up,
        )
    } else {
        (
            geom.z_down_m
                + solve_critical_depth_obstructed(table_down, geom.z_down_m, q_metric, geom, false),
            m_crit_down,
        )
    };

    let m_down = specific_force(table_down, tw_m, geom.z_down_m, q_metric, geom, false);
    let props = obstructed_hydraulics(table_down, tw_m, geom.z_down_m, geom, false, false);
    let v_down = if props.a_eff > 1e-5 {
        q_metric / props.a_eff
    } else {
        0.0
    };
    let fr_down = if props.a_eff > 1e-5 {
        v_down / (G_METRIC * props.a_eff / props.top_width.max(1e-3)).sqrt()
    } else {
        0.0
    };

    let _ = wsel_crit;

    if fr_down >= 1.0 && tw_m < geom.low_chord_m {
        LowFlowClass::C
    } else if m_down < m_crit {
        LowFlowClass::B
    } else {
        LowFlowClass::A
    }
}

/// HEC-RAS Yarnell low-flow pier head loss (Class A): drop from section 3 to section 2.
pub fn yarnell_pier_head_loss(
    q_metric: f64,
    wsel_down_metric: f64,
    z_bed_down_metric: f64,
    pier_width_m: f64,
    num_piers: i32,
    pier_shape: PierShape,
    flow_area_m2: f64,
) -> f64 {
    if q_metric <= 1e-5 || flow_area_m2 <= 1e-5 {
        return 0.0;
    }

    let depth_down = (wsel_down_metric - z_bed_down_metric).max(0.0);
    if depth_down <= 1e-5 {
        return 0.0;
    }

    let a_piers = (num_piers as f64) * pier_width_m * depth_down;
    yarnell_pier_head_loss_from_area(
        q_metric,
        wsel_down_metric,
        z_bed_down_metric,
        a_piers,
        flow_area_m2,
        pier_shape,
    )
}

pub(crate) fn yarnell_pier_head_loss_integrated(
    q_metric: f64,
    wsel_down_metric: f64,
    z_bed_down_metric: f64,
    geom: &BridgeGeometry,
    flow_area_m2: f64,
) -> f64 {
    if q_metric <= 1e-5 || flow_area_m2 <= 1e-5 {
        return 0.0;
    }
    let a_piers = pier_submerged_area_at_wsel(geom, wsel_down_metric, z_bed_down_metric);
    yarnell_pier_head_loss_from_area(
        q_metric,
        wsel_down_metric,
        z_bed_down_metric,
        a_piers,
        flow_area_m2,
        geom.pier_shape,
    )
}

pub(crate) fn yarnell_pier_head_loss_from_area(
    q_metric: f64,
    wsel_down_metric: f64,
    z_bed_down_metric: f64,
    a_piers: f64,
    flow_area_m2: f64,
    pier_shape: PierShape,
) -> f64 {
    let depth_down = (wsel_down_metric - z_bed_down_metric).max(0.0);
    if depth_down <= 1e-5 || a_piers <= 1e-6 {
        return 0.0;
    }

    let a_unobstructed = (flow_area_m2 - a_piers).max(1e-5);
    let a_piers_clamped = a_piers.min(a_unobstructed * 0.9);
    let alpha = a_piers_clamped / a_unobstructed;

    let v_ds = q_metric / flow_area_m2;
    let velocity_head = (v_ds * v_ds) / (2.0 * G_METRIC);
    let omega = velocity_head / depth_down;
    let k = pier_shape.yarnell_coefficient();

    2.0 * k * (k + 10.0 * omega - 0.6) * (alpha + 15.0 * alpha.powi(4)) * velocity_head
}

pub(crate) fn gross_opening_area_at_low_chord(
    geom: &BridgeGeometry,
    table: &GeometryTable,
    z_bed: f64,
    is_upstream: bool,
) -> f64 {
    let wsel = geom.low_chord_m;
    let z_eff = effective_z_bed_m(z_bed, geom);
    let height_under_deck = (wsel - z_eff).max(0.0);
    let deck_width = gross_projected_opening_width_m(geom);
    let a_gross = if deck_width > 1e-6 {
        deck_width * height_under_deck
    } else {
        let ineffective = ineffective_for_side(geom, is_upstream);
        let row = lookup_row(
            table,
            section_xs(geom, is_upstream),
            ineffective,
            None,
            wsel,
        );
        scale_base_area_for_ice(base_flow_area(&row, ineffective, None), wsel, z_bed, geom)
    };
    let a_piers = pier_submerged_area_at_wsel(geom, wsel, z_bed);
    let a_abut = geom.abutments.submerged_area_m2(wsel, z_eff);
    let a_debris = pier_floating_debris_obstruction_m2(geom, wsel, z_bed);
    apply_opening_blockage(
        (a_gross - a_piers - a_abut - a_debris).max(1e-4),
        geom,
    )
}

/// Net opening area at the low chord using HEC-RAS min(BU, BD) weighting.
pub(crate) fn net_opening_area_at_low_chord(
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let factor = profile_opening_area_factor(geom);
    let wsel = geom.low_chord_m;
    if geom.xs_up.is_some() || geom.xs_down.is_some() {
        let (props, _) = obstructed_opening_at_wsel(geom, table_up, table_down, wsel);
        return props.a_eff.max(1e-4) * factor;
    }
    let a_up = gross_opening_area_at_low_chord(geom, table_up, geom.z_up_m, true);
    let a_down = gross_opening_area_at_low_chord(geom, table_down, geom.z_down_m, false);
    a_up.min(a_down) * factor
}

pub(crate) fn upstream_energy_grade(
    wsel: f64,
    q_metric: f64,
    geom: &BridgeGeometry,
    table: &GeometryTable,
    z_bed: f64,
    is_upstream: bool,
) -> f64 {
    let props = obstructed_hydraulics(table, wsel, z_bed, geom, is_upstream, false);
    wsel + velocity_head(q_metric, props.a_eff)
}

pub(crate) fn high_flow_energy_uses_wspro(geom: &BridgeGeometry) -> bool {
    matches!(geom.low_flow_method, LowFlowMethod::Wspro)
        || (geom.low_flow_method == LowFlowMethod::Auto && geom.abutments.is_configured())
}

/// Energy balance through the obstructed opening (HEC-RAS high-flow energy method).
pub(crate) fn solve_high_flow_energy(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    solve_low_flow_energy_or_wspro(
        q_metric,
        tw_m,
        geom,
        table_up,
        table_down,
        high_flow_energy_uses_wspro(geom),
        true,
    )
}

pub(crate) fn solve_high_flow_energy_fallback(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    solve_high_flow_energy(q_metric, tw_m, geom, table_up, table_down)
}

pub(crate) fn solve_low_flow_class_a(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let method = match geom.low_flow_method {
        LowFlowMethod::Auto => auto_class_a_method(geom),
        other => other,
    };

    match method {
        LowFlowMethod::Energy => {
            return solve_low_flow_energy_or_wspro(
                q_metric, tw_m, geom, table_up, table_down, false, false,
            );
        }
        LowFlowMethod::Wspro => {
            return solve_low_flow_energy_or_wspro(
                q_metric, tw_m, geom, table_up, table_down, true, false,
            );
        }
        LowFlowMethod::Momentum => {}
        LowFlowMethod::Yarnell | LowFlowMethod::Auto => {}
    }

    let use_yarnell =
        matches!(method, LowFlowMethod::Yarnell) && !active_resolved_piers(geom).is_empty();

    if use_yarnell {
        let flow_area_net = yarnell_downstream_flow_area_m2(table_down, tw_m, geom.z_down_m, geom);

        if flow_area_net > 1e-5 && q_metric > 1e-5 {
            let hl = yarnell_pier_head_loss_integrated(
                q_metric,
                tw_m,
                geom.z_down_m,
                geom,
                flow_area_net,
            );
            return tw_m + hl;
        }
    }

    // Momentum (general) Class A: upstream specific force = downstream + pier drag
    let m_down = specific_force(table_down, tw_m, geom.z_down_m, q_metric, geom, false);
    let drag = pier_drag_momentum_with_table(table_up, q_metric, tw_m, geom.z_up_m, geom, true);
    let target = m_down + drag;

    let mut low = tw_m;
    let mut high = geom.low_chord_m;
    let mut best = low;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let m_up = specific_force(table_up, mid, geom.z_up_m, q_metric, geom, true);
        if (m_up - target).abs() < 1e-6 {
            return mid;
        }
        if m_up < target {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

pub(crate) fn solve_low_flow_class_b(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let use_energy = matches!(
        geom.low_flow_method,
        LowFlowMethod::Energy | LowFlowMethod::Wspro
    ) || geom.friction_weighting == BridgeFrictionWeighting::HecRasSegments;

    if !use_energy {
        let (_, m_crit_up) = critical_specific_force(table_up, geom.z_up_m, q_metric, geom, true);
        let (_, m_crit_down) = critical_specific_force(table_down, geom.z_down_m, q_metric, geom, false);
        let m_crit = m_crit_up.max(m_crit_down);
        let z_control = if m_crit_up >= m_crit_down {
            geom.z_up_m
        } else {
            geom.z_down_m
        };
        let table_control = if m_crit_up >= m_crit_down {
            table_up
        } else {
            table_down
        };
        let is_upstream = m_crit_up >= m_crit_down;
        let yc = solve_critical_depth_obstructed(table_control, z_control, q_metric, geom, is_upstream);
        let wsel_min = z_control + yc;

        let mut low = wsel_min;
        let mut high = geom.low_chord_m;
        let mut best = low;
        for _ in 0..50 {
            let mid = 0.5 * (low + high);
            let drag = pier_drag_momentum_with_table(table_up, q_metric, mid, geom.z_up_m, geom, true);
            let m_up = specific_force(table_up, mid, geom.z_up_m, q_metric, geom, true);
            let residual = m_up - drag - m_crit;
            if residual.abs() < 1e-5 {
                return mid;
            }
            if residual < 0.0 {
                low = mid;
            } else {
                high = mid;
            }
            best = mid;
        }
        if best > tw_m {
            return best;
        }
    }

    // HEC-RAS Class B energy fallback when momentum fails or energy/WSPRO is selected.
    let use_wspro = matches!(geom.low_flow_method, LowFlowMethod::Wspro)
        || (geom.low_flow_method == LowFlowMethod::Auto && geom.abutments.is_configured());
    solve_low_flow_energy_or_wspro(q_metric, tw_m, geom, table_up, table_down, use_wspro, false)
}

pub(crate) fn solve_low_flow_class_c(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    let m_down = specific_force(table_down, tw_m, geom.z_down_m, q_metric, geom, false);
    let mut low = tw_m;
    let mut high = geom.low_chord_m;
    let mut best = high;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let drag = pier_drag_momentum_with_table(table_up, q_metric, mid, geom.z_up_m, geom, true);
        let m_up = specific_force(table_up, mid, geom.z_up_m, q_metric, geom, true);
        let residual = (m_up - drag) - m_down;
        if residual.abs() < 1e-5 {
            return mid;
        }
        if residual < 0.0 {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

pub(crate) fn solve_low_flow(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> BridgeHeadwaterSolve {
    let class = classify_low_flow(q_metric, tw_m, geom, table_up, table_down);
    let wsel_up = match class {
        LowFlowClass::A => solve_low_flow_class_a(q_metric, tw_m, geom, table_up, table_down),
        LowFlowClass::B => solve_low_flow_class_b(q_metric, tw_m, geom, table_up, table_down),
        LowFlowClass::C => solve_low_flow_class_c(q_metric, tw_m, geom, table_up, table_down),
    };
    reconcile_low_flow_with_high_flow(
        q_metric, tw_m, wsel_up, class, geom, table_up, table_down,
    )
}

pub(crate) fn reconcile_low_flow_with_high_flow(
    q_metric: f64,
    tw_m: f64,
    wsel_low: f64,
    low_class: LowFlowClass,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> BridgeHeadwaterSolve {
    let egl = upstream_energy_grade(wsel_low, q_metric, geom, table_up, geom.z_up_m, true);
    if egl <= geom.low_chord_max_m {
        return BridgeHeadwaterSolve {
            wsel_m: wsel_low,
            regime: low_class.flow_regime(),
        };
    }
    let high = solve_high_flow(q_metric, geom, tw_m, table_up, table_down);
    let wsel = wsel_low.max(high.wsel_m);
    // EGL exceeded the deck; high-flow regime governs even when HW ties low-flow.
    BridgeHeadwaterSolve {
        wsel_m: wsel,
        regime: high.regime,
    }
}
