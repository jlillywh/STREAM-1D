use crate::utils::{G_METRIC, UnitSystem, FT_TO_M};
use crate::geometry::{CrossSection, GeometryTable};

/// Input parameters for the steady-state solver.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SteadyInputs {
    /// Cross-sections defining the river reach.
    pub cross_sections: Vec<CrossSection>,
    /// Flow rate (in cfs if unit_system is USCustomary, cms if Metric).
    pub flow_rate: f64,
    /// Number of uniform vertical intervals to slice cross-sections (default 100).
    pub num_slices: Option<usize>,
    /// Contraction loss coefficient (default 0.1).
    pub coeff_contraction: Option<f64>,
    /// Expansion loss coefficient (default 0.3).
    pub coeff_expansion: Option<f64>,
    /// Flow regime (0 = Subcritical, 1 = Supercritical, 2 = Mixed).
    pub regime: u8,
    /// Downstream WSEL boundary condition (optional, in user units).
    pub downstream_wsel: Option<f64>,
    /// Upstream WSEL boundary condition (optional, in user units).
    pub upstream_wsel: Option<f64>,
    /// Maximum distance between adjacent sections before automatic interpolation (optional, in user units).
    pub max_spacing: Option<f64>,
    /// Culvert stations (optional)
    #[serde(default)]
    pub culvert_stations: Option<Vec<f64>>,
    /// Culvert shape types (optional, 0 = Circular, 1 = Box, 2 = Arch)
    #[serde(default)]
    pub culvert_shape_types: Option<Vec<i32>>,
    /// Culvert spans/diameters (optional, in feet/meters)
    #[serde(default)]
    pub culvert_spans: Option<Vec<f64>>,
    /// Culvert rises (optional, in feet/meters)
    #[serde(default)]
    pub culvert_rises: Option<Vec<f64>>,
    /// Culvert Manning's n roughness coefficients (optional)
    #[serde(default)]
    pub culvert_roughness_ns: Option<Vec<f64>>,
    /// Culvert lengths (optional, in feet/meters)
    #[serde(default)]
    pub culvert_lengths: Option<Vec<f64>>,
    /// Culvert entrance loss coefficients Ke (optional)
    #[serde(default)]
    pub culvert_entrance_loss_coeffs: Option<Vec<f64>>,
    /// Culvert exit loss coefficients Kx (optional)
    #[serde(default)]
    pub culvert_exit_loss_coeffs: Option<Vec<f64>>,

    /// Stations where bridges are located (in user units, e.g. feet or meters)
    #[serde(default)]
    pub bridge_stations: Option<Vec<f64>>,
    /// Elevation of the lowest point of the bridge deck at each bridge
    #[serde(default)]
    pub bridge_low_chords: Option<Vec<f64>>,
    /// Elevation of the top of the roadway deck at each bridge
    #[serde(default)]
    pub bridge_high_chords: Option<Vec<f64>>,
    /// Thickness/width of a single pier at each bridge
    #[serde(default)]
    pub bridge_pier_widths: Option<Vec<f64>>,
    /// Number of piers at each bridge
    #[serde(default)]
    pub bridge_num_piers: Option<Vec<i32>>,
    /// Pier shape classification (0 = Square, 1 = Semicircular, 2 = Twin Cylinders, 3 = Sharp/Triangular)
    #[serde(default)]
    pub bridge_pier_shapes: Option<Vec<i32>>,
    /// Weir discharge coefficient Cw for overtopping flow (e.g., default 2.6 US, 1.44 Metric)
    #[serde(default)]
    pub bridge_weir_coeffs: Option<Vec<f64>>,
    /// Orifice discharge coefficient Cd for pressure flow (e.g., default 0.5 or 0.6)
    #[serde(default)]
    pub bridge_orifice_coeffs: Option<Vec<f64>>,
}

/// Output results from the steady-state solver.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SteadyResult {
    /// Solved water surface elevations (WSEL) at each cross-section (in user units).
    pub wsel: Vec<f64>,
    /// Critical depth elevations (y_c + z_min) at each cross-section (in user units).
    pub critical_wsel: Vec<f64>,
    /// Velocity values at each cross-section (in user units, ft/s or m/s).
    pub velocity: Vec<f64>,
    /// Flow areas at each cross-section (in user units, ft^2 or m^2).
    pub area: Vec<f64>,
    /// Froude numbers at each cross-section.
    pub froude: Vec<f64>,
}

impl GeometryTable {
    /// Calculates the first moment of area A*y_bar about the water surface.
    /// Mathematically, A*y_bar = \int_{y_min}^{elev} A(y) dy.
    pub fn calculate_area_moment(&self, elev: f64) -> f64 {
        let n_rows = self.rows.len();
        if n_rows == 0 || elev <= self.rows[0].elevation {
            return 0.0;
        }

        let mut moment = 0.0;
        let limit = elev.min(self.rows[n_rows - 1].elevation);

        // Integrate A(y) using trapezoids across the intervals
        for i in 0..n_rows - 1 {
            let y1 = self.rows[i].elevation;
            let y2 = self.rows[i + 1].elevation;

            if limit <= y1 {
                break;
            }

            let h = if limit >= y2 {
                y2 - y1
            } else {
                limit - y1
            };

            let a1 = self.rows[i].area;
            let a2 = if limit >= y2 {
                self.rows[i + 1].area
            } else {
                // Interpolate area at limit
                let t = h / (y2 - y1);
                a1 + t * (self.rows[i + 1].area - a1)
            };

            moment += 0.5 * (a1 + a2) * h;
        }

        // Handle extrapolation above maximum row if necessary
        if elev > self.rows[n_rows - 1].elevation {
            let last = self.rows[n_rows - 1];
            let h = elev - last.elevation;
            // A(y) = last.area + last.top_width * (y - last.elevation)
            // Integral of A(y) = last.area * h + 0.5 * last.top_width * h^2
            moment += last.area * h + 0.5 * last.top_width * h * h;
        }

        moment
    }

    /// Calculates momentum force (Specific Force) M = Q^2 / (g * A) + A * y_bar
    pub fn calculate_specific_force(&self, elev: f64, q: f64) -> f64 {
        let row = self.interpolate(elev);
        if row.area < 1e-6 {
            return f64::INFINITY;
        }
        let area_moment = self.calculate_area_moment(elev);
        (q * q) / (G_METRIC * row.area) + area_moment
    }
}

/// Solves critical depth (yc) relative to bottom elevation for a cross section lookup table.
pub fn solve_critical_depth_table(table: &GeometryTable, q: f64) -> f64 {
    if table.rows.is_empty() {
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
        let row = table.interpolate(elev);

        if row.area < 1e-6 {
            low = mid;
            continue;
        }

        // Fr^2 = Q^2 * T / (g * A^3)
        let fr_sq = (q * q * row.top_width) / (G_METRIC * row.area.powi(3));
        let f_val = 1.0 - fr_sq;

        if f_val.abs() < 1e-6 {
            best_yc = mid;
            break;
        }

        if f_val < 0.0 {
            // Supercritical (depth too small)
            low = mid;
        } else {
            // Subcritical (depth too big)
            high = mid;
        }
        best_yc = mid;
    }
    best_yc
}

/// Solves critical depth (yc) relative to bottom elevation for a cross section.
pub fn solve_critical_depth(_xs: &CrossSection, table: &GeometryTable, q: f64) -> f64 {
    solve_critical_depth_table(table, q)
}

/// Steps from section 1 (known WSEL) to section 2 (unknown WSEL) using the Standard Step Method.
pub fn solve_step(
    table1: &GeometryTable,
    y1: f64, // WSEL 1
    table2: &GeometryTable,
    z2_min: f64,
    yc2: f64,
    q: f64,
    length: f64,
    c_contraction: f64,
    c_expansion: f64,
    is_subcritical: bool,
) -> Option<f64> {
    let row1 = table1.interpolate(y1);
    if row1.area < 1e-6 {
        return None;
    }
    let hv1 = (q * q) / (2.0 * G_METRIC * row1.area * row1.area);
    let k1 = row1.conveyance;

    let target_residual = |y2: f64| -> Option<f64> {
        let row2 = table2.interpolate(y2);
        if row2.area < 1e-6 {
            return None;
        }
        let hv2 = (q * q) / (2.0 * G_METRIC * row2.area * row2.area);
        let k2 = row2.conveyance;

        let k_avg = 0.5 * (k1 + k2);
        if k_avg < 1e-9 {
            return None;
        }
        let sf = (q / k_avg).powi(2);
        let hf = length * sf;

        let c_ec = if hv2 > hv1 { c_contraction } else { c_expansion };
        let ho = c_ec * (hv2 - hv1).abs();

        if is_subcritical {
            // Upstream step: H2 = H1 + hf + ho
            Some(y2 + hv2 - (y1 + hv1 + hf + ho))
        } else {
            // Downstream step: H2 = H1 - hf - ho
            Some(y2 + hv2 - (y1 + hv1 - hf - ho))
        }
    };

    // Define search bounds based on flow regime to prevent conjugate depth crossing
    let (mut low, mut high) = if is_subcritical {
        let l = z2_min + yc2 + 1e-5;
        let h = y1.max(z2_min + yc2) + 20.0;
        (l, h)
    } else {
        let l = z2_min + 1e-5;
        let h = z2_min + yc2 - 1e-5;
        (l, h)
    };

    let res_low = target_residual(low)?;
    let mut res_high = target_residual(high)?;

    if res_low * res_high > 0.0 {
        if is_subcritical {
            // Expand subcritical upper bound if needed
            for _ in 0..5 {
                high += 20.0;
                if let Some(r_high) = target_residual(high) {
                    res_high = r_high;
                    if res_low * res_high <= 0.0 {
                        break;
                    }
                }
            }
        }
        if res_low * res_high > 0.0 {
            // Failed to bracket root, fallback to critical depth
            return Some(z2_min + yc2);
        }
    }

    let mut best_y = 0.5 * (low + high);
    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let res_mid = match target_residual(mid) {
            Some(r) => r,
            None => {
                high = mid;
                continue;
            }
        };

        if res_mid.abs() < 1e-8 {
            best_y = mid;
            break;
        }

        if is_subcritical {
            if res_mid < 0.0 {
                low = mid;
            } else {
                high = mid;
            }
        } else {
            if res_mid > 0.0 {
                low = mid;
            } else {
                high = mid;
            }
        }
        best_y = mid;
    }

    Some(best_y)
}

/// Runs the steady-state water surface profile solver.
pub fn solve_steady(inputs: &SteadyInputs) -> SteadyResult {
    let raw_units = inputs.cross_sections.first().map(|xs| xs.unit_system).unwrap_or(UnitSystem::Metric);
    let q = if raw_units == UnitSystem::USCustomary {
        inputs.flow_rate * crate::utils::CFS_TO_CMS
    } else {
        inputs.flow_rate
    };

    let num_slices = inputs.num_slices.unwrap_or(100);
    let c_contraction = inputs.coeff_contraction.unwrap_or(0.1);
    let c_expansion = inputs.coeff_expansion.unwrap_or(0.3);

    // Convert all cross sections to metric internally
    let mut xs_list: Vec<CrossSection> = inputs.cross_sections.iter().map(|xs| xs.to_metric()).collect();
    
    // Sort descending by station (upstream to downstream)
    // Upstream has larger station numbers, index 0 is most upstream.
    xs_list.sort_by(|a, b| b.station.partial_cmp(&a.station).unwrap());
    let m = xs_list.len();

    // Generate geometry tables and calculate bed elevations
    let tables: Vec<GeometryTable> = xs_list.iter().map(|xs| xs.generate_lookup_table(num_slices)).collect();
    let z_mins: Vec<f64> = xs_list.iter().map(|xs| xs.y.iter().cloned().fold(f64::INFINITY, f64::min)).collect();

    // DENSIFICATION STEP: Automatic Reach Interpolation
    let max_sp = inputs.max_spacing.map(|sp| {
        if raw_units == UnitSystem::USCustomary { sp * FT_TO_M } else { sp }
    });

    let mut densified_tables = Vec::new();
    let mut densified_z_mins = Vec::new();
    let mut densified_stations = Vec::new();
    let mut original_to_densified = Vec::new();

    for i in 0..m {
        let current_idx = densified_tables.len();
        original_to_densified.push(current_idx);

        densified_tables.push(tables[i].clone());
        densified_z_mins.push(z_mins[i]);
        densified_stations.push(xs_list[i].station);

        if i < m - 1 {
            let dx = xs_list[i].station - xs_list[i + 1].station;
            if let Some(limit) = max_sp {
                if limit > 0.0 && dx > limit {
                    let num_spaces = (dx / limit).ceil() as usize;
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
                        
                        densified_tables.push(t_interp);
                        densified_z_mins.push(z_interp);
                        densified_stations.push(s_interp);
                    }
                }
            }
        }
    }

    let dm = densified_tables.len();

    // Calculate critical depths and elevations for the densified grid
    let ycs: Vec<f64> = densified_tables.iter().map(|table| solve_critical_depth_table(table, q)).collect();
    let critical_wsels: Vec<f64> = densified_z_mins.iter().zip(&ycs).map(|(&z, &yc)| z + yc).collect();

    let regime = inputs.regime; // 0=Subcritical, 1=Supercritical, 2=Mixed
    let mut wsel_metric = vec![0.0; dm];

    // Boundary conditions in metric
    let ds_bc = inputs.downstream_wsel.map(|w| if raw_units == UnitSystem::USCustomary { w * FT_TO_M } else { w });
    let us_bc = inputs.upstream_wsel.map(|w| if raw_units == UnitSystem::USCustomary { w * FT_TO_M } else { w });

    // SWEEP 1: SUBCRITICAL (Downstream to Upstream)
    let mut sub_wsel = vec![0.0; dm];
    if regime == 0 || regime == 2 {
        sub_wsel[dm - 1] = ds_bc.unwrap_or(critical_wsels[dm - 1]);
        if sub_wsel[dm - 1] < critical_wsels[dm - 1] {
            sub_wsel[dm - 1] = critical_wsels[dm - 1];
        }

        for i in (0..dm - 1).rev() {
            let length = densified_stations[i] - densified_stations[i + 1];

            // Check if there is a bridge in this reach interval
            let mut bridge_idx = None;
            if let Some(ref b_stations) = inputs.bridge_stations {
                for (b_idx, &b_st) in b_stations.iter().enumerate() {
                    let b_st_metric = if raw_units == UnitSystem::USCustomary {
                        b_st * FT_TO_M
                    } else {
                        b_st
                    };
                    if b_st_metric >= densified_stations[i + 1] - 1e-4
                        && b_st_metric < densified_stations[i] + 1e-4
                    {
                        bridge_idx = Some(b_idx);
                        break;
                    }
                }
            }

            // Check if there is a culvert in this reach interval
            let mut culvert_idx = None;
            if let Some(ref c_stations) = inputs.culvert_stations {
                for (c_idx, &c_st) in c_stations.iter().enumerate() {
                    let c_st_metric = if raw_units == UnitSystem::USCustomary {
                        c_st * FT_TO_M
                    } else {
                        c_st
                    };
                    if c_st_metric >= densified_stations[i + 1] - 1e-4
                        && c_st_metric < densified_stations[i] + 1e-4
                    {
                        culvert_idx = Some(c_idx);
                        break;
                    }
                }
            }

            if let Some(b_idx) = bridge_idx {
                let low_chord = inputs.bridge_low_chords.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0);
                let high_chord = inputs.bridge_high_chords.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0);
                let pier_width = inputs.bridge_pier_widths.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.0);
                let num_piers = inputs.bridge_num_piers.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0);
                let pier_shape = inputs.bridge_pier_shapes.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0);
                let weir_coeff = inputs.bridge_weir_coeffs.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(if raw_units == UnitSystem::USCustomary { 2.6 } else { 1.44 });
                let orifice_coeff = inputs.bridge_orifice_coeffs.as_ref().and_then(|v| v.get(b_idx)).copied().unwrap_or(0.5);

                let tw_wsel_user = if raw_units == UnitSystem::USCustomary {
                    sub_wsel[i + 1] / FT_TO_M
                } else {
                    sub_wsel[i + 1]
                };
                let z_down_user = if raw_units == UnitSystem::USCustomary {
                    densified_z_mins[i + 1] / FT_TO_M
                } else {
                    densified_z_mins[i + 1]
                };
                let z_up_user = if raw_units == UnitSystem::USCustomary {
                    densified_z_mins[i] / FT_TO_M
                } else {
                    densified_z_mins[i]
                };

                let wsel_up_user = crate::solvers::bridge::solve_bridge_wsel(
                    inputs.flow_rate,
                    low_chord,
                    high_chord,
                    pier_width,
                    num_piers,
                    pier_shape,
                    weir_coeff,
                    orifice_coeff,
                    z_down_user,
                    z_up_user,
                    tw_wsel_user,
                    raw_units,
                    &densified_tables[i],
                    &densified_tables[i + 1],
                );

                sub_wsel[i] = if raw_units == UnitSystem::USCustomary {
                    wsel_up_user * FT_TO_M
                } else {
                    wsel_up_user
                };
            } else if let Some(c_idx) = culvert_idx {
                let shape_type = inputs.culvert_shape_types.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0);
                let span = inputs.culvert_spans.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(4.0);
                let rise = inputs.culvert_rises.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(4.0);
                let roughness_n = inputs.culvert_roughness_ns.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.013);
                let culv_len = inputs.culvert_lengths.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(100.0);
                let entrance_loss_coeff = inputs.culvert_entrance_loss_coeffs.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(0.5);
                let exit_loss_coeff = inputs.culvert_exit_loss_coeffs.as_ref().and_then(|v| v.get(c_idx)).copied().unwrap_or(1.0);

                let tw_wsel_user = if raw_units == UnitSystem::USCustomary {
                    sub_wsel[i + 1] / FT_TO_M
                } else {
                    sub_wsel[i + 1]
                };
                let z_down_user = if raw_units == UnitSystem::USCustomary {
                    densified_z_mins[i + 1] / FT_TO_M
                } else {
                    densified_z_mins[i + 1]
                };
                let z_up_user = if raw_units == UnitSystem::USCustomary {
                    densified_z_mins[i] / FT_TO_M
                } else {
                    densified_z_mins[i]
                };

                let wsel_up_user = crate::solvers::culvert::solve_culvert_wsel(
                    inputs.flow_rate,
                    shape_type,
                    span,
                    rise,
                    roughness_n,
                    culv_len,
                    entrance_loss_coeff,
                    exit_loss_coeff,
                    z_down_user,
                    z_up_user,
                    tw_wsel_user,
                    raw_units,
                );

                sub_wsel[i] = if raw_units == UnitSystem::USCustomary {
                    wsel_up_user * FT_TO_M
                } else {
                    wsel_up_user
                };
            } else {
                sub_wsel[i] = solve_step(
                    &densified_tables[i + 1],
                    sub_wsel[i + 1],
                    &densified_tables[i],
                    densified_z_mins[i],
                    ycs[i],
                    q,
                    length,
                    c_contraction,
                    c_expansion,
                    true,
                ).unwrap_or(critical_wsels[i]);
            }
        }
    }

    // SWEEP 2: SUPERCRITICAL (Upstream to Downstream)
    let mut super_wsel = vec![0.0; dm];
    if regime == 1 || regime == 2 {
        super_wsel[0] = us_bc.unwrap_or(critical_wsels[0]);
        if super_wsel[0] > critical_wsels[0] {
            super_wsel[0] = critical_wsels[0];
        }

        for i in 0..dm - 1 {
            let length = densified_stations[i] - densified_stations[i + 1];

            // Check if there is a bridge in this reach interval
            let mut bridge_idx = None;
            if let Some(ref b_stations) = inputs.bridge_stations {
                for (b_idx, &b_st) in b_stations.iter().enumerate() {
                    let b_st_metric = if raw_units == UnitSystem::USCustomary {
                        b_st * FT_TO_M
                    } else {
                        b_st
                    };
                    if b_st_metric >= densified_stations[i + 1] - 1e-4
                        && b_st_metric < densified_stations[i] + 1e-4
                    {
                        bridge_idx = Some(b_idx);
                        break;
                    }
                }
            }

            // Check if there is a culvert in this reach interval
            let mut culvert_idx = None;
            if let Some(ref c_stations) = inputs.culvert_stations {
                for (c_idx, &c_st) in c_stations.iter().enumerate() {
                    let c_st_metric = if raw_units == UnitSystem::USCustomary {
                        c_st * FT_TO_M
                    } else {
                        c_st
                    };
                    if c_st_metric >= densified_stations[i + 1] - 1e-4
                        && c_st_metric < densified_stations[i] + 1e-4
                    {
                        culvert_idx = Some(c_idx);
                        break;
                    }
                }
            }

            if bridge_idx.is_some() || culvert_idx.is_some() {
                super_wsel[i + 1] = critical_wsels[i + 1];
            } else {
                super_wsel[i + 1] = solve_step(
                    &densified_tables[i],
                    super_wsel[i],
                    &densified_tables[i + 1],
                    densified_z_mins[i + 1],
                    ycs[i + 1],
                    q,
                    length,
                    c_contraction,
                    c_expansion,
                    false,
                ).unwrap_or(critical_wsels[i + 1]);
            }
        }
    }

    // REGIME SELECTION / MIXED REGIME SOLVING
    if regime == 0 {
        wsel_metric = sub_wsel;
    } else if regime == 1 {
        wsel_metric = super_wsel;
    } else {
        // Mixed regime selection
        for i in 0..dm {
            let sub_m = densified_tables[i].calculate_specific_force(sub_wsel[i], q);
            let super_m = densified_tables[i].calculate_specific_force(super_wsel[i], q);
            if sub_m >= super_m {
                wsel_metric[i] = sub_wsel[i];
            } else {
                wsel_metric[i] = super_wsel[i];
            }
        }
    }

    // POST-PROCESSING: Calculate outputs for original sections and convert back to user units
    let mut out_wsel = vec![0.0; m];
    let mut out_yc = vec![0.0; m];
    let mut out_vel = vec![0.0; m];
    let mut out_area = vec![0.0; m];
    let mut out_fr = vec![0.0; m];

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

    for orig_idx in 0..m {
        let sorted_xs_idx = original_mapping[orig_idx];
        let sorted_idx = original_to_densified[sorted_xs_idx];
        
        let wsel_val = wsel_metric[sorted_idx];
        let yc_val = critical_wsels[sorted_idx];
        let table = &densified_tables[sorted_idx];
        let row = table.interpolate(wsel_val);

        let velocity = if row.area > 1e-6 { q / row.area } else { 0.0 };
        let froude = if row.area > 1e-6 && row.top_width > 1e-6 {
            let d_hydraulic = row.area / row.top_width;
            velocity / (G_METRIC * d_hydraulic).sqrt()
        } else {
            0.0
        };

        if raw_units == UnitSystem::USCustomary {
            out_wsel[orig_idx] = wsel_val / FT_TO_M;
            out_yc[orig_idx] = yc_val / FT_TO_M;
            out_vel[orig_idx] = velocity / FT_TO_M;
            out_area[orig_idx] = row.area / (FT_TO_M * FT_TO_M);
        } else {
            out_wsel[orig_idx] = wsel_val;
            out_yc[orig_idx] = yc_val;
            out_vel[orig_idx] = velocity;
            out_area[orig_idx] = row.area;
        }
        out_fr[orig_idx] = froude;
    }

    SteadyResult {
        wsel: out_wsel,
        critical_wsel: out_yc,
        velocity: out_vel,
        area: out_area,
        froude: out_fr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steady_critical_depth() {
        // Rectangular channel: bottom width = 10m. Q = 20 cms.
        // Analytical yc = (Q^2 / (g * B^2))^(1/3)
        // yc = (20^2 / (9.80665 * 10^2))^(1/3) = (400 / 980.665)^(1/3) = (0.407886)^0.33333 = 0.7416 m.
        let xs = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
        };

        let table = xs.generate_lookup_table(10);
        let yc = solve_critical_depth(&xs, &table, 20.0);
        assert!((yc - 0.7416).abs() < 1e-3, "yc was {}", yc);
    }

    #[test]
    fn test_steady_subcritical_profile() {
        // Set up 3 identical cross-sections spaced 100m apart.
        // Rectangular channel: width = 10m, Manning's n = 0.02.
        // Stationing: 200, 100, 0.
        // Slope = 0.001 (bottom elevations: 0.2m, 0.1m, 0.0m).
        // Flow rate Q = 15.0 cms.
        let xs200 = CrossSection {
            station: 200.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0 + 0.2, 0.2, 0.2, 5.0 + 0.2],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
        };
        let xs100 = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0 + 0.1, 0.1, 0.1, 5.0 + 0.1],
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

        let inputs = SteadyInputs {
            cross_sections: vec![xs200, xs100, xs0],
            flow_rate: 15.0,
            num_slices: Some(50),
            coeff_contraction: None,
            coeff_expansion: None,
            regime: 0, // Subcritical
            downstream_wsel: Some(1.2), // high tailwater boundary, creating backwater
            upstream_wsel: None,
            max_spacing: None,
            culvert_stations: None,
            culvert_shape_types: None,
            culvert_spans: None,
            culvert_rises: None,
            culvert_roughness_ns: None,
            culvert_lengths: None,
            culvert_entrance_loss_coeffs: None,
            culvert_exit_loss_coeffs: None,
            ..Default::default()
        };

        let result = solve_steady(&inputs);
        
        // Assertions
        // At station 0 (index 2 in inputs, but solver handles mapping back to match inputs array ordering)
        assert_eq!(result.wsel[2], 1.2);
        // At station 100 (index 1), WSEL should be higher than at station 0 but slope is lower than bed slope
        // Because backwater curve is M1, water depth decreases as you go upstream (depth at 0 is 1.2, depth at 100 should be < 1.2 - 0.1 = 1.1)
        let depth0 = result.wsel[2] - 0.0;
        let depth100 = result.wsel[1] - 0.1;
        let depth200 = result.wsel[0] - 0.2;
        
        assert!(depth100 < depth0, "depth100={} depth0={}", depth100, depth0);
        assert!(depth200 < depth100, "depth200={} depth100={}", depth200, depth100);
        
        // Froude number should be < 1.0 (subcritical)
        for &fr in &result.froude {
            assert!(fr < 1.0, "Froude was {}", fr);
        }
    }

    #[test]
    fn test_steady_reach_densification() {
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
        let inputs = SteadyInputs {
            cross_sections: vec![xs1000, xs0],
            flow_rate: 15.0,
            num_slices: Some(50),
            coeff_contraction: None,
            coeff_expansion: None,
            regime: 0, // Subcritical
            downstream_wsel: Some(1.2), // tailwater depth = 1.2m
            upstream_wsel: None,
            max_spacing: Some(100.0),
            culvert_stations: None,
            culvert_shape_types: None,
            culvert_spans: None,
            culvert_rises: None,
            culvert_roughness_ns: None,
            culvert_lengths: None,
            culvert_entrance_loss_coeffs: None,
            culvert_exit_loss_coeffs: None,
            ..Default::default()
        };

        let result = solve_steady(&inputs);

        // Verification
        // The solver should converge successfully. Check that output size matches original input size (2)
        assert_eq!(result.wsel.len(), 2);
        // Downstream boundary condition is preserved
        assert_eq!(result.wsel[1], 1.2);
        // Upstream water surface elevation should be solved successfully and be greater than bed level (1.0m)
        assert!(result.wsel[0] > 1.0);
    }

    #[test]
    fn test_steady_integrated_culvert() {
        // Concrete circular pipe: D = 5.0 ft, L = 100 ft, Q = 100 cfs, slope = 0.01
        // Channel reach with 3 sections at stations 200, 100, and 0 in US Customary.
        // Station 100 is just upstream of the culvert inlet (which sits between 100 and 0).
        let xs200 = CrossSection {
            station: 200.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0 + 2.0, 2.0, 2.0, 10.0 + 2.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
        };
        let xs100 = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0 + 1.0, 1.0, 1.0, 10.0 + 1.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0, 0.0, 0.0, 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
        };

        let inputs = SteadyInputs {
            cross_sections: vec![xs200, xs100, xs0],
            flow_rate: 100.0,
            num_slices: Some(50),
            coeff_contraction: None,
            coeff_expansion: None,
            regime: 0, // Subcritical
            downstream_wsel: Some(3.0), // TW = 3 ft above invert
            upstream_wsel: None,
            max_spacing: None,
            culvert_stations: Some(vec![50.0]), // culvert located between 0 and 100 (at station 50)
            culvert_shape_types: Some(vec![0]), // Circular
            culvert_spans: Some(vec![5.0]),
            culvert_rises: Some(vec![5.0]),
            culvert_roughness_ns: Some(vec![0.012]),
            culvert_lengths: Some(vec![100.0]),
            culvert_entrance_loss_coeffs: Some(vec![0.5]),
            culvert_exit_loss_coeffs: Some(vec![1.0]),
            ..Default::default()
        };

        let result = solve_steady(&inputs);

        // Verification of integrated culvert model
        // Downstream section station 0 (index 2) WSEL is tailwater: 3.0 ft.
        assert_eq!(result.wsel[2], 3.0);

        // Upstream section station 100 (index 1) WSEL is solved by culvert inlet control (~1.0 + 4.25 = 5.25 ft).
        // Let's verify it matches to within 0.05 ft.
        let hw_wsel = result.wsel[1];
        assert!((hw_wsel - 5.25).abs() < 0.05, "expected ~5.25, got {}", hw_wsel);

        // Upstream section station 200 (index 0) WSEL is GVF solved starting from station 100's solved WSEL.
        assert!(result.wsel[0] > 2.0);
    }

    #[test]
    fn test_steady_culvert_sensitivity() {
        let xs200 = CrossSection {
            station: 200.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![12.0, 2.0, 2.0, 12.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
        };
        let xs100 = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![11.0, 1.0, 1.0, 11.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0, 0.0, 0.0, 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::USCustomary,
        };

        let mut inputs = SteadyInputs {
            cross_sections: vec![xs200, xs100, xs0],
            flow_rate: 100.0,
            num_slices: Some(50),
            coeff_contraction: None,
            coeff_expansion: None,
            regime: 0,
            downstream_wsel: Some(3.0),
            upstream_wsel: None,
            max_spacing: None,
            culvert_stations: Some(vec![50.0]),
            culvert_shape_types: Some(vec![1]), // Box
            culvert_spans: Some(vec![8.0]),
            culvert_rises: Some(vec![6.0]),
            culvert_roughness_ns: Some(vec![0.012]),
            culvert_lengths: Some(vec![100.0]),
            culvert_entrance_loss_coeffs: Some(vec![0.5]),
            culvert_exit_loss_coeffs: Some(vec![1.0]),
            ..Default::default()
        };

        let result_wide = solve_steady(&inputs);
        
        inputs.culvert_spans = Some(vec![0.1]);
        let result_narrow = solve_steady(&inputs);

        println!("Wide WSEL: {:?}", result_wide.wsel);
        println!("Narrow WSEL: {:?}", result_narrow.wsel);
        assert!(
            result_narrow.wsel[1] > result_wide.wsel[1],
            "Narrow culvert WSEL ({}) should be greater than wide culvert WSEL ({})",
            result_narrow.wsel[1],
            result_wide.wsel[1]
        );
    }

    #[test]
    fn test_steady_integrated_bridge() {
        // Simple reach: stations 200, 100, 0
        // Rectangular channel: width = 10m
        // Bed elevations: 0.2m, 0.1m, 0.0m
        // Flow rate: 15.0 cms
        let xs200 = CrossSection {
            station: 200.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0 + 0.2, 0.2, 0.2, 10.0 + 0.2],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
        };
        let xs100 = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0 + 0.1, 0.1, 0.1, 10.0 + 0.1],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
        };
        let xs0 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![10.0, 0.0, 0.0, 10.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::Metric,
        };

        let inputs = SteadyInputs {
            cross_sections: vec![xs200, xs100, xs0],
            flow_rate: 15.0,
            num_slices: Some(50),
            coeff_contraction: None,
            coeff_expansion: None,
            regime: 0,
            downstream_wsel: Some(3.0),
            upstream_wsel: None,
            max_spacing: None,
            bridge_stations: Some(vec![50.0]), // bridge at station 50 (between 0 and 100)
            bridge_low_chords: Some(vec![5.0]),
            bridge_high_chords: Some(vec![7.0]),
            bridge_pier_widths: Some(vec![0.5]),
            bridge_num_piers: Some(vec![2]),
            bridge_pier_shapes: Some(vec![0]),
            bridge_weir_coeffs: Some(vec![1.44]),
            bridge_orifice_coeffs: Some(vec![0.5]),
            ..Default::default()
        };

        let result = solve_steady(&inputs);

        // Verification
        // WSEL at station 0 (index 2) should be 3.0 (downstream boundary)
        assert_eq!(result.wsel[2], 3.0);
        // WSEL at station 100 (index 1) should be solved by the bridge code
        // Since tw=3.0, it is low flow (below low-chord 5.0). So it includes Yarnell head loss.
        // Let's verify it solved WSEL and it is > downstream bed + tailwater depth
        assert!(result.wsel[1] > 3.0, "WSEL upstream of bridge should have backwater, got {}", result.wsel[1]);
    }
}

