//! Unsteady reach boundary conditions (downstream types mirror steady `SteadyInputs`).

use crate::geometry::GeometryTable;
use crate::solvers::steady::{
    interpolate_rating_curve, solve_critical_depth_table, solve_normal_depth_table,
};
use crate::utils::{UnitSystem, CFS_TO_CMS, FT_TO_M};

use super::preissmann::{solve_preissmann_step, PreissmannStepParams, PreissmannStepStats};

const DS_BC_ITER_MAX: usize = 12;
const DS_BC_TOL_M: f64 = 1e-4;

/// Downstream boundary configuration for one time step.
pub(crate) struct DownstreamBcParams<'a> {
    pub bc_type: i32,
    pub slope: f64,
    pub known_wsel_metric: f64,
    pub rating_q: Option<&'a [f64]>,
    pub rating_wsel: Option<&'a [f64]>,
    pub ds_table: &'a GeometryTable,
    pub ds_z_min: f64,
    pub raw_units: UnitSystem,
}

fn clamp_ds_wsel(z_min: f64, wsel: f64) -> f64 {
    wsel.max(z_min + 0.05)
}

/// Stage at the downstream node for the given discharge (metric WSEL).
pub(crate) fn downstream_wsel_from_flow(params: &DownstreamBcParams<'_>, q_ds_metric: f64) -> f64 {
    match params.bc_type {
        1 => {
            let yc = solve_critical_depth_table(params.ds_table, q_ds_metric);
            clamp_ds_wsel(params.ds_z_min, params.ds_z_min + yc)
        }
        2 => {
            let slope = if params.slope <= 0.0 { 0.01 } else { params.slope };
            clamp_ds_wsel(
                params.ds_z_min,
                solve_normal_depth_table(params.ds_table, q_ds_metric, slope),
            )
        }
        3 => {
            let q_user = if params.raw_units == UnitSystem::USCustomary {
                q_ds_metric / CFS_TO_CMS
            } else {
                q_ds_metric
            };
            let wsel = interpolate_rating_curve(
                q_user,
                params.rating_q.unwrap_or(&[]),
                params.rating_wsel.unwrap_or(&[]),
            )
            .unwrap_or_else(|| {
                if params.raw_units == UnitSystem::USCustomary {
                    params.known_wsel_metric / FT_TO_M
                } else {
                    params.known_wsel_metric
                }
            });
            let wsel_m = if params.raw_units == UnitSystem::USCustomary {
                wsel * FT_TO_M
            } else {
                wsel
            };
            clamp_ds_wsel(params.ds_z_min, wsel_m)
        }
        _ => clamp_ds_wsel(params.ds_z_min, params.known_wsel_metric),
    }
}

/// Preissmann step with known-WSEL or dynamically coupled downstream BC (types 1–3).
pub(crate) fn solve_preissmann_with_downstream_bc(
    mut step: PreissmannStepParams<'_>,
    ds_bc: &DownstreamBcParams<'_>,
) -> Option<(Vec<f64>, Vec<f64>, PreissmannStepStats)> {
    if ds_bc.bc_type == 0 {
        step.y_down_next = downstream_wsel_from_flow(ds_bc, 0.0);
        return solve_preissmann_step(&step);
    }

    let q_guess = step.q_current.last().copied()?;
    let mut y_down = downstream_wsel_from_flow(ds_bc, q_guess);

    let mut last = None;
    for _ in 0..DS_BC_ITER_MAX {
        step.y_down_next = y_down;
        let (y_next, q_next, stats) = solve_preissmann_step(&step)?;
        let q_ds = q_next.last().copied()?;
        let y_new = downstream_wsel_from_flow(ds_bc, q_ds);
        last = Some((y_next, q_next, stats));
        if (y_new - y_down).abs() < DS_BC_TOL_M {
            break;
        }
        y_down = y_new;
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::CrossSection;

    fn metric_trap_table() -> GeometryTable {
        CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
            blocked_obstructions: None,
            ineffective_flow_areas: None,
            guide_banks: None,
        }
        .generate_lookup_table(50)
    }

    #[test]
    fn friction_slope_wsel_increases_with_discharge() {
        let table = metric_trap_table();
        let params = DownstreamBcParams {
            bc_type: 2,
            slope: 0.001,
            known_wsel_metric: 0.0,
            rating_q: None,
            rating_wsel: None,
            ds_table: &table,
            ds_z_min: 0.0,
            raw_units: UnitSystem::Metric,
        };
        let w_low = downstream_wsel_from_flow(&params, 5.0);
        let w_high = downstream_wsel_from_flow(&params, 20.0);
        assert!(w_high > w_low);
    }

    #[test]
    fn rating_curve_wsel_tracks_discharge() {
        let table = metric_trap_table();
        let rating_q = vec![5.0, 10.0, 20.0];
        let rating_wsel = vec![1.0, 1.2, 1.5];
        let params = DownstreamBcParams {
            bc_type: 3,
            slope: 0.001,
            known_wsel_metric: 0.0,
            rating_q: Some(&rating_q),
            rating_wsel: Some(&rating_wsel),
            ds_table: &table,
            ds_z_min: 0.0,
            raw_units: UnitSystem::Metric,
        };
        let w_low = downstream_wsel_from_flow(&params, 5.0);
        let w_high = downstream_wsel_from_flow(&params, 20.0);
        assert!(w_high > w_low);
    }
}
