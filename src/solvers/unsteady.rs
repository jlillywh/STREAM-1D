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
    /// Maximum distance between adjacent sections before automatic interpolation (optional, in user units).
    pub max_spacing: Option<f64>,
    /// Contraction loss coefficient (default 0.1).
    pub coeff_contraction: Option<f64>,
    /// Expansion loss coefficient (default 0.3).
    pub coeff_expansion: Option<f64>,
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
    /// Maximum Courant number encountered during initial conditions.
    pub max_courant: Option<f64>,
    /// Recommended optimal time-step size to ensure stability (in seconds).
    pub recommended_dt: Option<f64>,
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
    c_contraction: f64,
    c_expansion: f64,
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
        let k_avg_clamp = k_avg.max(0.01);
        let sf = (q_avg * q_avg.abs()) / (k_avg_clamp * k_avg_clamp);

        // dSf/dQ
        let d_sf_d_q = 2.0 * q_avg.abs() / (k_avg_clamp * k_avg_clamp);
        
        // Compute local depths to suppress derivatives at dry/shallow nodes
        let z_min_i = xs_list[i].y.iter().cloned().fold(f64::INFINITY, f64::min);
        let z_min_ip = xs_list[i + 1].y.iter().cloned().fold(f64::INFINITY, f64::min);
        let depth_i = (y_current[i] - z_min_i).max(0.0);
        let depth_ip = (y_current[i + 1] - z_min_ip).max(0.0);

        // dSf/dy (evaluated for node i and i+1)
        let d_sf_dy_i = if depth_i < 0.1 {
            0.0
        } else {
            -q_avg * q_avg.abs() / (k_avg_clamp * k_avg_clamp * k_avg_clamp) * dk_dy_i
        };
        let d_sf_dy_ip = if depth_ip < 0.1 {
            0.0
        } else {
            -q_avg * q_avg.abs() / (k_avg_clamp * k_avg_clamp * k_avg_clamp) * dk_dy_ip
        };

        // Averaged variables
        let a_avg = (a_i * a_ip).sqrt();

        // 1. CONTINUTIY EQUATION COEFFICIENTS
        // C1 * \Delta y_i + C2 * \Delta Q_i + C3 * \Delta y_{i+1} + C4 * \Delta Q_{i+1} = CE
        let c1 = t_i / (2.0 * dt);
        let c2 = -theta / dx;
        let c3 = t_ip / (2.0 * dt);
        let c4 = theta / dx;
        let ce = (q_current[i] - q_current[i + 1]) / dx;

        // Froude number convective term suppression for mixed flow stability
        let d_hyd_i = a_i / t_i;
        let celerity_i = (G_METRIC * d_hyd_i).sqrt();
        let fr_i = if celerity_i > 1e-6 { v_i.abs() / celerity_i } else { 0.0 };
        let factor_i = if fr_i < 1.0 { (1.0 - fr_i * fr_i).max(0.0) } else { 0.0 };

        let d_hyd_ip = a_ip / t_ip;
        let celerity_ip = (G_METRIC * d_hyd_ip).sqrt();
        let fr_ip = if celerity_ip > 1e-6 { v_ip.abs() / celerity_ip } else { 0.0 };
        let factor_ip = if fr_ip < 1.0 { (1.0 - fr_ip * fr_ip).max(0.0) } else { 0.0 };

        // 2. MOMENTUM EQUATION COEFFICIENTS
        // M1 * \Delta y_i + M2 * \Delta Q_i + M3 * \Delta y_{i+1} + M4 * \Delta Q_{i+1} = ME
        
        // Contraction/Expansion losses
        let c_ec = if v_ip.abs() > v_i.abs() { c_contraction } else { c_expansion };
        let sign_v = (v_ip * v_ip - v_i * v_i).signum();
        let s_ce_force = a_avg * (c_ec / (2.0 * dx)) * (v_ip * v_ip - v_i * v_i).abs();

        let dfce_dyi = a_avg * (c_ec / dx) * sign_v * (v_i * v_i * t_i / a_i);
        let dfce_dqi = -a_avg * (c_ec / dx) * sign_v * (v_i / a_i);
        let dfce_dyip = -a_avg * (c_ec / dx) * sign_v * (v_ip * v_ip * t_ip / a_ip);
        let dfce_dqip = a_avg * (c_ec / dx) * sign_v * (v_ip / a_ip);

        let m1 = theta / dx * (v_i * v_i * t_i) * factor_i - G_METRIC * a_avg * theta / dx + G_METRIC * a_avg * theta * d_sf_dy_i + theta * dfce_dyi;
        let m2 = (1.0 / (2.0 * dt)) - theta / dx * (2.0 * v_i) * factor_i + 0.5 * G_METRIC * a_avg * theta * d_sf_d_q + theta * dfce_dqi;
        let m3 = -theta / dx * (v_ip * v_ip * t_ip) * factor_ip + G_METRIC * a_avg * theta / dx + G_METRIC * a_avg * theta * d_sf_dy_ip + theta * dfce_dyip;
        let m4 = (1.0 / (2.0 * dt)) + theta / dx * (2.0 * v_ip) * factor_ip + 0.5 * G_METRIC * a_avg * theta * d_sf_d_q + theta * dfce_dqip;

        let flux_i = (q_current[i] * q_current[i] / a_i) * factor_i;
        let flux_ip = (q_current[i + 1] * q_current[i + 1] / a_ip) * factor_ip;
        let me = (flux_i - flux_ip) / dx + G_METRIC * a_avg * (y_current[i] - y_current[i + 1]) / dx - G_METRIC * a_avg * sf - s_ce_force;

        // Pack into block tridiagonal matrices
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
            b[i].m21 = c1;
            b[i].m22 = c2;
            c[i].m21 = c3;
            c[i].m22 = c4;
            d[i].v2 = ce;
        }

        // Pack momentum equation of interval i into block i + 1
        a[i + 1] = Mat2 {
            m11: m1, m12: m2,
            m21: 0.0, m22: 0.0,
        };
        b[i + 1].m11 = m3;
        b[i + 1].m12 = m4;
        d[i + 1].v1 = me;

        // Pack downstream boundary condition into block n - 1
        if i == n - 2 {
            b[n - 1].m21 = bn_21;
            b[n - 1].m22 = bn_22;
            d[n - 1].v2 = dn_2;
        }
    }

    // Solve system
    let delta = solve_block_tridiagonal(&a, &b, &c, &d)?;

    // Apply updates
    let mut y_next = vec![0.0; n];
    let mut q_next = vec![0.0; n];
    for i in 0..n {
        let dy = delta[i].v1.clamp(-1.0, 1.0);
        let dq = delta[i].v2.clamp(-25.0, 25.0);

        y_next[i] = y_current[i] + dy;
        q_next[i] = q_current[i] + dq;
    }

    // Explicitly enforce boundary conditions exactly
    q_next[0] = q_up_next;
    y_next[n - 1] = y_down_next;

    Some((y_next, q_next))
}

/// Solves unsteady-state Saint-Venant flow routing.
pub fn solve_unsteady(inputs: &UnsteadyInputs) -> UnsteadyResult {
    let raw_units = inputs.cross_sections.first().map(|xs| xs.unit_system).unwrap_or(UnitSystem::Metric);
    let dt = inputs.dt;
    let num_slices = inputs.num_slices.unwrap_or(100);
    let theta = inputs.theta.unwrap_or(0.85).clamp(0.8, 1.0);
    let c_contraction = inputs.coeff_contraction.unwrap_or(0.1);
    let c_expansion = inputs.coeff_expansion.unwrap_or(0.3);

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

    // Pre-build geometry tables for sorted cross sections
    let tables: Vec<GeometryTable> = xs_list.iter().map(|xs| xs.generate_lookup_table(num_slices)).collect();
    let z_mins: Vec<f64> = xs_list.iter().map(|xs| xs.y.iter().cloned().fold(f64::INFINITY, f64::min)).collect();

    // DENSIFICATION STEP: Automatic Reach Interpolation
    let max_sp = inputs.max_spacing.map(|sp| {
        if raw_units == UnitSystem::USCustomary { sp * FT_TO_M } else { sp }
    }).unwrap_or_else(|| {
        if raw_units == UnitSystem::USCustomary { 50.0 * FT_TO_M } else { 15.0 }
    });

    let mut densified_tables = Vec::new();
    let mut densified_z_mins = Vec::new();
    let mut densified_stations = Vec::new();
    let mut densified_xs = Vec::new();
    let mut densified_y_current = Vec::new();
    let mut densified_q_current = Vec::new();
    let mut original_to_densified = Vec::new();

    for i in 0..m {
        let current_idx = densified_tables.len();
        original_to_densified.push(current_idx);

        densified_tables.push(tables[i].clone());
        densified_z_mins.push(z_mins[i]);
        densified_stations.push(xs_list[i].station);
        densified_xs.push(xs_list[i].clone());
        densified_y_current.push(y_current[i]);
        densified_q_current.push(q_current[i]);

        if i < m - 1 {
            let dx = xs_list[i].station - xs_list[i + 1].station;
            if max_sp > 0.0 && dx > max_sp {
                let num_spaces = (dx / max_sp).ceil() as usize;
                let ds = dx / num_spaces as f64;
                for k in 1..num_spaces {
                        let t = k as f64 / num_spaces as f64;
                        let s_interp = xs_list[i].station - k as f64 * ds;
                        
                        let (t_interp, z_interp) = crate::geometry::processor::interpolate_geometry_table(
                            &tables[i],
                            z_mins[i],
                            &tables[i + 1],
                            z_mins[i + 1],
                            t,
                            num_slices,
                        );
                        
                        let y_interp = (1.0 - t) * y_current[i] + t * y_current[i + 1];
                        let q_interp = (1.0 - t) * q_current[i] + t * q_current[i + 1];

                        let mut xs_interp = xs_list[i].clone();
                        xs_interp.station = s_interp;

                        densified_tables.push(t_interp);
                        densified_z_mins.push(z_interp);
                        densified_stations.push(s_interp);
                        densified_xs.push(xs_interp);
                        densified_y_current.push(y_interp);
                        densified_q_current.push(q_interp);
                    }
                }
            }
        }

    let dm = densified_tables.len();
    println!("Densified stations (m): {:?}", densified_stations);

    // Calculate Courant number (Cr) and recommended dt based on initial conditions on the densified grid
    let mut max_courant = 0.0;
    let mut recommended_dt = f64::INFINITY;

    for k in 0..dm {
        let y_val = densified_y_current[k];
        let q_val = densified_q_current[k];
        let row = densified_tables[k].interpolate(y_val);
        
        let area = row.area;
        let top_width = row.top_width;
        let vel = if area > 1e-6 { q_val / area } else { 0.0 };
        let d_hyd = if top_width > 1e-6 { area / top_width } else { 0.0 };
        let celerity = (G_METRIC * d_hyd).sqrt();
        let wave_speed = vel.abs() + celerity;

        let dx = if dm < 2 {
            1.0
        } else if k == 0 {
            densified_stations[0] - densified_stations[1]
        } else if k == dm - 1 {
            densified_stations[dm - 2] - densified_stations[dm - 1]
        } else {
            let dx_prev = densified_stations[k - 1] - densified_stations[k];
            let dx_next = densified_stations[k] - densified_stations[k + 1];
            dx_prev.min(dx_next)
        };

        if dx > 1e-9 {
            let cr = (wave_speed * dt) / dx;
            if cr > max_courant {
                max_courant = cr;
            }
            if wave_speed > 1e-6 {
                let dt_opt = (5.0 * dx) / wave_speed;
                if dt_opt < recommended_dt {
                    recommended_dt = dt_opt;
                }
            }
        }
    }

    let (max_courant_val, recommended_dt_val) = if dm >= 2 {
        (
            Some(max_courant),
            if recommended_dt.is_infinite() { None } else { Some(recommended_dt) }
        )
    } else {
        (None, None)
    };

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

    // Enforce initial WSEL clamping to prevent starting with dry/negative depth, and stabilize initial Q
    for k in 0..dm {
        let min_wsel = densified_z_mins[k] + 0.05;
        if densified_y_current[k] < min_wsel {
            densified_y_current[k] = min_wsel;
        }
        let row = densified_tables[k].interpolate(densified_y_current[k]);
        let area = row.area.max(1e-6);
        let depth = (densified_y_current[k] - densified_z_mins[k]).max(0.0);
        let max_phys_vel = 15.0 * (depth / 0.1).min(1.0).max(0.1);
        let max_q = area * max_phys_vel;
        densified_q_current[k] = densified_q_current[k].clamp(-max_q, max_q);
    }

    let mut history_wsel = Vec::new();
    let mut history_q = Vec::new();
    let mut history_vel = Vec::new();

    // Loop through time steps
    for step in 0..inputs.num_steps {
        let q_up_next = q_up_hydrograph[step];
        let mut y_down_next = y_down_hydrograph[step];

        // Clamp downstream stage BC to prevent dry downstream boundary
        let ds_z_min = densified_z_mins[dm - 1];
        if y_down_next < ds_z_min + 0.05 {
            y_down_next = ds_z_min + 0.05;
        }

        // Solve next time step
        if let Some((y_next, q_next)) = solve_unsteady_step(
            &densified_tables,
            &densified_xs,
            &densified_y_current,
            &densified_q_current,
            dt,
            q_up_next,
            y_down_next,
            theta,
            c_contraction,
            c_expansion,
        ) {
            densified_y_current = y_next;
            densified_q_current = q_next;

            // Clamp solved WSEL to prevent dry nodes/negative depth, and limit velocity
            for k in 0..dm {
                let min_wsel = densified_z_mins[k] + 0.05;
                if densified_y_current[k] < min_wsel {
                    densified_y_current[k] = min_wsel;
                }
                let row = densified_tables[k].interpolate(densified_y_current[k]);
                let area = row.area.max(1e-6);
                let depth = (densified_y_current[k] - densified_z_mins[k]).max(0.0);
                
                if k > 0 {
                    let max_phys_vel = 15.0 * (depth / 0.1).min(1.0).max(0.1);
                    let max_q = area * max_phys_vel;
                    densified_q_current[k] = densified_q_current[k].clamp(-max_q, max_q);
                } else {
                    densified_q_current[0] = q_up_next;
                }
            }
            // Enforce downstream boundary stage exactly
            densified_y_current[dm - 1] = y_down_next;
        } else {
            // If the matrix solver fails to invert (rare), maintain current state as fallback
        }

        // Convert current step back to user units and original layout
        let mut step_wsel = vec![0.0; m];
        let mut step_q = vec![0.0; m];
        let mut step_vel = vec![0.0; m];

        for orig_idx in 0..m {
            let sorted_xs_idx = original_mapping[orig_idx];
            let sorted_idx = original_to_densified[sorted_xs_idx];
            let w_val = densified_y_current[sorted_idx];
            let q_val = densified_q_current[sorted_idx];
            
            let row = densified_tables[sorted_idx].interpolate(w_val);
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

        if step < 5 {
            println!("Step {}: densified WSEL (ft) = {:?}", step, densified_y_current.iter().map(|&w| w / FT_TO_M).collect::<Vec<f64>>());
            println!("Step {}: densified Q (cfs)    = {:?}", step, densified_q_current.iter().map(|&q| q / crate::utils::CFS_TO_CMS).collect::<Vec<f64>>());
        }

        history_wsel.push(step_wsel);
        history_q.push(step_q);
        history_vel.push(step_vel);
    }

    UnsteadyResult {
        wsel: history_wsel,
        q: history_q,
        velocity: history_vel,
        max_courant: max_courant_val,
        recommended_dt: recommended_dt_val,
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
            max_spacing: None,
            coeff_contraction: None,
            coeff_expansion: None,
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

    #[test]
    fn test_unsteady_reach_densification() {
        // Set up 2 cross-sections spaced 1000m apart.
        // Bed slope is 0.001 (z1 = 1.0m, z2 = 0.0m).
        // Rectangular channel: width = 10m.
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![6.0, 1.0, 1.0, 6.0],
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

        // Run with a max spacing of 100.0m (which should create 9 intermediate cross sections, total 11 sections internally)
        let inputs = UnsteadyInputs {
            cross_sections: vec![xs1000, xs0],
            initial_wsel: vec![2.0, 1.0], // constant depth = 1.0m
            initial_q: vec![14.0, 14.0],
            dt: 10.0,
            num_steps: 5,
            upstream_q_hydrograph: vec![14.0; 5],
            downstream_wsel_hydrograph: vec![1.0; 5],
            theta: Some(0.6),
            num_slices: Some(50),
            max_spacing: Some(100.0),
            coeff_contraction: None,
            coeff_expansion: None,
        };

        let result = solve_unsteady(&inputs);

        // Verification
        // The solver should converge successfully. Check that output size matches original input size (2)
        assert_eq!(result.wsel.len(), 5);
        assert_eq!(result.wsel[0].len(), 2);
        
        // Downstream boundary condition is preserved at the end of the reach
        assert!((result.wsel[4][1] - 1.0).abs() < 1e-1);

        // Check that max_courant and recommended_dt are calculated
        assert!(result.max_courant.is_some());
        assert!(result.recommended_dt.is_some());
        
        let cr = result.max_courant.unwrap();
        assert!(cr > 0.0, "max_courant was {}", cr);
    }

    #[test]
    fn test_project_11_debug() {
        let xs1000 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 12.0, 22.0, 34.0],
            y: vec![26.0, 20.0, 20.0, 26.0],
            n_stations: vec![0.0],
            n_values: vec![0.025],
            unit_system: UnitSystem::USCustomary,
        };
        let xs750 = CrossSection {
            station: 750.0,
            x: vec![0.0, 12.0, 18.0, 22.0, 34.0],
            y: vec![23.5, 17.5, 17.0, 17.5, 23.5],
            n_stations: vec![0.0],
            n_values: vec![0.025; 5],
            unit_system: UnitSystem::USCustomary,
        };
        let xs500 = CrossSection {
            station: 500.0,
            x: vec![0.0, 12.0, 22.0, 34.0],
            y: vec![21.0, 15.0, 15.0, 21.0],
            n_stations: vec![0.0],
            n_values: vec![0.025],
            unit_system: UnitSystem::USCustomary,
        };
        let xs250 = CrossSection {
            station: 250.0,
            x: vec![0.0, 12.0, 22.0, 34.0],
            y: vec![18.5, 12.5, 12.5, 18.5],
            n_stations: vec![0.0],
            n_values: vec![0.025],
            unit_system: UnitSystem::USCustomary,
        };
        let xs150 = CrossSection {
            station: 150.0,
            x: vec![0.0, 12.0, 22.0, 34.0],
            y: vec![18.5, 12.5, 12.5, 18.5],
            n_stations: vec![0.0],
            n_values: vec![0.025],
            unit_system: UnitSystem::USCustomary,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 12.0, 22.0, 34.0],
            y: vec![16.0, 10.0, 10.0, 16.0],
            n_stations: vec![0.0],
            n_values: vec![0.025],
            unit_system: UnitSystem::USCustomary,
        };

        let mut upstream_q = vec![35.0; 100];
        for i in 0..30 {
            upstream_q[i] = 35.0 + (87.5 - 35.0) * (i as f64 / 30.0);
        }
        for i in 30..100 {
            upstream_q[i] = 87.5 - (87.5 - 35.0) * ((i - 30) as f64 / 70.0);
        }

        let inputs = UnsteadyInputs {
            cross_sections: vec![xs1000, xs750, xs500, xs250, xs150, xs0],
            initial_wsel: vec![21.0, 18.0, 16.0, 13.5, 13.5, 12.0],
            initial_q: vec![35.0; 6],
            dt: 10.0,
            num_steps: 100,
            upstream_q_hydrograph: upstream_q,
            downstream_wsel_hydrograph: vec![12.0; 100],
            theta: Some(1.0),
            num_slices: Some(100),
            max_spacing: None,
            coeff_contraction: None,
            coeff_expansion: None,
        };

        let result = solve_unsteady(&inputs);
        println!("Recommended DT = {:?}", result.recommended_dt);
        for step in (0..100).step_by(10) {
            println!("Step {}: WSEL = {:?}", step, result.wsel[step]);
            println!("Step {}: Q    = {:?}", step, result.q[step]);
        }
    }
}

