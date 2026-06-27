use crate::geometry::GeometryTable;

use super::geometry::BridgeGeometry;
use super::high_flow::solve_high_flow;
use super::low_flow::solve_low_flow;
use super::types::BridgeHeadwaterSolve;

pub(crate) fn solve_bridge_headwater_metric(
    q_metric: f64,
    tw_clamped: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> BridgeHeadwaterSolve {
    if tw_clamped < geom.low_chord_m {
        solve_low_flow(q_metric, tw_clamped, geom, table_up, table_down)
    } else {
        solve_high_flow(q_metric, geom, tw_clamped, table_up, table_down)
    }
}

pub(crate) fn solve_low_flow_tailwater(
    q_metric: f64,
    hw_m: f64,
    geom: &BridgeGeometry,
    table_up: &GeometryTable,
    table_down: &GeometryTable,
) -> f64 {
    // Invert subcritical low-flow solvers via bisection on tailwater.
    let mut low = geom.z_down_m + 1e-4;
    let mut high = hw_m.min(geom.low_chord_m);
    let mut best = low;
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let hw_calc =
            solve_bridge_headwater_metric(q_metric, mid, geom, table_up, table_down).wsel_m;
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
