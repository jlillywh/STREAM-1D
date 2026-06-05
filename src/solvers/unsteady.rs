use crate::utils::{G_METRIC, UnitSystem, FT_TO_M, Mat2, Vec2, solve_block_tridiagonal};
use crate::geometry::{CrossSection, GeometryTable};

/// Input parameters for the unsteady-state solver.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnsteadyInputs {
    /// Cross-sections defining the river reach.
    pub cross_sections: Vec<CrossSection>,
    /// Initial water surface elevations (WSEL) at each section (in user units).
    pub initial_wsel: Vec<f64>,
    /// Initial flow rates (Q) at each section (in user units).
    pub initial_q: Vec<f64>,
    /// Simulation time step size (in seconds).
    pub dt: f64,
    /// Number of time steps to run.
    pub num_steps: usize,
    /// Upstream flow hydrograph boundary condition (in user units, array of size num_steps).
    pub upstream_q_hydrograph: Vec<f64>,
    /// Downstream stage hydrograph boundary condition (in user units, array of size num_steps).
    pub downstream_wsel_hydrograph: Vec<f64>,
    /// Preissmann weighting factor theta (typically 0.55 to 0.7, default 0.6).
    pub theta: Option<f64>,
    /// Number of uniform vertical slices for geometry lookup tables (default 100).
    pub num_slices: Option<usize>,
}

/// Output results from the unsteady-state solver.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnsteadyResult {
    /// Time history of water surface elevations (WSEL) [step][section] (in user units).
    pub wsel: Vec<Vec<f64>>,
    /// Time history of flow rates (Q) [step][section] (in user units).
    pub q: Vec<Vec<f64>>,
    /// Time history of flow velocities [step][section] (in user units).
    pub velocity: Vec<Vec<f64>>,
}

/// Helper to compute numerical derivative of conveyance K with respect to elevation y.
fn compute_dk_dy(table: &GeometryTable, elev: f64) -> f64 {
    let dy = 0.01;
    let k_plus = table.interpolate(elev + dy).conveyance;
    let k_minus = table.interpolate(elev - dy).conveyance;
    (k_plus - k_minus) / (2.0 * dy)
}

/// Solves a single unsteady time step.
pub fn solve_unsteady_step(
    tables: &[GeometryTable],
    xs_list: &[CrossSection],
    y_current: &[f64], // current WSEL (metric)
    q_current: &[f64], // current Q (metric)
    dt: f64,
    q_up_next: f64,    // upstream flow BC at t+1 (metric)
    y_down_next: f64,  // downstream stage BC at t+1 (metric)
    theta: f64,
) -> Option<(Vec<f64>, Vec<f64>)> {
    let n = y_current.len();
    if n < 2 {
        return None;
    }

    // Allocate block tridiagonal matrices
    let mut a = vec![Mat2::zero(); n];
    let mut b = vec![Mat2::zero(); n];
    let mut c = vec![Mat2::zero(); n];
    let mut d = vec![Vec2::zero(); n];

    // Node 0: Upstream Boundary Condition
    // BC: \Delta Q_0 = q_up_next - q_current[0]
    // Equation 1 of node 0: 0 * \Delta y_0 + 1 * \Delta Q_0 = q_up_next - q_current[0]
    let b0_11 = 0.0;
    let b0_12 = 1.0;
    let d0_1 = q_up_next - q_current[0];

    // Node N-1: Downstream Boundary Condition
    // BC: \Delta y_{N-1} = y_down_next - y_current[N-1]
    // Equation 2 of node N-1: 1 * \Delta y_{N-1} + 0 * \Delta Q_{N-1} = y_down_next - y_current[N-1]
    let bn_21 = 1.0;
    let bn_22 = 0.0;
    let dn_2 = y_down_next - y_current[n - 1];

    // Populate intervals (0 to N-2)
    for i in 0..n - 1 {
        let dx = xs_list[i].station - xs_list[i + 1].station; // Reach length
        if dx <= 0.0 {
            return None; // Invalid station spacing
        }

        // Section properties at current time step
        let row_i = tables[i].interpolate(y_current[i]);
        let row_ip = tables[i + 1].interpolate(y_current[i + 1]);

        let a_i = row_i.area.max(1e-6);
        let a_ip = row_ip.area.max(1e-6);
        let t_i = row_i.top_width.max(1e-6);
        let t_ip = row_ip.top_width.max(1e-6);

        let v_i = q_current[i] / a_i;
        let v_ip = q_current[i + 1] / a_ip;

        // Conveyance and its derivatives
        let k_i = row_i.conveyance.max(1e-6);
        let k_ip = row_ip.conveyance.max(1e-6);
        
        let dk_dy_i = compute_dk_dy(&tables[i], y_current[i]);
        let dk_dy_ip = compute_dk_dy(&tables[i + 1], y_current[i + 1]);

        // Friction slope and derivatives
        let q_avg = 0.5 * (q_current[i] + q_current[i + 1]);
        let k_avg = 0.5 * (k_i + k_ip);
        let sf = (q_avg * q_avg.abs()) / (k_avg * k_avg);

        // dSf/dQ
        let d_sf_d_q = 2.0 * q_avg.abs() / (k_avg * k_avg);
        // dSf/dy (evaluated for node i and i+1)
        let d_sf_dy_i = -q_avg * q_avg.abs() / (k_avg * k_avg * k_avg) * dk_dy_i;
        let d_sf_dy_ip = -q_avg * q_avg.abs() / (k_avg * k_avg * k_avg) * dk_dy_ip;

        // Averaged variables
        let a_avg = 0.5 * (a_i + a_ip);

        // 1. CONTINUTIY EQUATION COEFFICIENTS
        // C1 * \Delta y_i + C2 * \Delta Q_i + C3 * \Delta y_{i+1} + C4 * \Delta Q_{i+1} = CE
        let c1 = t_i / (2.0 * dt);
        let c2 = -theta / dx;
        let c3 = t_ip / (2.0 * dt);
        let c4 = theta / dx;
        let ce = -(q_current[i + 1] - q_current[i]) / dx;

        // 2. MOMENTUM EQUATION COEFFICIENTS
        // M1 * \Delta y_i + M2 * \Delta Q_i + M3 * \Delta y_{i+1} + M4 * \Delta Q_{i+1} = ME
        // Momentum flux: d(Q^2/A)/dx.
        // d/dy(Q^2/A) = -Q^2/A^2 * dA/dy = -V^2 * T
        // d/dQ(Q^2/A) = 2Q/A = 2V
        let m1 = -theta / dx * (v_i * v_i * t_i) - G_METRIC * a_avg * theta / dx + 0.5 * G_METRIC * a_avg * theta * d_sf_dy_i;
        let m2 = 1.0 / (2.0 * dt) - theta / dx * (2.0 * v_i) + 0.5 * G_METRIC * a_avg * theta * d_sf_d_q;
        let m3 = theta / dx * (v_ip * v_ip * t_ip) + G_METRIC * a_avg * theta / dx + 0.5 * G_METRIC * a_avg * theta * d_sf_dy_ip;
        let m4 = 1.0 / (2.0 * dt) + theta / dx * (2.0 * v_ip) + 0.5 * G_METRIC * a_avg * theta * d_sf_d_q;

        let flux_t = (q_current[i + 1] * q_current[i + 1] / a_ip) - (q_current[i] * q_current[i] / a_i);
        let me = -(
            (q_current[i] + q_current[i + 1] - q_current[i] - q_current[i + 1]) / (2.0 * dt) // actually 0 for steady initial
            + flux_t / dx
            + G_METRIC * a_avg * (y_current[i + 1] - y_current[i]) / dx
            + G_METRIC * a_avg * sf
        );

        // Pack into block tridiagonal matrices
        // Block 0
        if i == 0 {
            b[0] = Mat2 {
                m11: b0_11, m12: b0_12,
                m21: c1,    m22: c2,
            };
            c[0] = Mat2 {
                m11: 0.0, m12: 0.0,
                m21: c3,  m22: c4,
            };
            d[0] = Vec2 {
                v1: d0_1,
                v2: ce,
            };
        } else {
            // Block i
            b[i] = Mat2 {
                m11: m3, m12: m4,
                m21: c1, m22: c2,
            };
            c[i] = Mat2 {
                m11: 0.0, m12: 0.0,
                m21: c3,  m22: c4,
            };
            d[i] = Vec2 {
                v1: me,
                v2: ce,
            };
            a[i] = Mat2 {
                m11: m1, m12: m2,
                m21: 0.0, m22: 0.0,
            };
        }

        // Block N-1 (last node)
        if i == n - 2 {
            a[n - 1] = Mat2 {
                m11: m1, m12: m2,
                m21: 0.0, m22: 0.0,
            };
            b[n - 1] = Mat2 {
                m11: m3,    m12: m4,
                m21: bn_21, m22: bn_22,
            };
            d[n - 1] = Vec2 {
                v1: me,
                v2: dn_2,
            };
        }
    }

    // Solve system
    let delta = solve_block_tridiagonal(&a, &b, &c, &d)?;

    // Apply updates
    let mut y_next = vec![0.0; n];
    let mut q_next = vec![0.0; n];
    for i in 0..n {
        y_next[i] = y_current[i] + delta[i].v1;
        q_next[i] = q_current[i] + delta[i].v2;
    }

    Some((y_next, q_next))
}

/// Solves unsteady-state Saint-Venant flow routing.
pub fn solve_unsteady(inputs: &UnsteadyInputs) -> UnsteadyResult {
    let raw_units = inputs.cross_sections.first().map(|xs| xs.unit_system).unwrap_or(UnitSystem::Metric);
    let dt = inputs.dt;
    let num_slices = inputs.num_slices.unwrap_or(100);
    let theta = inputs.theta.unwrap_or(0.6).clamp(0.5, 1.0);

    // Convert cross-sections to metric and sort descending by station
    let mut xs_list: Vec<CrossSection> = inputs.cross_sections.iter().map(|xs| xs.to_metric()).collect();
    xs_list.sort_by(|a, b| b.station.partial_cmp(&a.station).unwrap());
    let m = xs_list.len();

    // Map from original index to sorted index for indexing initial states
    let mut original_mapping = vec![0; m];
    for (orig_idx, orig_xs) in inputs.cross_sections.iter().enumerate() {
        let mut sorted_idx = 0;
        for (s_idx, s_xs) in xs_list.iter().enumerate() {
            if (s_xs.station - (orig_xs.station * if raw_units == UnitSystem::USCustomary { FT_TO_M } else { 1.0 })).abs() < 1e-4 {
                sorted_idx = s_idx;
                break;
            }
        }
        original_mapping[orig_idx] = sorted_idx;
    }

    // Setup initial conditions in metric
    let mut y_current = vec![0.0; m];
    let mut q_current = vec![0.0; m];
    for orig_idx in 0..m {
        let sorted_idx = original_mapping[orig_idx];
        let wsel_val = inputs.initial_wsel[orig_idx];
        let q_val = inputs.initial_q[orig_idx];
        
        y_current[sorted_idx] = if raw_units == UnitSystem::USCustomary { wsel_val * FT_TO_M } else { wsel_val };
        q_current[sorted_idx] = if raw_units == UnitSystem::USCustomary { q_val * crate::utils::CFS_TO_CMS } else { q_val };
    }

    // Pre-build geometry tables
    let tables: Vec<GeometryTable> = xs_list.iter().map(|xs| xs.generate_lookup_table(num_slices)).collect();

    // Prepare time hydrographs in metric
    let mut q_up_hydrograph = vec![0.0; inputs.num_steps];
    let mut y_down_hydrograph = vec![0.0; inputs.num_steps];
    for step in 0..inputs.num_steps {
        q_up_hydrograph[step] = if raw_units == UnitSystem::USCustomary {
            inputs.upstream_q_hydrograph[step] * crate::utils::CFS_TO_CMS
        } else {
            inputs.upstream_q_hydrograph[step]
        };
        y_down_hydrograph[step] = if raw_units == UnitSystem::USCustomary {
            inputs.downstream_wsel_hydrograph[step] * FT_TO_M
        } else {
            inputs.downstream_wsel_hydrograph[step]
        };
    }

    let mut history_wsel = Vec::new();
    let mut history_q = Vec::new();
    let mut history_vel = Vec::new();

    // Loop through time steps
    for step in 0..inputs.num_steps {
        let q_up_next = q_up_hydrograph[step];
        let y_down_next = y_down_hydrograph[step];

        // Solve next time step
        if let Some((y_next, q_next)) = solve_unsteady_step(
            &tables,
            &xs_list,
            &y_current,
            &q_current,
            dt,
            q_up_next,
            y_down_next,
            theta,
        ) {
            y_current = y_next;
            q_current = q_next;
        } else {
            // If the matrix solver fails to invert (rare), maintain current state as fallback
        }

        // Convert current step back to user units and original layout
        let mut step_wsel = vec![0.0; m];
        let mut step_q = vec![0.0; m];
        let mut step_vel = vec![0.0; m];

        for orig_idx in 0..m {
            let sorted_idx = original_mapping[orig_idx];
            let w_val = y_current[sorted_idx];
            let q_val = q_current[sorted_idx];
            
            let row = tables[sorted_idx].interpolate(w_val);
            let vel_val = if row.area > 1e-6 { q_val / row.area } else { 0.0 };

            if raw_units == UnitSystem::USCustomary {
                step_wsel[orig_idx] = w_val / FT_TO_M;
                step_q[orig_idx] = q_val / crate::utils::CFS_TO_CMS;
                step_vel[orig_idx] = vel_val / FT_TO_M;
            } else {
                step_wsel[orig_idx] = w_val;
                step_q[orig_idx] = q_val;
                step_vel[orig_idx] = vel_val;
            }
        }

        history_wsel.push(step_wsel);
        history_q.push(step_q);
        history_vel.push(step_vel);
    }

    UnsteadyResult {
        wsel: history_wsel,
        q: history_q,
        velocity: history_vel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsteady_stability() {
        // Set up 3 cross-sections spaced 500m apart (total 1000m length).
        // Rectangular channel: width = 10m, Manning's n = 0.02.
        // Stationing: 1000, 500, 0.
        // Bed elevations: 1.0, 0.5, 0.0.
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0 + 1.0, 1.0, 1.0, 5.0 + 1.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
        };
        let xs500 = CrossSection {
            station: 500.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0 + 0.5, 0.5, 0.5, 5.0 + 0.5],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
        };

        // Run a simulation keeping inputs constant at 14.0 cms (uniform flow equilibrium depth = 1.0m) and WSEL = 1.0m downstream
        let inputs = UnsteadyInputs {
            cross_sections: vec![xs1000, xs500, xs0],
            initial_wsel: vec![2.0, 1.5, 1.0], // constant depth = 1.0m
            initial_q: vec![14.0, 14.0, 14.0],
            dt: 60.0,
            num_steps: 5,
            upstream_q_hydrograph: vec![14.0; 5],
            downstream_wsel_hydrograph: vec![1.0; 5],
            theta: Some(0.6),
            num_slices: Some(50),
        };

        let result = solve_unsteady(&inputs);

        // Result assertions
        assert_eq!(result.wsel.len(), 5);
        assert_eq!(result.q.len(), 5);

        // Verify that the flow rates Q remain close to 14.0 cms over the simulation
        for step in 0..5 {
            for node in 0..3 {
                let q_val = result.q[step][node];
                assert!((q_val - 14.0).abs() < 1e-1, "Step {} Node {} Q was {}", step, node, q_val);
            }
        }
    }
}

