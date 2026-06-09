//! Per-side bridge abutment geometry (HEC-RAS-style left/right abutments).

use crate::utils::{FT_TO_M, UnitSystem};

/// User/API abutment fields for one bridge (user units until resolved to metric).
#[derive(Debug, Clone, Default)]
pub struct BridgeAbutmentUserInput {
    /// Legacy total width blocked by left + right abutments (perpendicular to flow).
    pub legacy_total_width: f64,
    pub left_width: Option<f64>,
    pub right_width: Option<f64>,
    pub left_station: Option<f64>,
    pub right_station: Option<f64>,
    pub left_top_elevation: Option<f64>,
    pub right_top_elevation: Option<f64>,
    pub left_top_profile_stations: Option<Vec<f64>>,
    pub left_top_profile_elevations: Option<Vec<f64>>,
    pub right_top_profile_stations: Option<Vec<f64>>,
    pub right_top_profile_elevations: Option<Vec<f64>>,
}

/// Resolved abutment face in opening coordinates (metric, skew-adjusted width along opening).
#[derive(Debug, Clone)]
pub struct BridgeAbutmentSide {
    /// Left abutment: leftmost station. Right abutment: rightmost station.
    pub outer_station_m: f64,
    /// Width along opening (perpendicular input / cos(skew)).
    pub width_m: f64,
    pub top_elevation_m: Option<f64>,
    pub top_profile_stations_m: Vec<f64>,
    pub top_profile_elevations_m: Vec<f64>,
}

impl Default for BridgeAbutmentSide {
    fn default() -> Self {
        Self {
            outer_station_m: 0.0,
            width_m: 0.0,
            top_elevation_m: None,
            top_profile_stations_m: Vec::new(),
            top_profile_elevations_m: Vec::new(),
        }
    }
}

/// Left and right abutments at a bridge opening.
#[derive(Debug, Clone, Default)]
pub struct BridgeAbutments {
    pub left: Option<BridgeAbutmentSide>,
    pub right: Option<BridgeAbutmentSide>,
}

impl BridgeAbutments {
    pub fn is_configured(&self) -> bool {
        self.total_block_width_m() > 1e-6
    }

    pub fn total_block_width_m(&self) -> f64 {
        self.left_width_m() + self.right_width_m()
    }

    pub fn left_width_m(&self) -> f64 {
        self.left.as_ref().map(|s| s.width_m).unwrap_or(0.0)
    }

    pub fn right_width_m(&self) -> f64 {
        self.right.as_ref().map(|s| s.width_m).unwrap_or(0.0)
    }

    /// Symmetric legacy split for tests and backward compatibility.
    pub fn symmetric_total_width_m(total_width_m: f64, opening_s_min: f64, opening_s_max: f64) -> Self {
        if total_width_m <= 1e-6 {
            return Self::default();
        }
        let half = total_width_m * 0.5;
        Self {
            left: Some(BridgeAbutmentSide {
                outer_station_m: opening_s_min,
                width_m: half,
                ..Default::default()
            }),
            right: Some(BridgeAbutmentSide {
                outer_station_m: opening_s_max,
                width_m: half,
                ..Default::default()
            }),
        }
    }

    pub fn submerged_area_m2(&self, wsel_m: f64, z_bed_m: f64) -> f64 {
        let mut total = 0.0;
        if let Some(left) = &self.left {
            total += left.submerged_area_m2(wsel_m, z_bed_m, true);
        }
        if let Some(right) = &self.right {
            total += right.submerged_area_m2(wsel_m, z_bed_m, false);
        }
        total
    }

    /// Horizontal abutment width obstructing the water surface at `wsel_m`.
    pub fn submerged_width_at_wsel_m(&self, wsel_m: f64, z_bed_m: f64) -> f64 {
        let mut total = 0.0;
        if let Some(left) = &self.left {
            total += left.submerged_width_at_wsel_m(wsel_m, z_bed_m, true);
        }
        if let Some(right) = &self.right {
            total += right.submerged_width_at_wsel_m(wsel_m, z_bed_m, false);
        }
        total
    }
}

fn interpolate_profile(stations: &[f64], elevations: &[f64], station: f64) -> f64 {
    if stations.is_empty() {
        return 0.0;
    }
    if station <= stations[0] {
        return elevations[0];
    }
    if station >= stations[stations.len() - 1] {
        return elevations[elevations.len() - 1];
    }
    for i in 0..stations.len() - 1 {
        if station <= stations[i + 1] {
            let t = (station - stations[i]) / (stations[i + 1] - stations[i]);
            return elevations[i] + t * (elevations[i + 1] - elevations[i]);
        }
    }
    elevations[elevations.len() - 1]
}

impl BridgeAbutmentSide {
    /// Inclusive station span along the opening (left abutment grows rightward from outer face).
    pub fn station_span_m(&self, is_left: bool) -> (f64, f64) {
        if is_left {
            (self.outer_station_m, self.outer_station_m + self.width_m)
        } else {
            (self.outer_station_m - self.width_m, self.outer_station_m)
        }
    }

    fn has_top_profile(&self) -> bool {
        self.top_profile_stations_m.len() >= 2
            && self.top_profile_stations_m.len() == self.top_profile_elevations_m.len()
    }

    fn top_elevation_at_station_m(&self, station_m: f64) -> Option<f64> {
        if self.has_top_profile() {
            return Some(interpolate_profile(
                &self.top_profile_stations_m,
                &self.top_profile_elevations_m,
                station_m,
            ));
        }
        self.top_elevation_m
    }

    fn obstruction_depth_at(&self, wsel_m: f64, z_bed_m: f64, station_m: f64) -> f64 {
        match self.top_elevation_at_station_m(station_m) {
            Some(top) => (wsel_m - top).max(0.0),
            None => (wsel_m - z_bed_m).max(0.0),
        }
    }

    fn sample_stations_along_face(&self, s0: f64, s1: f64) -> Vec<f64> {
        let mut pts = vec![s0, s1];
        if self.has_top_profile() {
            for &s in &self.top_profile_stations_m {
                if s >= s0 - 1e-9 && s <= s1 + 1e-9 {
                    pts.push(s);
                }
            }
        }
        pts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        pts.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        pts
    }

    /// Submerged plan area for this abutment face (width × depth integrated per side/top profile).
    pub fn submerged_area_m2(&self, wsel_m: f64, z_bed_m: f64, is_left: bool) -> f64 {
        if self.width_m <= 1e-6 {
            return 0.0;
        }
        let (s0, s1) = self.station_span_m(is_left);
        let stations = self.sample_stations_along_face(s0, s1);
        if stations.len() < 2 {
            return 0.0;
        }
        let mut area = 0.0;
        for pair in stations.windows(2) {
            let sa = pair[0];
            let sb = pair[1];
            let ha = self.obstruction_depth_at(wsel_m, z_bed_m, sa);
            let hb = self.obstruction_depth_at(wsel_m, z_bed_m, sb);
            area += 0.5 * (ha + hb) * (sb - sa);
        }
        area
    }

    /// Width of this abutment intersecting the water surface at `wsel_m`.
    pub fn submerged_width_at_wsel_m(&self, wsel_m: f64, z_bed_m: f64, is_left: bool) -> f64 {
        if self.width_m <= 1e-6 {
            return 0.0;
        }
        let (s0, s1) = self.station_span_m(is_left);
        let stations = self.sample_stations_along_face(s0, s1);
        if stations.len() < 2 {
            return 0.0;
        }
        let mut width = 0.0;
        for pair in stations.windows(2) {
            let sa = pair[0];
            let sb = pair[1];
            let ha = self.obstruction_depth_at(wsel_m, z_bed_m, sa);
            let hb = self.obstruction_depth_at(wsel_m, z_bed_m, sb);
            if ha > 1e-6 || hb > 1e-6 {
                width += sb - sa;
            }
        }
        width
    }
}

fn width_along_opening_m(width_perpendicular: f64, skew_cos: f64) -> f64 {
    width_perpendicular / skew_cos.max(0.52)
}

fn to_metric_length(value: f64, units: UnitSystem) -> f64 {
    if units == UnitSystem::USCustomary {
        value * FT_TO_M
    } else {
        value
    }
}

fn build_side(
    width_perpendicular: f64,
    station: Option<f64>,
    top_elevation: Option<f64>,
    top_profile_stations: Option<Vec<f64>>,
    top_profile_elevations: Option<Vec<f64>>,
    default_outer_station: f64,
    units: UnitSystem,
    skew_cos: f64,
) -> Option<BridgeAbutmentSide> {
    if width_perpendicular <= 1e-6 {
        return None;
    }
    let to_m = |v: f64| to_metric_length(v, units);
    let profile_stations = top_profile_stations
        .filter(|v| v.len() >= 2)
        .map(|v| v.iter().map(|x| to_m(*x)).collect())
        .unwrap_or_default();
    let profile_elevations = top_profile_elevations
        .filter(|v| v.len() >= 2)
        .map(|v| v.iter().map(|x| to_m(*x)).collect())
        .unwrap_or_default();
    let top_elev_m = top_elevation.map(|e| to_m(e));
    Some(BridgeAbutmentSide {
        outer_station_m: station.map(to_m).unwrap_or(default_outer_station),
        width_m: width_along_opening_m(to_m(width_perpendicular), skew_cos),
        top_elevation_m: top_elev_m,
        top_profile_stations_m: profile_stations,
        top_profile_elevations_m: profile_elevations,
    })
}

/// Resolve per-side abutments from API input and opening bounds.
pub fn resolve_abutments(
    input: &BridgeAbutmentUserInput,
    opening_s_min_m: f64,
    opening_s_max_m: f64,
    skew_cos: f64,
    units: UnitSystem,
) -> BridgeAbutments {
    let per_side = input.left_width.is_some() || input.right_width.is_some();
    if per_side {
        let left_w = input.left_width.unwrap_or(0.0);
        let right_w = input.right_width.unwrap_or(0.0);
        return BridgeAbutments {
            left: build_side(
                left_w,
                input.left_station,
                input.left_top_elevation,
                input.left_top_profile_stations.clone(),
                input.left_top_profile_elevations.clone(),
                opening_s_min_m,
                units,
                skew_cos,
            ),
            right: build_side(
                right_w,
                input.right_station,
                input.right_top_elevation,
                input.right_top_profile_stations.clone(),
                input.right_top_profile_elevations.clone(),
                opening_s_max_m,
                units,
                skew_cos,
            ),
        };
    }

    if input.legacy_total_width > 1e-6 {
        let total_m = width_along_opening_m(to_metric_length(input.legacy_total_width, units), skew_cos);
        return BridgeAbutments::symmetric_total_width_m(total_m, opening_s_min_m, opening_s_max_m);
    }

    BridgeAbutments::default()
}

pub fn abutment_user_input_from_steady(
    legacy_total: Option<f64>,
    left_widths: Option<&Vec<f64>>,
    right_widths: Option<&Vec<f64>>,
    left_stations: Option<&Vec<f64>>,
    right_stations: Option<&Vec<f64>>,
    left_top_elevs: Option<&Vec<f64>>,
    right_top_elevs: Option<&Vec<f64>>,
    left_top_profile_stations: Option<&Vec<Vec<f64>>>,
    left_top_profile_elevations: Option<&Vec<Vec<f64>>>,
    right_top_profile_stations: Option<&Vec<Vec<f64>>>,
    right_top_profile_elevations: Option<&Vec<Vec<f64>>>,
    b_idx: usize,
) -> BridgeAbutmentUserInput {
    BridgeAbutmentUserInput {
        legacy_total_width: legacy_total.unwrap_or(0.0),
        left_width: left_widths.and_then(|v| v.get(b_idx)).copied(),
        right_width: right_widths.and_then(|v| v.get(b_idx)).copied(),
        left_station: left_stations.and_then(|v| v.get(b_idx)).copied(),
        right_station: right_stations.and_then(|v| v.get(b_idx)).copied(),
        left_top_elevation: left_top_elevs.and_then(|v| v.get(b_idx)).copied(),
        right_top_elevation: right_top_elevs.and_then(|v| v.get(b_idx)).copied(),
        left_top_profile_stations: left_top_profile_stations
            .and_then(|v| v.get(b_idx))
            .cloned(),
        left_top_profile_elevations: left_top_profile_elevations
            .and_then(|v| v.get(b_idx))
            .cloned(),
        right_top_profile_stations: right_top_profile_stations
            .and_then(|v| v.get(b_idx))
            .cloned(),
        right_top_profile_elevations: right_top_profile_elevations
            .and_then(|v| v.get(b_idx))
            .cloned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_total_splits_symmetrically() {
        let input = BridgeAbutmentUserInput {
            legacy_total_width: 4.0,
            ..Default::default()
        };
        let abut = resolve_abutments(&input, 0.0, 20.0, 1.0, UnitSystem::Metric);
        assert!((abut.left_width_m() - 2.0).abs() < 1e-9);
        assert!((abut.right_width_m() - 2.0).abs() < 1e-9);
        assert!((abut.total_block_width_m() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn asymmetric_widths_do_not_split_legacy() {
        let input = BridgeAbutmentUserInput {
            legacy_total_width: 4.0,
            left_width: Some(1.0),
            right_width: Some(3.0),
            ..Default::default()
        };
        let abut = resolve_abutments(&input, 0.0, 20.0, 1.0, UnitSystem::Metric);
        assert!((abut.left_width_m() - 1.0).abs() < 1e-9);
        assert!((abut.right_width_m() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn partial_height_abutment_blocks_less_area() {
        let side = BridgeAbutmentSide {
            outer_station_m: 0.0,
            width_m: 2.0,
            top_elevation_m: Some(2.0),
            ..Default::default()
        };
        let full = side.submerged_area_m2(3.0, 0.0, true);
        let partial = BridgeAbutmentSide {
            top_elevation_m: Some(2.5),
            ..side
        }
        .submerged_area_m2(3.0, 0.0, true);
        assert!((full - 2.0).abs() < 1e-9);
        assert!((partial - 1.0).abs() < 1e-9);
    }

    #[test]
    fn per_side_top_elevations_sum_independently() {
        let abut = BridgeAbutments {
            left: Some(BridgeAbutmentSide {
                outer_station_m: 0.0,
                width_m: 2.0,
                top_elevation_m: Some(2.0),
                ..Default::default()
            }),
            right: Some(BridgeAbutmentSide {
                outer_station_m: 10.0,
                width_m: 3.0,
                top_elevation_m: Some(2.8),
                ..Default::default()
            }),
        };
        // WSEL 2.5: left blocks 0.5 m deep (1.0 m²); right top 2.8 m is above water.
        assert!((abut.submerged_area_m2(2.5, 0.0) - 1.0).abs() < 1e-9);
        // WSEL 3.0: left 2.0 m² + right 0.6 m².
        assert!((abut.submerged_area_m2(3.0, 0.0) - 2.6).abs() < 1e-9);
        assert!((abut.submerged_width_at_wsel_m(2.5, 0.0) - 2.0).abs() < 1e-9);
        assert!((abut.submerged_width_at_wsel_m(3.0, 0.0) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn profile_top_integrates_along_abutment_width() {
        let side = BridgeAbutmentSide {
            outer_station_m: 0.0,
            width_m: 4.0,
            top_elevation_m: None,
            top_profile_stations_m: vec![0.0, 2.0, 4.0],
            top_profile_elevations_m: vec![2.0, 2.0, 3.0],
        };
        // wsel=2.5: trapezoidal integration along sloped top (not a single min-elevation depth).
        let area = side.submerged_area_m2(2.5, 0.0, true);
        assert!((area - 1.5).abs() < 1e-6, "expected 1.5 m², got {area}");
        let width = side.submerged_width_at_wsel_m(2.5, 0.0, true);
        assert!(width > 2.0, "sloped top should block more than the low half alone");
    }

    #[test]
    fn one_sided_left_abutment_only() {
        let left_only = resolve_abutments(
            &BridgeAbutmentUserInput {
                left_width: Some(3.0),
                left_top_elevation: Some(0.0),
                ..Default::default()
            },
            0.0,
            10.0,
            1.0,
            UnitSystem::Metric,
        );
        assert!(left_only.left.is_some());
        assert!(left_only.right.is_none());
        assert!((left_only.left_width_m() - 3.0).abs() < 1e-9);
        assert!(left_only.right_width_m().abs() < 1e-9);
        // Hand calc @ WSEL 3.0 m, bed 0: 3.0 m × 3.0 m depth = 9.0 m².
        assert!((left_only.submerged_area_m2(3.0, 0.0) - 9.0).abs() < 1e-9);
    }

    #[test]
    fn one_sided_right_abutment_only() {
        let right_only = resolve_abutments(
            &BridgeAbutmentUserInput {
                right_width: Some(3.0),
                right_top_elevation: Some(2.0),
                ..Default::default()
            },
            0.0,
            10.0,
            1.0,
            UnitSystem::Metric,
        );
        assert!(right_only.left.is_none());
        assert!(right_only.right.is_some());
        assert!((right_only.right_width_m() - 3.0).abs() < 1e-9);
        // Hand calc @ WSEL 3.0 m: right top 2.0 m → 3.0 m × 1.0 m depth = 3.0 m².
        assert!((right_only.submerged_area_m2(3.0, 0.0) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn asymmetric_opening_hand_calc_submerged_area_at_wsel_3() {
        let abut = resolve_abutments(
            &BridgeAbutmentUserInput {
                left_width: Some(1.0),
                right_width: Some(4.0),
                left_top_elevation: Some(0.0),
                right_top_elevation: Some(2.5),
                ..Default::default()
            },
            0.0,
            10.0,
            1.0,
            UnitSystem::Metric,
        );
        // Left: 1.0 m × 3.0 m = 3.0 m². Right: 4.0 m × 0.5 m = 2.0 m². Total = 5.0 m².
        assert!((abut.submerged_area_m2(3.0, 0.0) - 5.0).abs() < 1e-9);
        assert!((abut.submerged_width_at_wsel_m(3.0, 0.0) - 5.0).abs() < 1e-9);
        // Same total width (5 m) with uniform full-height tops → 15.0 m².
        let uniform = resolve_abutments(
            &BridgeAbutmentUserInput {
                left_width: Some(1.0),
                right_width: Some(4.0),
                left_top_elevation: Some(0.0),
                right_top_elevation: Some(0.0),
                ..Default::default()
            },
            0.0,
            10.0,
            1.0,
            UnitSystem::Metric,
        );
        assert!((uniform.submerged_area_m2(3.0, 0.0) - 15.0).abs() < 1e-9);
        assert!(
            abut.submerged_area_m2(3.0, 0.0) < uniform.submerged_area_m2(3.0, 0.0),
            "partial right abutment top should block less plan area"
        );
    }

    #[test]
    fn asymmetric_widths_differ_from_symmetric_split_at_surface() {
        let asymmetric = resolve_abutments(
            &BridgeAbutmentUserInput {
                left_width: Some(1.0),
                right_width: Some(4.0),
                left_top_elevation: Some(0.0),
                right_top_elevation: Some(2.5),
                ..Default::default()
            },
            0.0,
            10.0,
            1.0,
            UnitSystem::Metric,
        );
        let symmetric = BridgeAbutments::symmetric_total_width_m(5.0, 0.0, 10.0);
        assert!((asymmetric.submerged_area_m2(3.0, 0.0) - 5.0).abs() < 1e-6);
        // Symmetric 2.5 m per side, full height: 5.0 m × 3.0 m depth.
        assert!((symmetric.submerged_area_m2(3.0, 0.0) - 15.0).abs() < 1e-6);
        assert!(
            (asymmetric.submerged_area_m2(3.0, 0.0) - symmetric.submerged_area_m2(3.0, 0.0)).abs()
                > 0.5,
            "per-side tops should not match equal split of total width"
        );
    }
}
