//! Low-flow bridge headwater residual for implicit Preissmann coupling (Phase 4).

use crate::geometry::GeometryTable;

use super::geometry::BridgeGeometry;
use super::low_flow::{
    classify_low_flow, solve_low_flow_class_a, solve_low_flow_class_b, solve_low_flow_class_c,
    upstream_energy_grade,
};
use super::types::LowFlowClass;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BridgeHeadwaterImplicitResidual {
    pub r: f64,
    /// ∂R/∂y on the headwater face (metric).
    pub dr_dy_hw: f64,
    /// ∂R/∂y on the tailwater face (metric).
    pub dr_dy_tw: f64,
    /// ∂R/∂Q (metric); held at zero in the Jacobian row today.
    pub dr_dq: f64,
    pub pinned_class: LowFlowClass,
}

/// B1: classify once per residual evaluation; caller pins `pinned` across the Preissmann step.
pub(crate) fn bridge_implicit_pin_class(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_hyd_us: &GeometryTable,
    table_hyd_ds: &GeometryTable,
) -> Option<LowFlowClass> {
    if q_metric <= 1e-6 {
        return None;
    }
    let tw_clamped = tw_m.max(geom.z_down_m + 1e-4);
    if tw_clamped >= geom.low_chord_m {
        return None;
    }
    let class = classify_low_flow(q_metric, tw_clamped, geom, table_hyd_us, table_hyd_ds);
    if matches!(class, LowFlowClass::C) {
        return None;
    }
    Some(class)
}

pub(crate) fn solve_low_flow_headwater_pinned(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_hyd_us: &GeometryTable,
    table_hyd_ds: &GeometryTable,
    pinned: LowFlowClass,
) -> f64 {
    let tw_clamped = tw_m.max(geom.z_down_m + 1e-4);
    match pinned {
        LowFlowClass::A => {
            solve_low_flow_class_a(q_metric, tw_clamped, geom, table_hyd_us, table_hyd_ds)
        }
        LowFlowClass::B => {
            solve_low_flow_class_b(q_metric, tw_clamped, geom, table_hyd_us, table_hyd_ds)
        }
        LowFlowClass::C => {
            solve_low_flow_class_c(q_metric, tw_clamped, geom, table_hyd_us, table_hyd_ds)
        }
    }
}

fn solve_low_flow_tailwater_pinned(
    q_metric: f64,
    hw_m: f64,
    geom: &BridgeGeometry,
    table_hyd_us: &GeometryTable,
    table_hyd_ds: &GeometryTable,
    pinned: LowFlowClass,
) -> f64 {
    let mut low = geom.z_down_m + 1e-4;
    let mut high = hw_m.min(geom.low_chord_m);
    let mut best = low;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let hw_calc = solve_low_flow_headwater_pinned(
            q_metric,
            mid,
            geom,
            table_hyd_us,
            table_hyd_ds,
            pinned,
        );
        if (hw_calc - hw_m).abs() < 1e-4 {
            return mid;
        }
        if hw_calc < hw_m {
            low = mid;
        } else {
            high = mid;
        }
        best = mid;
    }
    best
}

fn hw_escalates_to_high_flow(
    q_metric: f64,
    hw_m: f64,
    geom: &BridgeGeometry,
    table_hyd_us: &GeometryTable,
) -> bool {
    let egl = upstream_energy_grade(hw_m, q_metric, geom, table_hyd_us, geom.z_up_m, true);
    egl > geom.low_chord_max_m
}

fn centered_hw_derivatives(
    q_metric: f64,
    tw_m: f64,
    geom: &BridgeGeometry,
    table_hyd_us: &GeometryTable,
    table_hyd_ds: &GeometryTable,
    pinned: LowFlowClass,
) -> (f64, f64) {
    let dy = (tw_m.abs() * 1e-4).max(0.003);
    let hw_lo = solve_low_flow_headwater_pinned(
        q_metric,
        (tw_m - dy).max(geom.z_down_m + 1e-4),
        geom,
        table_hyd_us,
        table_hyd_ds,
        pinned,
    );
    let hw_hi = solve_low_flow_headwater_pinned(
        q_metric,
        tw_m + dy,
        geom,
        table_hyd_us,
        table_hyd_ds,
        pinned,
    );
    let dr_dy_tw = -0.5 * (hw_hi - hw_lo) / dy;

    let dq = (q_metric * 1e-4).max(1e-4);
    let hw_qlo = solve_low_flow_headwater_pinned(
        (q_metric - dq).max(1e-6),
        tw_m,
        geom,
        table_hyd_us,
        table_hyd_ds,
        pinned,
    );
    let hw_qhi = solve_low_flow_headwater_pinned(
        q_metric + dq,
        tw_m,
        geom,
        table_hyd_us,
        table_hyd_ds,
        pinned,
    );
    let dr_dq = -0.5 * (hw_qhi - hw_qlo) / dq;
    (dr_dy_tw, dr_dq)
}

/// Headwater residual: R = y_hw − HW_low(y_tw, Q) with pinned low-flow class.
pub(crate) fn bridge_headwater_implicit_rhs(
    y_hw_metric: f64,
    y_tw_metric: f64,
    q_metric: f64,
    pinned: LowFlowClass,
    geom: &BridgeGeometry,
    table_hyd_us: &GeometryTable,
    table_hyd_ds: &GeometryTable,
) -> Option<BridgeHeadwaterImplicitResidual> {
    if matches!(pinned, LowFlowClass::C) {
        return None;
    }
    if q_metric <= 1e-6 {
        return None;
    }
    let tw_clamped = y_tw_metric.max(geom.z_down_m + 1e-4);
    if tw_clamped >= geom.low_chord_m {
        return None;
    }

    let hw_calc = solve_low_flow_headwater_pinned(
        q_metric,
        tw_clamped,
        geom,
        table_hyd_us,
        table_hyd_ds,
        pinned,
    );
    if hw_escalates_to_high_flow(q_metric, hw_calc, geom, table_hyd_us) {
        return None;
    }

    let r = y_hw_metric - hw_calc;
    let (dr_dy_tw, dr_dq) = centered_hw_derivatives(
        q_metric,
        tw_clamped,
        geom,
        table_hyd_us,
        table_hyd_ds,
        pinned,
    );

    Some(BridgeHeadwaterImplicitResidual {
        r,
        dr_dy_hw: 1.0,
        dr_dy_tw,
        dr_dq,
        pinned_class: pinned,
    })
}

/// Tailwater residual for reverse-flow BC inversion: R = y_tw − TW_low(y_hw, Q).
#[allow(dead_code)]
pub(crate) fn bridge_tailwater_implicit_rhs(
    y_hw_metric: f64,
    y_tw_metric: f64,
    q_metric: f64,
    pinned: LowFlowClass,
    geom: &BridgeGeometry,
    table_hyd_us: &GeometryTable,
    table_hyd_ds: &GeometryTable,
) -> Option<BridgeHeadwaterImplicitResidual> {
    if matches!(pinned, LowFlowClass::C) {
        return None;
    }
    if q_metric <= 1e-6 {
        return None;
    }
    let hw_clamped = y_hw_metric.min(geom.low_chord_m);
    if hw_clamped >= geom.low_chord_m {
        return None;
    }

    let tw_calc = solve_low_flow_tailwater_pinned(
        q_metric,
        hw_clamped,
        geom,
        table_hyd_us,
        table_hyd_ds,
        pinned,
    );
    if tw_clamped_ge_low_chord(tw_calc, geom) {
        return None;
    }
    if hw_escalates_to_high_flow(q_metric, hw_clamped, geom, table_hyd_us) {
        return None;
    }

    let r = y_tw_metric - tw_calc;
    let dy = (hw_clamped * 1e-4).max(0.003);
    let tw_lo = solve_low_flow_tailwater_pinned(
        q_metric,
        (hw_clamped - dy).max(geom.z_up_m + 1e-4),
        geom,
        table_hyd_us,
        table_hyd_ds,
        pinned,
    );
    let tw_hi = solve_low_flow_tailwater_pinned(
        q_metric,
        (hw_clamped + dy).min(geom.low_chord_m),
        geom,
        table_hyd_us,
        table_hyd_ds,
        pinned,
    );
    let dr_dy_hw = -0.5 * (tw_hi - tw_lo) / dy;

    let dq = (q_metric * 1e-4).max(1e-4);
    let tw_qlo = solve_low_flow_tailwater_pinned(
        (q_metric - dq).max(1e-6),
        hw_clamped,
        geom,
        table_hyd_us,
        table_hyd_ds,
        pinned,
    );
    let tw_qhi = solve_low_flow_tailwater_pinned(
        q_metric + dq,
        hw_clamped,
        geom,
        table_hyd_us,
        table_hyd_ds,
        pinned,
    );
    let dr_dq = -0.5 * (tw_qhi - tw_qlo) / dq;

    Some(BridgeHeadwaterImplicitResidual {
        r,
        dr_dy_hw,
        dr_dy_tw: 1.0,
        dr_dq,
        pinned_class: pinned,
    })
}

#[allow(dead_code)]
fn tw_clamped_ge_low_chord(tw_m: f64, geom: &BridgeGeometry) -> bool {
    tw_m.max(geom.z_down_m + 1e-4) >= geom.low_chord_m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::GeometryTable;
    use crate::solvers::bridge::geometry::build_bridge_geometry;
    use crate::solvers::bridge::headwater::solve_bridge_headwater_metric;
    use crate::solvers::bridge::types::{BridgeCouplingParams, LowFlowMethod};
    use crate::utils::UnitSystem;

    fn metric_channel_table() -> GeometryTable {
        let xs = crate::geometry::CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            coeff_contraction: None,
            coeff_expansion: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        };
        xs.generate_lookup_table(50)
    }

    fn pier_geom(low_flow_method: i32) -> BridgeGeometry {
        let coupling = BridgeCouplingParams {
            low_flow_method,
            ..BridgeCouplingParams::default()
        };
        build_bridge_geometry(
            5.0,
            7.0,
            0.5,
            2,
            0,
            1.44,
            0.5,
            0.0,
            0.5,
            UnitSystem::Metric,
            &coupling,
            250.0,
            None,
            None,
        )
    }

    #[test]
    fn bridge_headwater_implicit_rhs_matches_headwater_solve() {
        let table = metric_channel_table();
        let geom = pier_geom(0);
        let q = 15.0;
        let tw = 1.5;
        let tw_m = tw;
        let pinned = bridge_implicit_pin_class(q, tw_m, &geom, &table, &table).expect("low_a");
        let solved = solve_bridge_headwater_metric(q, tw_m, &geom, &table, &table);
        assert_eq!(pinned, LowFlowClass::A);
        let residual =
            bridge_headwater_implicit_rhs(solved.wsel_m, tw_m, q, pinned, &geom, &table, &table)
                .expect("residual");
        assert!(residual.r.abs() < 0.01, "R={}", residual.r);
    }

    #[test]
    fn bridge_implicit_returns_none_above_low_chord() {
        let table = metric_channel_table();
        let geom = pier_geom(0);
        let pinned = LowFlowClass::A;
        assert!(
            bridge_headwater_implicit_rhs(6.0, 5.5, 15.0, pinned, &geom, &table, &table).is_none()
        );
        assert!(bridge_implicit_pin_class(15.0, 5.5, &geom, &table, &table).is_none());
    }

    #[test]
    fn bridge_headwater_implicit_rhs_energy_method() {
        let table = metric_channel_table();
        let geom = pier_geom(LowFlowMethod::Energy as i32);
        let q = 8.0;
        let tw_m = 1.2;
        let pinned = bridge_implicit_pin_class(q, tw_m, &geom, &table, &table).expect("class");
        let solved = solve_bridge_headwater_metric(q, tw_m, &geom, &table, &table);
        let residual =
            bridge_headwater_implicit_rhs(solved.wsel_m, tw_m, q, pinned, &geom, &table, &table)
                .expect("energy residual");
        assert!(residual.r.abs() < 0.02);
    }
}
