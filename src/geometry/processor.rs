use crate::utils::{UnitSystem, FT_TO_M};

/// A raw cross-section definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrossSection {
    /// Station location along the river reach (e.g. downstream is 0, upstream is positive).
    pub station: f64,
    /// X stations across the channel (lateral coordinates).
    pub x: Vec<f64>,
    /// Y elevations of the channel profile.
    pub y: Vec<f64>,
    /// Station points where Manning's n changes.
    pub n_stations: Vec<f64>,
    /// Manning's n values corresponding to each interval.
    pub n_values: Vec<f64>,
    /// The unit system used in these raw inputs.
    pub unit_system: UnitSystem,
    /// Optional overbank flags corresponding to each coordinate point.
    pub is_overbank: Option<Vec<bool>>,
}

/// A single row in the hydraulic lookup table.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct GeometryRow {
    pub elevation: f64,
    pub area: f64,
    pub perimeter: f64,
    pub top_width: f64,
    pub conveyance: f64,
    pub channel_area: f64,
}

/// A lookup table mapping elevation to geometric properties.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeometryTable {
    pub rows: Vec<GeometryRow>,
}

impl CrossSection {
    /// Converts this cross-section to Metric (SI) units internally.
    /// If already metric, returns a clone.
    pub fn to_metric(&self) -> Self {
        if self.unit_system == UnitSystem::Metric {
            return self.clone();
        }

        let x_metric = self.x.iter().map(|&val| val * FT_TO_M).collect();
        let y_metric = self.y.iter().map(|&val| val * FT_TO_M).collect();
        let n_stations_metric = self.n_stations.iter().map(|&val| val * FT_TO_M).collect();

        Self {
            station: self.station * FT_TO_M,
            x: x_metric,
            y: y_metric,
            n_stations: n_stations_metric,
            n_values: self.n_values.clone(),
            unit_system: UnitSystem::Metric,
            is_overbank: self.is_overbank.clone(),
        }
    }

    /// Looks up the Manning's n value at a given lateral coordinate (station).
    pub fn get_manning_n(&self, x_coord: f64) -> f64 {
        if self.n_stations.is_empty() {
            return 0.035; // Default fallback
        }
        if x_coord <= self.n_stations[0] {
            return self.n_values[0];
        }
        let mut active_n = self.n_values[0];
        for (i, &st) in self.n_stations.iter().enumerate() {
            if x_coord >= st {
                active_n = self.n_values[i];
            } else {
                break;
            }
        }
        active_n
    }

    /// Generates a sorted list of unique elevations for the slicing lookup table.
    /// Combines uniform spacing steps with the actual vertices of the channel profile.
    pub fn get_slicing_elevations(&self, num_uniform_slices: usize) -> Vec<f64> {
        if self.y.is_empty() {
            return vec![];
        }

        let mut y_min = self.y[0];
        let mut y_max = self.y[0];
        for &val in &self.y {
            if val < y_min {
                y_min = val;
            }
            if val > y_max {
                y_max = val;
            }
        }

        let mut elevations = Vec::new();

        // 1. Uniform slices
        if num_uniform_slices > 1 && y_max > y_min {
            let step = (y_max - y_min) / (num_uniform_slices - 1) as f64;
            for i in 0..num_uniform_slices {
                elevations.push(y_min + i as f64 * step);
            }
        } else {
            elevations.push(y_min);
            if y_max > y_min {
                elevations.push(y_max);
            }
        }

        // 2. Add raw vertices
        for &val in &self.y {
            elevations.push(val);
        }

        // 3. Add elevations at Manning's n break points (interpolated from profile)
        for &n_st in &self.n_stations {
            if let Some(y_interp) = self.interpolate_y_at_x(n_st) {
                elevations.push(y_interp);
            }
        }

        // Sort
        elevations.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Deduplicate with small tolerance (e.g. 1e-6 meters)
        let mut unique_elevations = Vec::new();
        if !elevations.is_empty() {
            unique_elevations.push(elevations[0]);
            for &el in &elevations[1..] {
                if (el - unique_elevations.last().unwrap()).abs() > 1e-6 {
                    unique_elevations.push(el);
                }
            }
        }

        unique_elevations
    }

    /// Linearly interpolates the profile elevation (Y) at a given station (X).
    fn interpolate_y_at_x(&self, x_target: f64) -> Option<f64> {
        let n_pts = self.x.len();
        if n_pts < 2 {
            return None;
        }

        if x_target <= self.x[0] {
            return Some(self.y[0]);
        }
        if x_target >= self.x[n_pts - 1] {
            return Some(self.y[n_pts - 1]);
        }

        for i in 0..n_pts - 1 {
            let x1 = self.x[i];
            let x2 = self.x[i + 1];
            if x_target >= x1 && x_target <= x2 {
                let dx = x2 - x1;
                if dx.abs() < 1e-9 {
                    return Some(self.y[i]);
                }
                let t = (x_target - x1) / dx;
                return Some(self.y[i] + t * (self.y[i + 1] - self.y[i]));
            }
        }
        None
    }

    /// Computes the hydraulic properties for a specific water surface elevation.
    /// All calculations must be in Metric (SI).
    pub fn compute_properties_at_elevation(&self, elev: f64) -> GeometryRow {
        let n_pts = self.x.len();
        if n_pts < 2 || elev <= self.y.iter().cloned().fold(f64::INFINITY, f64::min) {
            return GeometryRow {
                elevation: elev,
                area: 0.0,
                perimeter: 0.0,
                top_width: 0.0,
                conveyance: 0.0,
                channel_area: 0.0,
            };
        }

        struct ZoneProperties {
            area: f64,
            perimeter: f64,
            top_width: f64,
            sum_pn15: f64,
        }

        let mut lob = ZoneProperties { area: 0.0, perimeter: 0.0, top_width: 0.0, sum_pn15: 0.0 };
        let mut ch  = ZoneProperties { area: 0.0, perimeter: 0.0, top_width: 0.0, sum_pn15: 0.0 };
        let mut rob = ZoneProperties { area: 0.0, perimeter: 0.0, top_width: 0.0, sum_pn15: 0.0 };

        let mut is_subdivided = false;
        let mut left_bank_x = 0.0;
        let mut right_bank_x = 0.0;

        if let Some(ref overbank_flags) = self.is_overbank {
            if overbank_flags.len() == n_pts {
                let first_false = overbank_flags.iter().position(|&flag| !flag);
                let last_false = overbank_flags.iter().rposition(|&flag| !flag);
                if let (Some(l_idx), Some(r_idx)) = (first_false, last_false) {
                    left_bank_x = self.x[l_idx];
                    right_bank_x = self.x[r_idx];
                    is_subdivided = true;
                }
            }
        }

        for i in 0..n_pts - 1 {
            let x1 = self.x[i];
            let y1 = self.y[i];
            let x2 = self.x[i + 1];
            let y2 = self.y[i + 1];

            let y_min_seg = y1.min(y2);
            let y_max_seg = y1.max(y2);

            // Segment is entirely above water level
            if elev <= y_min_seg {
                continue;
            }

            let (xa, ya, xb, yb) = if elev >= y_max_seg {
                // Segment is entirely submerged
                (x1, y1, x2, y2)
            } else {
                // Segment is partially submerged
                let t1 = (elev - y1) / (y2 - y1);
                let x_int = x1 + t1 * (x2 - x1);
                if y1 < y2 {
                    (x1, y1, x_int, elev)
                } else {
                    (x_int, elev, x2, y2)
                }
            };

            let seg_width = (xb - xa).abs();
            let seg_height_a = elev - ya;
            let seg_height_b = elev - yb;

            // Wetted length along the channel boundary (excluding the water surface)
            let seg_wetted_len = (seg_width * seg_width + (yb - ya) * (yb - ya)).sqrt();

            // Area contribution (trapezoid of water volume)
            let seg_area = 0.5 * (seg_height_a + seg_height_b) * seg_width;

            // Midpoint of submerged segment to look up Manning's n
            let x_mid = 0.5 * (xa + xb);
            let n_val = self.get_manning_n(x_mid);

            let sum_pn15_contrib = seg_wetted_len * n_val.powf(1.5);

            if is_subdivided {
                if x_mid < left_bank_x {
                    lob.area += seg_area;
                    lob.perimeter += seg_wetted_len;
                    lob.top_width += seg_width;
                    lob.sum_pn15 += sum_pn15_contrib;
                } else if x_mid > right_bank_x {
                    rob.area += seg_area;
                    rob.perimeter += seg_wetted_len;
                    rob.top_width += seg_width;
                    rob.sum_pn15 += sum_pn15_contrib;
                } else {
                    ch.area += seg_area;
                    ch.perimeter += seg_wetted_len;
                    ch.top_width += seg_width;
                    ch.sum_pn15 += sum_pn15_contrib;
                }
            } else {
                ch.area += seg_area;
                ch.perimeter += seg_wetted_len;
                ch.top_width += seg_width;
                ch.sum_pn15 += sum_pn15_contrib;
            }
        }

        let area = lob.area + ch.area + rob.area;
        let perimeter = lob.perimeter + ch.perimeter + rob.perimeter;
        let top_width = lob.top_width + ch.top_width + rob.top_width;

        let get_conveyance = |zone: &ZoneProperties| -> f64 {
            if zone.perimeter > 1e-9 {
                let comp_n = (zone.sum_pn15 / zone.perimeter).powf(2.0 / 3.0);
                if comp_n > 1e-9 {
                    let r = zone.area / zone.perimeter;
                    (1.0 / comp_n) * zone.area * r.powf(2.0 / 3.0)
                } else {
                    0.0
                }
            } else {
                0.0
            }
        };

        let conveyance = if is_subdivided {
            get_conveyance(&lob) + get_conveyance(&ch) + get_conveyance(&rob)
        } else {
            get_conveyance(&ch)
        };

        GeometryRow {
            elevation: elev,
            area,
            perimeter,
            top_width,
            conveyance,
            channel_area: ch.area,
        }
    }

    /// Generates the full lookup table for this cross-section.
    pub fn generate_lookup_table(&self, num_uniform_slices: usize) -> GeometryTable {
        // Build metric equivalent of self
        let metric_xs = self.to_metric();
        let elevs = metric_xs.get_slicing_elevations(num_uniform_slices);

        let mut rows = Vec::new();
        for &el in &elevs {
            rows.push(metric_xs.compute_properties_at_elevation(el));
        }

        GeometryTable { rows }
    }
}

impl GeometryTable {
    /// Linearly interpolates hydraulic properties at a given elevation.
    /// Input elevation must be in Metric. Returns Metric properties.
    pub fn interpolate(&self, elev: f64) -> GeometryRow {
        let n_rows = self.rows.len();
        if n_rows == 0 {
            return GeometryRow {
                elevation: elev,
                area: 0.0,
                perimeter: 0.0,
                top_width: 0.0,
                conveyance: 0.0,
                channel_area: 0.0,
            };
        }

        // Clamp to minimum elevation corresponding to a minimum depth of 0.05 meters (stabilization)
        let min_elev = self.rows[0].elevation + 0.05;
        let target_elev = elev.max(min_elev);

        // Clamp to highest elevation if above maximum
        if target_elev >= self.rows[n_rows - 1].elevation {
            let last = self.rows[n_rows - 1];
            // Extrapolate area and top width based on last top width
            let dy = target_elev - last.elevation;
            let new_area = last.area + last.top_width * dy;
            return GeometryRow {
                elevation: target_elev,
                area: new_area,
                perimeter: last.perimeter + 2.0 * dy, // Simple boundary wall extension
                top_width: last.top_width,
                conveyance: last.conveyance, // conservative approximation
                channel_area: last.channel_area + last.top_width * dy,
            };
        }

        // Binary search for interval
        let mut low = 0;
        let mut high = n_rows - 1;
        while high - low > 1 {
            let mid = (low + high) / 2;
            if self.rows[mid].elevation <= target_elev {
                low = mid;
            } else {
                high = mid;
            }
        }

        let r1 = self.rows[low];
        let r2 = self.rows[high];

        let dy = r2.elevation - r1.elevation;
        if dy.abs() < 1e-9 {
            return r1;
        }

        let t = (target_elev - r1.elevation) / dy;

        GeometryRow {
            elevation: target_elev,
            area: r1.area + t * (r2.area - r1.area),
            perimeter: r1.perimeter + t * (r2.perimeter - r1.perimeter),
            top_width: r1.top_width + t * (r2.top_width - r1.top_width),
            conveyance: r1.conveyance + t * (r2.conveyance - r1.conveyance),
            channel_area: r1.channel_area + t * (r2.channel_area - r1.channel_area),
        }
    }
}

/// Interpolates a new GeometryTable between table1 (at bed z1) and table2 (at bed z2) at interpolation factor t (0.0 to 1.0).
pub fn interpolate_geometry_table(
    table1: &GeometryTable,
    z1: f64,
    table2: &GeometryTable,
    z2: f64,
    t: f64,
    num_slices: usize,
) -> (GeometryTable, f64) {
    let z_interp = (1.0 - t) * z1 + t * z2;

    // Find maximum depth of both sections
    let max_d1 = table1.rows.last().map(|r| r.elevation - z1).unwrap_or(10.0);
    let max_d2 = table2.rows.last().map(|r| r.elevation - z2).unwrap_or(10.0);
    let max_d = max_d1.max(max_d2);

    let mut rows = Vec::new();
    let step = max_d / (num_slices - 1) as f64;

    for i in 0..num_slices {
        let depth = i as f64 * step;

        let row1 = table1.interpolate(z1 + depth);
        let row2 = table2.interpolate(z2 + depth);

        rows.push(GeometryRow {
            elevation: z_interp + depth,
            area: (1.0 - t) * row1.area + t * row2.area,
            perimeter: (1.0 - t) * row1.perimeter + t * row2.perimeter,
            top_width: (1.0 - t) * row1.top_width + t * row2.top_width,
            conveyance: (1.0 - t) * row1.conveyance + t * row2.conveyance,
            channel_area: (1.0 - t) * row1.channel_area + t * row2.channel_area,
        });
    }

    (GeometryTable { rows }, z_interp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geometry_table_interpolation() {
        // Table 1: Rectangular channel 10m wide, bed z1 = 1.0m
        let xs1 = CrossSection {
            station: 1000.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![6.0, 1.0, 1.0, 6.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
        };
        let table1 = xs1.generate_lookup_table(10);

        // Table 2: Rectangular channel 20m wide, bed z2 = 0.0m
        let xs2 = CrossSection {
            station: 0.0,
            x: vec![0.0, 0.0, 20.0, 20.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
        };
        let table2 = xs2.generate_lookup_table(10);

        // Interpolate at t = 0.5 (midpoint)
        let (table_interp, z_interp) = interpolate_geometry_table(&table1, 1.0, &table2, 0.0, 0.5, 50);

        // Bed elevation should be 0.5m
        assert_eq!(z_interp, 0.5);

        // Query at depth 1.0m (absolute elevation z_interp + 1.0 = 1.5m)
        // Expected width = 15m. Expected area = 15.0 m2. Expected perimeter = 15 + 1 + 1 = 17m.
        let row = table_interp.interpolate(1.5);
        assert!((row.area - 15.0).abs() < 1e-2, "Area was {}", row.area);
        assert!((row.perimeter - 17.0).abs() < 1e-2, "Perimeter was {}", row.perimeter);
        assert!((row.top_width - 15.0).abs() < 1e-2, "Top width was {}", row.top_width);
    }

    #[test]
    fn test_rectangular_channel() {
        // A rectangular channel, 10 meters wide, bottom elevation 0.0.
        // Left wall at X=0, bottom from X=0 to X=10, right wall at X=10.
        let xs = CrossSection {
            station: 100.0,
            x: vec![0.0, 0.0, 10.0, 10.0],
            y: vec![5.0, 0.0, 0.0, 5.0],
            n_stations: vec![0.0],
            n_values: vec![0.02],
            unit_system: UnitSystem::Metric,
            is_overbank: None,
        };

        // Generating a table
        let table = xs.generate_lookup_table(10);
        
        // At y = 2.0, area should be 20.0, perimeter should be 10 + 2 + 2 = 14.0, top width = 10.0
        let row = table.interpolate(2.0);
        assert!((row.area - 20.0).abs() < 1e-3, "Area was {}", row.area);
        assert!((row.perimeter - 14.0).abs() < 1e-3, "Perimeter was {}", row.perimeter);
        assert!((row.top_width - 10.0).abs() < 1e-3, "Top width was {}", row.top_width);
    }

    #[test]
    fn test_trapezoidal_channel_us_units() {
        // Trapezoidal channel in US Customary units:
        // Bottom width = 20 ft, side slopes 2:1 (H:V)
        // Bottom elevation = 10 ft.
        // At WSEL = 15 ft (depth = 5 ft):
        // Area = (20 + 2 * side_slope * depth) * depth / 2 ? No, standard formula: Area = (W + side_slope * depth) * depth
        // Area = (20 + 2 * 5) * 5 = 150 ft^2.
        // In metric:
        // Bottom width = 20 * 0.3048 = 6.096 m
        // Depth = 5 * 0.3048 = 1.524 m
        // side slope = 2.0 (so width increases by 2 * 1.524 on each side, or top width = 6.096 + 2 * 2 * 1.524 = 12.192 m)
        // Area = (6.096 + 2.0 * 1.524) * 1.524 = 13.9354 m^2.
        // 150 ft^2 in m^2 = 150 * 0.3048^2 = 13.9354 m^2 (matches!).
        let xs = CrossSection {
            station: 50.0,
            x: vec![0.0, 20.0, 40.0, 60.0], // 10 ft wall down, bottom, wall up (representing 2:1 slope)
            y: vec![20.0, 10.0, 10.0, 20.0],
            n_stations: vec![0.0],
            n_values: vec![0.03],
            unit_system: UnitSystem::USCustomary,
            is_overbank: None,
        };

        let table = xs.generate_lookup_table(50);
        
        // Query at WSEL = 15 ft (which is 15 * FT_TO_M = 4.572 m)
        let row = table.interpolate(15.0 * FT_TO_M);
        
        let expected_area_m2 = 150.0 * FT_TO_M * FT_TO_M;
        let expected_perimeter_m = (20.0 + 2.0 * (5.0f64.powi(2) + 10.0f64.powi(2)).sqrt()) * FT_TO_M;
        let expected_top_width_m = 40.0 * FT_TO_M; // at y=15, width is 20 + 2 * 2 * 5 = 40 ft.
        
        assert!((row.area - expected_area_m2).abs() < 5e-3, "Area: expected {}, got {}", expected_area_m2, row.area);
        assert!((row.perimeter - expected_perimeter_m).abs() < 1e-4, "Perimeter: expected {}, got {}", expected_perimeter_m, row.perimeter);
        assert!((row.top_width - expected_top_width_m).abs() < 1e-4, "Top width: expected {}, got {}", expected_top_width_m, row.top_width);
    }
}

